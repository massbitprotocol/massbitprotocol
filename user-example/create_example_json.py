import os
import json
import os.path
from os import path
import urllib.parse

git_path = "https://raw.githubusercontent.com/massbitprotocol/massbitprotocol/main/"
user_example_folder = "user-example"
result_file = "examples.json"


def get_dirs(path):
    dirs = next(os.walk(path))[1]
    # Remove hidden dirs
    dirs = [d for d in dirs if d[0] != "."]

    return dirs


def get_file(path):
    files = [f for f in os.listdir(path) if os.path.isfile(os.path.join(path, f))]
    # Remove hidden files
    files = [f for f in files if f[0] != "."]
    print(path)
    return files


def read_file_content(path):
    file = open(path)
    content = urllib.parse.quote_plus(file.read())
    file.close()
    return content


if __name__ == '__main__':
    dir_json = {}
    chains = get_dirs(os.path.join('.'))
    for chain in chains:  # substrate
        dir_json[chain] = {}
        types = get_dirs(os.path.join('.', chain))
        for type in types:  # substrate/so
            dir_json[chain][type] = {}
            chain_and_type = os.path.join('.', chain, type)
            for example in get_dirs(chain_and_type):  # substrate/so/test-block
                dir_json[chain][type][example] = {}
                chain_and_type_and_example = os.path.join(chain_and_type, example)

                # Find folders (src, abis,...) in the example
                for folder in get_dirs(chain_and_type_and_example):  # substrate/so/test-block/src or substrate/so/test-block/abis  ...
                    dir_json[chain][type][example][folder] = {}
                    dir_json[chain][type][example]["src"] = {}  # Set this up for quickswap
                    chain_and_type_and_example_and_folder = os.path.join(chain_and_type_and_example, folder)

                    # Go 1 deeper level to find the mappings of quickswap  # ethereum/wasm/quickswap/src/mappings/...
                    if "quickswap" in chain_and_type_and_example_and_folder:
                        quickswap_mappings_path = os.path.join(chain_and_type_and_example, "src", "mappings")
                        for quickswap_file in get_file(quickswap_mappings_path):
                            dir_json[chain][type][example]["src"]["mappings/" + quickswap_file] = read_file_content(os.path.join(quickswap_mappings_path, quickswap_file))

                    for file in get_file(chain_and_type_and_example_and_folder):  # substrate/so/test-block/src/mapping.rs or substrate/so/test-block/abis/abi.json ....
                        chain_and_type_and_example_and_folder_and_file = os.path.join(chain_and_type_and_example_and_folder, file)
                        dir_json[chain][type][example][folder][file] = read_file_content(chain_and_type_and_example_and_folder_and_file)

                # Find files (project.yaml, schema.graphql, package.json...) in the example
                for file in get_file(chain_and_type_and_example):
                    chain_and_type_and_example_and_file = os.path.join(chain_and_type_and_example, file)
                    dir_json[chain][type][example][file] = read_file_content(chain_and_type_and_example_and_file)

    print(json.dumps(dir_json, indent=4, sort_keys=True))
    with open(result_file, 'w') as fp:
        json.dump(dir_json, fp, indent=4, sort_keys=True)
