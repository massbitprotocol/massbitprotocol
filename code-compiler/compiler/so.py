import urllib.parse
import os
import subprocess
import threading
import ipfshttpclient
import requests
import yaml
from distutils.dir_util import copy_tree
from helper.helper import write_to_disk, populate_stub, get_abi_files, upload_abi_to_ipfs, replace_abi_with_hash, \
    get_index_manager_url


class CargoCodegen(threading.Thread):
    """
    CargoCodgen is to create a new thread to build new code from schema.graphql & project.yml

    """

    def __init__(self, generated_folder):
        self.stdout = None
        self.stderr = None
        self.generated_folder = generated_folder
        threading.Thread.__init__(self)

    def run(self):
        try:
            # Config
            schema = os.path.join("src/schema.graphql")
            project = os.path.join("src/subgraph.yaml")
            folder = os.path.join("src/")
            command = "$HOME/.cargo/bin/cargo run --manifest-path=../../../Cargo.toml --bin cli -- codegen -s {schema} -c {project} -o {folder} " \
                .format(schema=schema, project=project, folder=folder)
            print("Running: " + command)

            # Start
            output = subprocess.check_output([command], stderr=subprocess.STDOUT,
                                             shell=True, universal_newlines=True, cwd=self.generated_folder)
        except subprocess.CalledProcessError as exc:
            print("Codegen has failed. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/error-codegen.txt", exc.output)
        else:
            print("Codegen was success. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/success-codegen.txt", output)


class CargoGenAndBuild(threading.Thread):
    """
    CargoBuild is class that will run `cargo build --release` in a new thread, not blocking the main thread

    """

    def __init__(self, generated_folder):
        self.stdout = None
        self.stderr = None
        self.generated_folder = generated_folder
        threading.Thread.__init__(self)

    def run(self):
        cargo_codegen = CargoCodegen(self.generated_folder)
        cargo_codegen.run()  # TODO: This still block the request

        print("Compiling...")
        try:
            # Docker container doesn't know about cargo path so we need to use $HOME
            output = subprocess.check_output(["$HOME/.cargo/bin/cargo build --release"], stderr=subprocess.STDOUT,
                                             shell=True, universal_newlines=True, cwd=self.generated_folder)
        except subprocess.CalledProcessError as exc:
            print("Compilation has failed. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/error.txt", exc.output)
        else:
            print("Compilation was success. The result can be found in: " + self.generated_folder)
            write_to_disk(self.generated_folder + "/success.txt", output)


def compile_so(data, hash):
    generated_folder = "generated/" + hash

    # Create new folder
    os.mkdir(generated_folder)
    os.mkdir(generated_folder + "/src")

    # URL-decode the data
    mapping = urllib.parse.unquote(data["mappings"]["mapping.rs"])
    project = urllib.parse.unquote(data["configs"]["subgraph.yaml"])
    schema = urllib.parse.unquote(data["configs"]["schema.graphql"])
    abis = data["abis"]

    # Populating stub data
    populate_stub(generated_folder, "Cargo.lock")
    populate_stub(generated_folder, "Cargo.toml")
    copy_tree("stub/target", generated_folder + "/target")

    # Save the formatted data from request to disk, ready for compiling
    write_to_disk(generated_folder + "/src/mapping.rs", mapping)
    write_to_disk(generated_folder + "/src/subgraph.yaml", project)
    write_to_disk(generated_folder + "/src/schema.graphql", schema)
    for file_name in abis:
        write_to_disk(os.path.join(generated_folder, "abis", file_name), urllib.parse.unquote(abis[file_name]))

    # Codegen + Build
    print("Generating code + compiling for: " + hash + ". This will take a while!")
    cargo_gen_and_build = CargoGenAndBuild(generated_folder)
    cargo_gen_and_build.start()


def deploy_so(data):
    # Parse the data
    compilation_id = urllib.parse.unquote(data["compilation_id"])

    # Get the files path from generated/hash folder
    subgraph_path = os.path.join("./generated", compilation_id, "src", "subgraph.yaml")
    parsed_subgraph_path = os.path.join("./generated", compilation_id, "parsed_subgraph.yaml")
    mapping_path = os.path.join("./generated", compilation_id, "target/release/libblock.so")
    schema_path = os.path.join("./generated", compilation_id, "src/schema.graphql")
    abi = get_abi_files(compilation_id)

    # Uploading files to IPFS
    if os.environ.get('IPFS_URL'):
        client = ipfshttpclient.connect(os.environ.get('IPFS_URL'))  # Connect with IPFS container name
    else:
        client = ipfshttpclient.connect()

    print("Uploading files to IPFS...")
    subgraph_res = client.add(subgraph_path)
    mapping_res = client.add(mapping_path)
    schema_res = client.add(schema_path)
    abi_res = upload_abi_to_ipfs(client, abi)

    # Uploading to IPFS result
    print(f"{subgraph_path}: {subgraph_res['Hash']}")
    print(f"libblock.so: : {mapping_res['Hash']}")
    print(f"{schema_path}: {schema_res['Hash']}")
    for abi_object in abi_res:
        print(f"{os.path.join('./generated', compilation_id, abi_object['name'])} : {abi_object['hash']}")

    # Parse subgraph file and upload to IPFS
    parse_subgraph(subgraph_path, parsed_subgraph_path, mapping_res, schema_res, abi_res)
    parsed_subgraph_res = client.add(parsed_subgraph_path)

    # Deploy a new index to Index Manager
    deploy_to_index_manager(subgraph_res, parsed_subgraph_res, mapping_res, schema_res)


def deploy_to_index_manager(subgraph_res, parsed_subgraph_res, mapping_res, schema_res):
    null = None
    res = requests.post(get_index_manager_url(),
                        json={
                            'jsonrpc': '2.0',
                            'method': 'index_deploy',
                            'params': [
                                subgraph_res['Hash'],
                                mapping_res['Hash'],
                                schema_res['Hash'],
                                null,
                                parsed_subgraph_res['Hash']
                            ],
                            'id': '1',
                        })
    print(res.json())


def parse_subgraph(subgraph_path, parsed_subgraph_path, mapping_res, schema_res, abi_res):
    """
    Parse subgraph.yaml and create a new parsed_subgraph.yaml with IPFS hash populated
    """
    # Create new file
    stream = open(subgraph_path, 'r')
    # Load subgraph content
    subgraph = yaml.safe_load(stream)

    # Parsing subgraph content
    subgraph['schema']['file'] = {'/': '/ipfs/' + schema_res['Hash']}

    # Quick hack so we have file with ipfs link
    subgraph['dataSources'][0]['mapping']['file'] = {'/': '/ipfs/' + mapping_res['Hash']}
    subgraph = replace_abi_with_hash('dataSources', subgraph, abi_res)

    # Write the new file to local disk
    file = open(parsed_subgraph_path, "w")
    yaml.safe_dump(subgraph, file)
    file.close()
