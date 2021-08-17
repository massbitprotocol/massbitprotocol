import os
import json

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
    return files


if __name__ == '__main__':
    dir_json = {}
    chains = get_dirs(os.path.join('.'))
    for chain in chains:
        dir_json[chain] = {}
        examples = get_dirs(os.path.join('.', chain))
        for example in examples:
            dir_json[chain][example] = {}
            files = get_file(os.path.join('.', chain, example, "src"))

            # If there are files in `src` folder
            if len(files) != 0:
                for file in files:
                    dir_json[chain][example][file] = git_path + os.path.join(user_example_folder, chain, example, "src",
                                                                             file)
            # If there are folders in `src` folder
            else:
                sub_src_dirs = get_dirs(os.path.join('.', chain, example, "src"))
                for sub_src_dir in sub_src_dirs:
                    dir_json[chain][example][sub_src_dir] = {}
                    files = get_file(os.path.join('.', chain, example, "src", sub_src_dir))
                    for file in files:
                        dir_json[chain][example][file] = git_path + os.path.join(user_example_folder, chain, example,
                                                                                 "src", sub_src_dir, file)

    print(json.dumps(dir_json, indent=4, sort_keys=True))
    with open(result_file, 'w') as fp:
        json.dump(dir_json, fp, indent=4, sort_keys=True)
