import urllib.parse
import os


def read_so_example(path):
    # Read mapping.rs
    mapping_file = open(os.path.join(path, "src", "mapping.rs"))
    mapping = urllib.parse.quote(mapping_file.read())
    mapping_file.close()

    # Read project.yaml
    project_file = open(os.path.join(path, "subgraph.yaml"))
    project = urllib.parse.quote(project_file.read())
    project_file.close()

    # Read schema.graphql
    schema_file = open(os.path.join(path, "schema.graphql"))
    schema = urllib.parse.quote(schema_file.read())
    schema_file.close()

    payload = {
        "mappings": {
            "mapping.rs": mapping,
        },
        "configs": {
            "project.yaml": project,
            "schema.graphql": schema
        }
    }
    print(payload)
    return payload


def read_wasm_example(path, custom_mapping_path):
    """
    Read wasm example from user-examples

    :param path: (String) path to the example folder

    :param custom_mapping_path: (String) path to the mapping folder in side of the example folder
    :return: (Dict) Payload for calling to /compile/wasm endpoint
    """
    # Read abis
    abis_files = os.listdir(os.path.join(path, "abis"))
    abis_dict = {}
    for name in abis_files:
        f = open(os.path.join(path, "abis", name))
        content = urllib.parse.quote(f.read())
        abis_dict[name] = content
        f.close()

    # Read mapping
    mapping_dict = {}

    if custom_mapping_path == 'default':  # Read in /src if custom_mapping_path is none
        mapping_files = os.listdir(os.path.join(path, "src"))
        for name in mapping_files:
            f = open(os.path.join(path, "src", name))
            content = urllib.parse.quote(f.read())
            mapping_dict[name] = content
            f.close()
    else:  # Read in /src/[custom_mapping_path]
        mapping_files = os.listdir(os.path.join(path, "src", custom_mapping_path))
        for name in mapping_files:
            f = open(os.path.join(path, "src", custom_mapping_path, name))
            content = urllib.parse.quote(f.read())
            mapping_dict[custom_mapping_path + '/' + name] = content
            f.close()

    # Read subgraph.yaml
    subgraph_file = open(os.path.join(path, "subgraph.yaml"))
    subgraph = urllib.parse.quote(subgraph_file.read())
    subgraph_file.close()

    # Read schema.graphql
    schema_file = open(os.path.join(path, "schema.graphql"))
    schema = urllib.parse.quote(schema_file.read())
    schema_file.close()

    # Read package.json
    package_file = open(os.path.join(path, "package.json"))
    package = urllib.parse.quote(package_file.read())
    package_file.close()

    payload = {
        "abis": abis_dict,
        "mappings": mapping_dict,
        "configs": {
            "subgraph.yaml": subgraph,
            "schema.graphql": schema,
            "package.json": package
        }
    }
    print(payload)
    return payload
