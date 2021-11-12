from compiler.so import deploy_so, compile_so
from compiler.wasm import compile_wasm, deploy_wasm
from flask import Flask, request
from flask_cors import CORS, cross_origin
import hashlib

################
# Config Flask #
################
from helper.helper import random_hash, check_compile_status

app = Flask(__name__)
cors = CORS(app)



@app.route("/compile/so", methods=['POST'])
@cross_origin()
def compile_handler():
    data = request.json
    deployment_hash = compile_so(data)
    return {
               "status": "success",
               "payload": deployment_hash,
           }, 200



@app.route("/compile/wasm", methods=['POST'])
@cross_origin()
def compile_wasm_handler():
    data = request.json
    deployment_hash = compile_wasm(data)
    return {
               "status": "success",
               "payload": deployment_hash,
           }, 200


@app.route("/compile/status/<deployment_hash>", methods=['GET'])
@cross_origin()
def compile_status_handler(deployment_hash):
    status, payload = check_compile_status(deployment_hash)

    return {
               "status": status,
               "payload": payload
           }, 200


@app.route("/deploy/so", methods=['POST'])
@cross_origin()
def deploy_handler():
    data = request.json
    res = deploy_so(data)

    return res, 200


@app.route("/deploy/wasm", methods=['POST'])
@cross_origin()
def deploy_wasm_handler():
    data = request.json
    deploy_wasm(data)
    return {
               "status": "success",
               "payload": "",
           }, 200


@app.route('/', methods=['GET'])
@cross_origin()
def index():
    return "Code compiler server is up & running", 200


if __name__ == '__main__':
    # start server
    app.run(host="0.0.0.0", debug=True)
