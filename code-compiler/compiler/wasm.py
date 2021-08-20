import urllib.parse
import os
import subprocess
import threading
import ipfshttpclient
import requests
from helper.helper import write_to_disk


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

    # Uploading files to IPFS
    if os.environ.get('IPFS_URL'):
        client = ipfshttpclient.connect(os.environ.get('IPFS_URL'))  # Connect with IPFS container name
    else:
        client = ipfshttpclient.connect()

    print("Uploading files to IPFS...")
    config_res = client.add(project)
    mapping_res = client.add(mapping)
    schema_res = client.add(schema)

    # Uploading to IPFS result
    print("project.yaml: " + config_res['Hash'])
    print(model + ".wasm: " + mapping_res['Hash'])
    print("schema.graphql: " + schema_res['Hash'])

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
                                schema_res['Hash']
                            ],
                            'id': 1,
                        })
    print(res.json())
