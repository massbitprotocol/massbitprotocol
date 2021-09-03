import urllib.parse
import os
import subprocess
import threading
import requests
import yaml
from helper.helper import write_to_disk, get_abi_files, upload_abi_to_ipfs, ipfs_client_init, get_index_manager_url


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
            write_to_disk(self.generated_folder + "/error.txt", exc.output)
        else:
            print("Compilation was success. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/success.txt", output)


def compile_wasm(data, hash):
    # Create new folder
    generated_folder = "generated/" + hash

    # URL-decode the data
    mappings = data["mappings"]
    abis = data["abis"]
    subgraph = urllib.parse.unquote(data["configs"]["subgraph.yaml"])
    schema = urllib.parse.unquote(data["configs"]["schema.graphql"])
    package = urllib.parse.unquote(data["configs"]["package.json"])

    # Save the formatted data from request to disk, ready for compiling
    for file_name in mappings:
        write_to_disk(os.path.join(generated_folder, "src", file_name), urllib.parse.unquote(mappings[file_name]))
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
    subgraph_path = os.path.join("./generated", compilation_id, "build", "subgraph.yaml")
    schema_path = os.path.join("./generated", compilation_id, "schema.graphql")
    parsed_subgraph_path = os.path.join("./generated", compilation_id, "parsed_subgraph.yaml")
    abi = get_abi_files(compilation_id)
    ds_mapping_path = get_ds_mapping_path(subgraph_path, compilation_id)
    if is_template_exist(subgraph_path):
        tp_mapping_path = get_tp_mapping_path(subgraph_path, compilation_id)

    # Upload files to IPFS
    print("Uploading files to IPFS...")
    client = ipfs_client_init()
    subgraph_res = client.add(subgraph_path)
    schema_res = client.add(schema_path)
    abi_res = upload_abi_to_ipfs(client, abi)
    ds_mapping_res = client.add(ds_mapping_path)
    if is_template_exist(subgraph_path):
        tp_mapping_res = client.add(tp_mapping_path)

    # IPFS upload result
    print(f"{subgraph_path}: {subgraph_res['Hash']}")
    print(f"{schema_path}: {schema_res['Hash']}")
    print(f"{ds_mapping_path}: {ds_mapping_res['Hash']}")
    for abi_object in abi_res:
        print(f"{os.path.join('./generated', compilation_id, abi_object['name'])} : {abi_object['hash']}")
    if is_template_exist(subgraph_path):
        print(f"{tp_mapping_path}: {tp_mapping_res['Hash']}")

    # Parse subgraph file and upload to IPFS
    if is_template_exist(subgraph_path):
        parse_subgraph(subgraph_path, parsed_subgraph_path, schema_res, abi_res, ds_mapping_res, tp_mapping_res)
    else:
        parse_subgraph(subgraph_path, parsed_subgraph_path, schema_res, abi_res, ds_mapping_res)
    parsed_subgraph_res = client.add(parsed_subgraph_path)

    # Deploy a new index to Index Manager
    deploy_to_index_manager(parsed_subgraph_res, ds_mapping_res, schema_res, abi_res)


def parse_subgraph(subgraph_path, parsed_subgraph_path, schema_res, abi_res, ds_mapping_res, tp_mapping_res = None):
    """
    Parse subgraph.yaml and create a new parsed_subgraph.yaml with IPFS hash populated
    """
    # Create new file
    stream = open(subgraph_path, 'r')
    # Load subgraph content
    subgraph = yaml.safe_load(stream)

    # Parsing subgraph content
    subgraph['schema']['file'] = {'/': '/ipfs/' + schema_res['Hash']}
    subgraph = replace_ipfs_hash('dataSources', subgraph, abi_res)
    subgraph = replace_ipfs_hash('templates', subgraph, abi_res)
    subgraph['dataSources'][0]['mapping']['file'] = {'/': '/ipfs/' + ds_mapping_res['Hash']}
    if is_template_exist(subgraph_path):
        subgraph['templates'][0]['mapping']['file'] = {'/': '/ipfs/' + tp_mapping_res['Hash']}

    # Write the new file to local disk
    file = open(parsed_subgraph_path, "w")
    yaml.safe_dump(subgraph, file)
    file.close()


def replace_ipfs_hash(subgraph_type, subgraph, abi_res):
    if subgraph_type in subgraph:
        for i in range(0, len(subgraph[subgraph_type][0]['mapping']['abis'])):
            file_name = os.path.basename(subgraph[subgraph_type][0]['mapping']['abis'][i]['file'])
            name = subgraph[subgraph_type][0]['mapping']['abis'][i]['name']
            for abi_object in abi_res:
                if file_name.lower() == abi_object["name"].lower():
                    subgraph[subgraph_type][0]['mapping']['abis'][i] = {'name': name,
                                                                        'file': {'/': '/ipfs/' + abi_object["hash"]}}
    return subgraph


def deploy_to_index_manager(parsed_subgraph_res, ds_mapping_res, schema_res, abi_res):
    # TODO: The config and subgraph is the same for WASM, so we only need to send one.
    # TODO: ds_mapping_res should be removed because when we use the graph's logic,
    #       we only need the parsed_subgraph.yaml Note:
    # Note: we don't support  uploading template mapping yet because this flow will
    #       soon be replaced by uploading parsed_subgraph.yaml only
    res = requests.post(get_index_manager_url(),
                        json={
                            'jsonrpc': '2.0',
                            'method': 'index_deploy',
                            'params': [
                                parsed_subgraph_res['Hash'],
                                ds_mapping_res['Hash'],
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


def is_template_exist(subgraph_path):
    stream = open(subgraph_path, 'r')
    subgraph = yaml.safe_load(stream)
    if 'templates' in subgraph:
        stream.close()
        return True
    return False
