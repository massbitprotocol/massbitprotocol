import errno
import random
import os
import ipfshttpclient
from shutil import copyfile


def write_to_disk(file, data):
    """
    write_to_disk create and save the file to disk

    :param file: (String) path to the file + the file's name

    :param data: (String) raw data. Any data with "\ n" will be created as newline
    :return: (String) ok
    """
    # Auto create folder parent folder if not exists
    if not os.path.exists(os.path.dirname(file)):
        try:
            os.makedirs(os.path.dirname(file))
        except OSError as exc:  # Guard against race condition
            if exc.errno != errno.EEXIST:
                raise

    f = open(file, "w+")
    f.write(data)
    return "ok"


def populate_stub(dst_dir, file_name):
    """
    populate_stub use the existing stub folder to populate the new folder with it's existing files

    :param dst_dir: (String) Path to directory we want to populate data

    :param file_name: (String) Path to the file + the file's name that we want to copy
    :return: (String) ok
    """
    print("Populating " + file_name + " from /stub")
    copyfile("./stub/" + file_name, dst_dir + "/" + file_name)


def random_hash():
    # Random hash should be used as folder name for each new deployment
    hash = random.getrandbits(128)
    hash = "%032x" % hash
    return hash


def check_compile_status(deployment_hash):
    generated_folder = "generated/" + deployment_hash  # Where we'll be looking for the compilation status
    file = None
    status = None
    try:
        file = open(generated_folder + "/success.txt")
        status = "success"
    except IOError:
        print("Looking for success.txt file in " + generated_folder)

    try:
        file = open(generated_folder + "/error.txt")
        status = "error"
    except IOError:
        print("Looking for error.txt file in " + generated_folder)

    # If could not find success or error file, the compiling progress maybe is still in-progress
    if not file:
        return {
                   "status": "in-progress",
                   "payload": ""
               }, 200

    # Return compilation result to user
    print("Found " + status + ".txt file in " + generated_folder)
    payload = file.read()
    return status, payload


def get_file(path):
    """
    Look for files in a folder

    :param path: (String) Path to the directory where we want to get all the file names inside
    :return: (Array) File names
    """
    files = [f for f in os.listdir(path) if os.path.isfile(os.path.join(path, f))]
    # Remove hidden files
    files = [f for f in files if f[0] != "."]
    return files


def get_abi_files(compilation_id):
    """
    Build a new array of abi object from the /generated/hash/abis folder

    :param compilation_id: (String) Hash Identifier of the new index. It's also the name of the folders in the generated folder
    :return: (Array) ABI file objects (name of the file, path to the file)
    """
    abi = []
    for file_name in get_file(os.path.join("./generated", compilation_id, "abis")):
        abi_object = {
            "name": file_name,
            "path": os.path.join("./generated", compilation_id, "abis", file_name)
        }
        abi.append(abi_object)
    return abi


def upload_abi_to_ipfs(client, abi):
    """
    Upload abi files to IPFS and build a new abi object for ease of access

    :param client: IPFS Client
    :param abi: ABI Objects (name, hash)
    :return: (Array) ABI file objects (name of the file, path to the file, hash of the IPFS upload result)
    """
    abi_new = []
    for abi_object in abi:
        # Upload to IPFS
        res = client.add(abi_object["path"])
        # Build a new abi object with more attribute
        abi_object["hash"] = res["Hash"]
        abi_new.append(abi_object)
    return abi


def ipfs_client_init():
    if os.environ.get('IPFS_URL'):
        return ipfshttpclient.connect(os.environ.get('IPFS_URL'))  # Connect with IPFS container name
    else:
        return ipfshttpclient.connect()


def get_index_manager_url():
    if os.environ.get('INDEX_MANAGER_URL'):
        return os.environ.get('INDEX_MANAGER_URL')  # Connection to indexer
    else:
        return 'http://0.0.0.0:3030'