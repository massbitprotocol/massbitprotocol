import urllib.parse
import os
import subprocess
import threading
import ipfshttpclient
import requests
import yaml
from helper.helper import write_to_disk, get_abi_files, upload_abi_to_ipfs


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
    subgraph = urllib.parse.unquote_plus(data["configs"]["subgraph.yaml"])
    schema = urllib.parse.unquote_plus(data["configs"]["schema.graphql"])
    package = urllib.parse.unquote_plus(data["configs"]["package.json"])

    # Save the formatted data from request to disk, ready for compiling
    for file_name in mappings:
        write_to_disk(os.path.join(generated_folder, "src", file_name), urllib.parse.unquote_plus(mappings[file_name]))
    for file_name in abis:
        write_to_disk(os.path.join(generated_folder, "abis", file_name), urllib.parse.unquote_plus(abis[file_name]))
    write_to_disk(os.path.join(generated_folder, "subgraph.yaml"), subgraph)
    write_to_disk(os.path.join(generated_folder, "schema.graphql"), schema)
    write_to_disk(os.path.join(generated_folder, "package.json"), package)

    # Codegen & Build
    print("Generating code + compiling for: " + hash + ". This will take a while!")
    wasm_codegen_and_build = WasmCodegenAndBuild(generated_folder)
    wasm_codegen_and_build.start()
    return hash


def deploy_wasm(data):
    # Parse the data
    compilation_id = urllib.parse.unquote_plus(data["compilation_id"])
    model = urllib.parse.unquote_plus(data["configs"]["model"])

    # Get the files path from generated/hash folder
    project = os.path.join("./generated", compilation_id, "subgraph.yaml")
    mapping = os.path.join("./generated", compilation_id, "build", model, model + ".wasm")
    schema = os.path.join("./generated", compilation_id, "schema.graphql")
    config = os.path.join("./generated", compilation_id, "config.yaml")
    abi = get_abi_files(compilation_id)

    # Uploading files to IPFS
    if os.environ.get('IPFS_URL'):
        client = ipfshttpclient.connect(os.environ.get('IPFS_URL'))  # Connect with IPFS container name
    else:
        client = ipfshttpclient.connect()

    print("Uploading files to IPFS...")
    config_res = client.add(project)
    mapping_res = client.add(mapping)
    schema_res = client.add(schema)
    abi_res = upload_abi_to_ipfs(client, abi)

    # Uploading to IPFS result
    print("subgraph.yaml: {}".format(config_res['Hash']))
    print(model + ".wasm: {}".format(mapping_res['Hash']))
    print("schema.graphql: {}".format(schema_res['Hash']))
    for abi_object in abi_res:
        print('{}: {}'.format(abi_object["name"], abi_object["hash"]))

    # Generate the new config file based on the subgraph.yaml content and other file IPFS
    generate_new_config(project, schema_res, abi_res, config)

    # Upload the config to IPFS
    config_res = client.add(config)

    # Uploading IPFS files to Index Manager
    if os.environ.get('INDEX_MANAGER_URL'):
        index_manager_url = os.environ.get('INDEX_MANAGER_URL')  # Connection to indexer
    else:
        index_manager_url = 'http://0.0.0.0:3030'

    res = requests.post(index_manager_url,
                        json={
                            'jsonrpc': '2.0',
                            'method': 'index_deploy',
                            'params': [
                                config_res['Hash'],
                                mapping_res['Hash'],
                                schema_res['Hash'],
                                abi_res,
                                config_res['Hash']
                            ],
                            'id': '1',
                        })
    print(res.json())

def generate_new_config(project, schema_res, abi_res, config):
    # Generate new yaml config file
    stream = open(project, 'r')
    subgraph = yaml.safe_load(stream)

    # Parse subgraph.yaml and create a new config.yaml with IPFS hash populated
    subgraph['schema']['file'] = {'/': '/ipfs/' + schema_res['Hash']}
    subgraph = replace_ipfs_hash('dataSources', subgraph, abi_res)
    subgraph = replace_ipfs_hash('templates', subgraph, abi_res)

    # This is a small hack, should be fix in the next MR
    # Providing the file here with any hash so it doesn't return error when parsing the datasource in index-manager
    subgraph['dataSources'][0]['mapping']['file'] = {'/': '/ipfs/' + schema_res['Hash']}
    if 'templates' in subgraph:
        subgraph['templates'][0]['mapping']['file'] = {'/': '/ipfs/' + schema_res['Hash']}

    # Write the new config.yaml to local disk
    file = open(config, "w")
    yaml.safe_dump(subgraph, file)
    file.close()


def replace_ipfs_hash(subgraph_type, subgraph, abi_res):
    if subgraph_type in subgraph:
        for i in range(0, len(subgraph[subgraph_type][0]['mapping']['abis'])):
            file_name = os.path.basename(subgraph[subgraph_type][0]['mapping']['abis'][i]['file'])
            name = subgraph[subgraph_type][0]['mapping']['abis'][i]['name']
            for abi in abi_res:
                if file_name.lower() == abi["name"].lower():
                    subgraph[subgraph_type][0]['mapping']['abis'][i] = {'name': name, 'file': {'/': '/ipfs/' + abi["hash"]}}
    return subgraph
