from flask import Flask, request
from flask_cors import CORS
from shutil import copyfile
import urllib.parse
import random
import os
import subprocess

################
# Config Flask #
################
app = Flask(__name__)
cors = CORS(app)

###################
# Helper function #
###################
def write_to_disk(file, data):
    """
    write_to_disk create and save the file to disk

    :param file: (String) path to the file + the file's name

    :param data: (String) raw data. Any data with "\ n" will be created as newline
    :return: (String) ok
    """
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
    copyfile("./stub/" + file_name, dst_dir + "/" + file_name)

def reader(pipe, queue):
    try:
        with pipe:
            for line in iter(pipe.readline, b''):
                queue.put((pipe, line))
    finally:
        queue.put(None)

@app.route("/compile", methods=['POST'])
def compile_handler():
    # Get data
    data = request.json

    # Random hash should be used as folder name for each new deployment
    hash = random.getrandbits(128)
    hash = "%032x" % hash

    # Create new folder
    os.mkdir(hash)
    os.mkdir(hash + "/src")

    # Populate with existing stub
    populate_stub(hash, "Cargo.lock")
    populate_stub(hash, "Cargo.toml")
    populate_stub(hash, "src/lib.rs")

    # URL-decode the data
    mapping = urllib.parse.unquote_plus(data["mapping.rs"])
    models = urllib.parse.unquote_plus(data["models.rs"])
    schema = urllib.parse.unquote_plus(data["schema.rs"])

    # Save the formatted data from request to disk, ready for compiling
    write_to_disk(hash + "/src/mapping.rs", mapping)
    write_to_disk(hash + "/src/models.rs", models)
    write_to_disk(hash + "/src/schema.rs", schema)

    # Compile the newly created deployment
    manifest_path = hash + "/Cargo.toml"
    print("Compiling...")
    try:
        output = subprocess.check_output(["cargo build --release"], stderr=subprocess.STDOUT, shell=True, universal_newlines=True, cwd=hash)
    except subprocess.CalledProcessError as exc:
        print("Status : FAIL", exc.returncode, exc.output)
        return {
            "status": "error",
            "payload": urllib.parse.quote(exc.output),
        }, 200
    else:
        print("Output: \n{}\n".format(output))
        return {
            "status": "success",
            "payload": urllib.parse.quote(output),
        }, 200

@app.route('/', methods=['GET'])
def get_grade_result():
    return "Code compiler server is up & running", 200

if __name__ == '__main__':
    # start server
    app.run(debug=True)
