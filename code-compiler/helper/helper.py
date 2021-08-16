import errno
import random
import os
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
