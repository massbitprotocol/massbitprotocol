import hashlib
import json
import shutil
import urllib.parse
import os
import subprocess
import threading
import requests
import yaml
from helper.helper import write_to_disk, get_abi_files, upload_abi_to_ipfs, ipfs_client_init, get_index_manager_url, \
    is_template_exist, upload_mapping_to_ipfs, replace_mapping_v1, replace_abi_v2

success_file = "success.txt"
error_file = "error.txt"

success_codegen_file = "success-codegen.txt"
error_codegen_file = "error-codegen.txt"


class WasmCodegenAndBuild(threading.Thread):
    """
    NpmBuild is class that will run `npm run codegen && npm run build` in a new thread, not blocking the main thread

    """

    def __init__(self, generated_folder):
        self.stdout = None
        self.stderr = None
        self.generated_folder = generated_folder
        threading.Thread.__init__(self)

    def run(self):
        try:
            output = subprocess.check_output(["npm install && npm run codegen && npm run build"],
                                             stderr=subprocess.STDOUT,
                                             shell=True, universal_newlines=True, cwd=self.generated_folder)

        except subprocess.CalledProcessError as exc:
            print("Compilation has failed. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/" + error_file, exc.output)
        else:
            print("Compilation was success. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/" + success_file, output)


def compile_wasm(data, use_precompile=True):
    # Create hash for generated folder name
    dump_data = json.dumps(data).encode('utf-8')
    print(dump_data)
    hash = hashlib.md5(dump_data).hexdigest()

    generated_folder = os.path.join("generated", hash)
    success_file_full = os.path.join(generated_folder, success_file)
    success_codegen_file_full = os.path.join(generated_folder, success_codegen_file)

    # Check if we could reuse the precompile
    if use_precompile and os.path.isfile(success_file_full):
        return hash

    # Remove the exist folder
    if os.path.isdir(generated_folder):
        shutil.rmtree(generated_folder)

    # URL-decode the data
    mappings = data["mappings"]
    abis = data["abis"]
    subgraph = urllib.parse.unquote(data["configs"]["subgraph.yaml"])
    schema = urllib.parse.unquote(data["configs"]["schema.graphql"])
    package = urllib.parse.unquote(data["configs"]["package.json"])

    # Save the formatted data from request to disk, ready for compiling
    for file_name in mappings:
        write_to_disk(os.path.join(generated_folder, file_name), urllib.parse.unquote(mappings[file_name]))
    for file_name in abis:
        write_to_disk(os.path.join(generated_folder, "abis", file_name), urllib.parse.unquote(abis[file_name]))
    write_to_disk(os.path.join(generated_folder, "subgraph.yaml"), subgraph)
    write_to_disk(os.path.join(generated_folder, "schema.graphql"), schema)
    write_to_disk(os.path.join(generated_folder, "package.json"), package)

    # Codegen & Build
    print("Generating code + compiling for: " + hash + ". This will take a while!")
    wasm_codegen_and_build = WasmCodegenAndBuild(generated_folder)
    wasm_codegen_and_build.start()
    return hash


def deploy_wasm(data):
    # Parse the request data
    compilation_id = urllib.parse.unquote(data["compilation_id"])

    # Get the files path from generated/hash folder
    root_path = os.path.join("./generated", compilation_id)
    subgraph_path = os.path.join(root_path, "build", "subgraph.yaml")
    schema_path = os.path.join(root_path, "schema.graphql")
    parsed_subgraph_path = os.path.join(root_path, "parsed_subgraph.yaml")
    abi = get_abi_files(compilation_id)

    # Upload files to IPFS
    print("Uploading files to IPFS for {}...", root_path)
    client = ipfs_client_init()
    schema_res = client.add(schema_path)
    abi_res = upload_abi_to_ipfs(client, abi)
    ds_mapping_res = upload_mapping_to_ipfs(client, 'dataSources', root_path, subgraph_path)
    if is_template_exist(subgraph_path):
        tp_mapping_res = upload_mapping_to_ipfs(client, 'templates', root_path, subgraph_path)

    # Load subgraph content
    subgraph_content = get_subgraph_content(subgraph_path)

    # Replace mapping file and schema
    if is_template_exist(subgraph_path):
        subgraph_content = replace_mapping_and_schema(subgraph_path, subgraph_content, schema_res,
                                                      ds_mapping_res, tp_mapping_res)
    else:
        subgraph_content = replace_mapping_and_schema(subgraph_path, subgraph_content, schema_res,
                                                      ds_mapping_res)

    # Replace abi
    subgraph_content = replace_abi_v2(client, root_path, 'dataSources', subgraph_content)
    if is_template_exist(subgraph_path):
        subgraph_content = replace_abi_v2(client, root_path, 'templates', subgraph_content)

    # Write new parsed subgraph to local and upload to IPFS
    create_new_parsed_subgraph(parsed_subgraph_path, subgraph_content)
    parsed_subgraph_res = client.add(parsed_subgraph_path)

    # Deploy a new index to Index Manager
    deploy_to_index_manager(parsed_subgraph_res, ds_mapping_res, schema_res, abi_res)


def replace_mapping_and_schema(subgraph_path, subgraph, schema_res, ds_mapping_res, tp_mapping_res=None):
    """
    Parse subgraph.yaml and create a new parsed_subgraph.yaml with IPFS hash populated
    """
    # After files are deployed to IFPS, we replace the files IPFS hash in the build/subgraph.yaml file
    subgraph['schema']['file'] = {'/': '/ipfs/' + schema_res['Hash']}
    subgraph = replace_mapping_v1('dataSources', subgraph, ds_mapping_res)
    if is_template_exist(subgraph_path):
        subgraph = replace_mapping_v1('templates', subgraph, tp_mapping_res)
    return subgraph


def deploy_to_index_manager(parsed_subgraph_res, ds_mapping_res, schema_res, abi_res):
    # Todo: remove abi_res, ds_mapping_res
    res = requests.post(get_index_manager_url(),
                        json={
                            'jsonrpc': '2.0',
                            'method': 'index_deploy',
                            'params': [
                                parsed_subgraph_res['Hash'],
                                ds_mapping_res[0]['file_hash'],
                                schema_res['Hash'],
                                abi_res,
                                parsed_subgraph_res['Hash']
                            ],
                            'id': '1',
                        })
    print(res.json())


def get_ds_mapping_path(subgraph_path, compilation_id):
    stream = open(subgraph_path, 'r')
    subgraph = yaml.safe_load(stream)
    stream.close()
    return os.path.join("./generated", compilation_id, "build", subgraph['dataSources'][0]['mapping']['file'])


def get_tp_mapping_path(subgraph_path, compilation_id):
    stream = open(subgraph_path, 'r')
    subgraph = yaml.safe_load(stream)
    stream.close()
    return os.path.join("./generated", compilation_id, "build", subgraph['templates'][0]['mapping']['file'])


def get_subgraph_content(subgraph_path):
    stream = open(subgraph_path, 'r')
    subgraph_content = yaml.safe_load(stream)
    stream.close()
    return subgraph_content


def create_new_parsed_subgraph(parsed_subgraph_path, subgraph_content):
    file = open(parsed_subgraph_path, "w")
    yaml.safe_dump(subgraph_content, file)
    file.close()
