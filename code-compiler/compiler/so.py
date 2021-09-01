import urllib.parse
import os
import subprocess
import threading
import ipfshttpclient
import requests
import yaml
from distutils.dir_util import copy_tree
from helper.helper import write_to_disk, populate_stub


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
            project = os.path.join("src/project.yaml")
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
    project = urllib.parse.unquote(data["configs"]["project.yaml"])
    schema = urllib.parse.unquote(data["configs"]["schema.graphql"])

    # Populating stub data
    populate_stub(generated_folder, "Cargo.lock")
    populate_stub(generated_folder, "Cargo.toml")
    copy_tree("stub/target", generated_folder + "/target")

    # Save the formatted data from request to disk, ready for compiling
    write_to_disk(generated_folder + "/src/mapping.rs", mapping)
    write_to_disk(generated_folder + "/src/project.yaml", project)
    write_to_disk(generated_folder + "/src/schema.graphql", schema)

    # Codegen + Build
    print("Generating code + compiling for: " + hash + ". This will take a while!")
    cargo_gen_and_build = CargoGenAndBuild(generated_folder)
    cargo_gen_and_build.start()


def deploy_so(data):
    # Parse the data
    compilation_id = urllib.parse.unquote(data["compilation_id"])

    # Get the files path from generated/hash folder
    subgraph_path = os.path.join("./generated", compilation_id, "src", "project.yaml") # TODO replace to have the same name project.yaml or subgraph.yaml
    parsed_subgraph_path = os.path.join("./generated", compilation_id, "parsed_subgraph.yaml")
    project = os.path.join("./generated", compilation_id, "src/project.yaml")
    so = os.path.join("./generated", compilation_id, "target/release/libblock.so")
    schema = os.path.join("./generated", compilation_id, "src/schema.graphql")
    # ds_mapping_path = get_ds_mapping_path(subgraph_path, compilation_id)
    # if is_template_exist(subgraph_path):
    #     tp_mapping_path = get_tp_mapping_path(subgraph_path, compilation_id)

    # Uploading files to IPFS
    if os.environ.get('IPFS_URL'):
        client = ipfshttpclient.connect(os.environ.get('IPFS_URL'))  # Connect with IPFS container name
    else:
        client = ipfshttpclient.connect()

    print("Uploading files to IPFS...")
    config_res = client.add(project)
    so_res = client.add(so)
    schema_res = client.add(schema)

    # Uploading to IPFS result
    print("project.yaml: " + config_res['Hash'])
    print("libblock.so: " + so_res['Hash'])
    print("schema.graphql: " + schema_res['Hash'])

    # Uploading IPFS files to Index Manager
    if os.environ.get('INDEX_MANAGER_URL'):
        index_manager_url = os.environ.get('INDEX_MANAGER_URL')  # Connection to indexer
    else:
        index_manager_url = 'http://0.0.0.0:3030'

    parse_subgraph(subgraph_path, parsed_subgraph_path, schema_res)
    parsed_subgraph_res = client.add(parsed_subgraph_path)

    null = None
    res = requests.post(index_manager_url,
                        json={
                            'jsonrpc': '2.0',
                            'method': 'index_deploy',
                            'params': [
                                config_res['Hash'],
                                so_res['Hash'],
                                schema_res['Hash'],
                                null,
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


def parse_subgraph(subgraph_path, parsed_subgraph_path, schema_res):
    """
    Parse subgraph.yaml and create a new parsed_subgraph.yaml with IPFS hash populated
    """
    # Create new file
    stream = open(subgraph_path, 'r')
    # Load subgraph content
    subgraph = yaml.safe_load(stream)

    # Parsing subgraph content
    subgraph['schema']['file'] = {'/': '/ipfs/' + schema_res['Hash']}

    # Write the new file to local disk
    file = open(parsed_subgraph_path, "w")
    yaml.safe_dump(subgraph, file)
    file.close()