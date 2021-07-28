import os
import json

if __name__ == '__main__':
    git_path = "https://raw.githubusercontent.com/massbitprotocol/massbitprotocol/main/"
    user_example_folder = "user-example"
    result_file = "examples.json"

    dir_json = {}
    chains = next(os.walk('.'))[1]
    for chain in chains:
        dir_json[chain] = {}
        examples = next(os.walk(os.path.join('.', chain)))[1]
        for example in examples:
            dir_json[chain][example]={}
            files = [f for f in os.listdir(os.path.join('.', chain, example, "src")) if os.path.isfile(os.path.join(os.path.join('.', chain, example, "src"), f))]
            for file in files:
                dir_json[chain][example][file] = git_path + os.path.join(user_example_folder, chain, example, "src", file)

    print(dir_json)
    with open(result_file, 'w') as fp:
        json.dump(dir_json, fp, indent=4)

