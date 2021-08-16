import urllib.parse
import os

def read_so_example(path):
    # Read mapping.rs
    mapping_file = open(os.path.join(path, "mapping.rs"))
    mapping = urllib.parse.quote_plus(mapping_file.read())
    mapping_file.close()

    # Read project.yaml
    project_file = open(os.path.join(path, "project.yaml"))
    project = urllib.parse.quote_plus(project_file.read())
    project_file.close()

    # Read schema.graphql
    schema_file = open(os.path.join(path, "schema.graphql"))
    schema = urllib.parse.quote_plus(schema_file.read())
    schema_file.close()

    payload = {
        "mapping.rs": mapping,
        "project.yaml": project,
        "schema.graphql": schema
    }
    print(payload)
    return payload


def read_wasm_example(path, mapping_path):
    """
    Read wasm example from user-examples

    :param path: (String) path to the example folder

    :param mapping_path: (String) path to the mapping folder in side of the example folder
    :return: (Dict) Payload for calling to /compile/wasm endpoint
    """
    # Read abis
    abis_files = os.listdir(os.path.join(path, "abis"))
    abis_dict = {}
    for name in abis_files:
        f = open(os.path.join(path, "abis", name))
        content = urllib.parse.quote_plus(f.read())
        abis_dict[name] = content
        f.close()

    # Read mapping
    mapping_files = os.listdir(os.path.join(path, mapping_path))
    mapping_dict = {}
    for name in mapping_files:
        f = open(os.path.join(path, mapping_path, name))
        content = urllib.parse.quote_plus(f.read())
        mapping_dict[name] = content
        f.close()

    # Read subgraph.yaml
    subgraph_file = open(os.path.join(path, "subgraph.yaml"))
    subgraph = urllib.parse.quote_plus(subgraph_file.read())
    subgraph_file.close()

    # Read schema.graphql
    schema_file = open(os.path.join(path, "schema.graphql"))
    schema = urllib.parse.quote_plus(schema_file.read())
    schema_file.close()

    # Read package.json
    package_file = open(os.path.join(path, "package.json"))
    package = urllib.parse.quote_plus(package_file.read())
    package_file.close()

    payload = {
        "abis": abis_dict,
        "mapping": mapping_dict,
        "subgraph.yaml": subgraph,
        "schema.graphql": schema,
        "package.json": package,
    }
    print(payload)
    return payload