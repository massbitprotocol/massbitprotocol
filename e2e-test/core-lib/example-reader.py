import urllib.parse
import os
import yaml

def read_so_example(path):
    # Read Content
    mapping = read_file_content(os.path.join(path, "src", "mapping.rs"))
    subgraph = read_file_content(os.path.join(path, "subgraph.yaml"))
    schema = read_file_content(os.path.join(path, "schema.graphql"))
    abi = read_abi_content(os.path.join(path, "abis"))

    payload = {
        "mappings": {
            "mapping.rs": mapping,
        },
        "configs": {
            "subgraph.yaml": subgraph,
            "schema.graphql": schema
        },
        "abis": abi,
    }
    print(payload)
    return payload


def read_wasm_example(path):
    """
    Read wasm example from user-examples

    :param path: (String) path to the example folder

    :return: (Dict) Payload for calling to /compile/wasm endpoint
    """
    # Read Content
    subgraph = read_file_content(os.path.join(path, "subgraph.yaml"))
    schema = read_file_content(os.path.join(path, "schema.graphql"))
    package = read_file_content(os.path.join(path, "package.json"))
    abi = read_abi_content(os.path.join(path, "abis"))
    mapping = read_mapping_content(path, "subgraph.yaml")

    payload = {
        "abis": abi,
        "mappings": mapping,
        "configs": {
            "subgraph.yaml": subgraph,
            "schema.graphql": schema,
            "package.json": package
        }
    }
    print(payload)
    return payload


def get_dirs(path):
    dirs = next(os.walk(path))[1]
    # Remove hidden dirs
    dirs = [d for d in dirs if d[0] != "."]
    return dirs


def get_file(path):
    files = [f for f in os.listdir(path) if os.path.isfile(os.path.join(path, f))]
    # Remove hidden files
    files = [f for f in files if f[0] != "."]
    return files


def read_file_content(path):
    file = open(path)
    content = urllib.parse.quote(file.read())
    file.close()
    return content


def update_payload(payload, key_path, value_path):
    payload[key_path] = read_file_content(value_path)
    return payload


def read_abi_content(path):
    """
     Read abis content by traversing files
    """
    files = os.listdir(path)
    abi = {}
    for name in files:
        abi[name] = read_file_content(os.path.join(path, name))
    return abi


def read_yaml_content(path):
    stream = open(path, 'r')
    yaml_content = yaml.safe_load(stream)
    stream.close()
    return yaml_content


def path_to_array(p):
    """
    Break path into an array

    Example Input: ./mappings/file.txt

    Example Output: ['.','mappings','file.txt']
    """
    head,tail = os.path.split(p)
    components = []
    while len(tail)>0:
        components.insert(0,tail)
        head,tail = os.path.split(head)
    return components


def get_mapping_root_name(path):
    subgraph_yaml = read_yaml_content(path)
    file_path = subgraph_yaml["dataSources"][0]["mapping"]["file"]
    return path_to_array(file_path)[1]


def read_mapping_content(path, subgraph_name):
    """
    Read the mapping content by traversing files.
    First we need to identify the root mapping folder name that was defined the the subgraph file. It could be "src" or "mappings" or "src/mappings"
    """
    mapping = {}
    root_name = get_mapping_root_name(os.path.join(path, subgraph_name))
    mapping_path = os.path.join(path, root_name)
    for folder in get_dirs(mapping_path):
        # Go 1 level deeper
        mapping_path_lvl_1 = os.path.join(mapping_path, folder)
        for file_lvl_1 in get_file(mapping_path_lvl_1):
            mapping = update_payload(mapping,
                                     os.path.join(root_name, folder, file_lvl_1),
                                     os.path.join(mapping_path_lvl_1, file_lvl_1))

    for file in get_file(mapping_path):
        mapping = update_payload(mapping,
                                 os.path.join(root_name, file),
                                 os.path.join(mapping_path, file))
    return mapping
