from scalecodec.type_registry import load_type_registry_file
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException
import os
import json
from time import sleep
#from flask import Flask, request
from requests import get
from apscheduler.schedulers.background import BackgroundScheduler
import requests
import hashlib

############################# Changing Provider behavior (Demo only) ##################################
# app = Flask(__name__)
# is_good_provider = 1
# @app.route("/")
# def index():
#     value = request.args.get('is_good_provider')
#     if value is None:
#         return "<h1>no parameter is set<h1>"
#     if int(value) == 0:
#         is_good_provider = 0
#         return '''<h1>The provider behavior is set to Bad</h1>'''.format(is_good_provider)
#     else:
#         is_good_provider = 1
#         return '''<h1>The provider behavior is set to Good</h1>'''.format(is_good_provider)
############################# Changing Provider behavior (Demo only) ##################################

# import logging
# logging.basicConfig(level=logging.DEBUG)

IS_TEST = True

file_custom_type = "worker_agent/provider/custom_type.json"
CHECK_REPORT_PERIOD = 5 #sec


def array_of_bytes_to_string(arr):
    try:
        result = bytes(arr).decode("utf-8")
    except:
        result = "error"
    return result

def create_request_url_from_report(report):
    return ""

def get_available_reports(massbit_url):

    ####### Reformat report #######
    # report={
    #   job_report_index: int,
    #   job_input: String,
    #   job_output: String
    # }
    reports = []

    payload = "{\n     \"jsonrpc\":\"2.0\",\n      \"id\":1,\n      \"method\":\"massbit_getJobReports\",\n      \"params\": []\n    }"
    headers = {
        'content-type': "application/json;charset=utf-8",
        'cache-control': "no-cache",
        'postman-token': "804a2911-abb0-64d3-4710-35210aeec73f"
        }

    response = requests.request("POST", massbit_url, data=payload, headers=headers)
    #print(response.text)

    response = json.loads(response.text)
    response = response["result"]
    for element in response:
        report = {
            "job_report_id": int(element[0]),
            "job_input": array_of_bytes_to_string(element[1]),
            "job_output": array_of_bytes_to_string(element[2]),
        }

        reports.append(report)

    #print (f"Reports:{reports}")
    return reports

def get_workers(massbit_url):

    workers = {}

    payload = "{\n     \"jsonrpc\":\"2.0\",\n      \"id\":1,\n      \"method\":\"massbit_getWorkers\",\n      \"params\": []\n    }"
    headers = {
        'content-type': "application/json;charset=utf-8",
        'cache-control': "no-cache",
        'postman-token': "804a2911-abb0-64d3-4710-35210aeec73f"
        }

    response = requests.request("POST", massbit_url, data=payload, headers=headers)
    #print(response.text)

    response = json.loads(response.text)
    response = response["result"]
    #print(response)
    for element in response:
        worker_ip = array_of_bytes_to_string(element[1])
        workers[worker_ip]= element[0]
        
    print (f"Workers:{workers}")
    return workers

# Get job result from localhost
def get_job_result(url,method,block_number):

    if not IS_TEST:
        url = "http://localhost"

    payload = "{\n    \"jsonrpc\": \"2.0\",\n    \"method\": \""+ method + "\",\n    \"params\": [\""+ block_number +"\"],\n    \"id\": 1\n}"
    headers = {
        'content-type': "application/json",
        'cache-control': "no-cache",
        'postman-token': "9958d5aa-dc79-112c-df38-e1c106a726d7"
        }
    response = requests.request("POST", url, data=payload, headers=headers)

    print(response.text)

    return response.text

def vote_report(voted_worker_id,my_result, report):
    # Check my_result and report result
    verify_agree = my_result==report["job_output"]

    # if (verify_agree):
    #     print(f"Correct result: {report['job_output']}")
    # else:
    #     print(f"Wrong result: {report['job_output']}, my result:{my_result}")

    call_extrinsic_vote_report(voted_worker_id,report["job_report_id"], verify_agree)
    print(f"#### Worker {voted_worker_id} vote {verify_agree} for report {report['job_report_id']}!" )

# Hash the output string
def hash_output(output_string):
    return hashlib.sha3_256(output_string.encode()).hexdigest()


def check_reports_and_vote():
    print("Check and vote report...")
    # Get reports
    reports = get_available_reports(massbit_https)

    for report in reports:
        # Get result
        try:
            job_input = json.loads(report["job_input"])
            #job_input["url"] = "http://54.252.153.209/"
            #job_input["method"] = "eth_getBlockTransactionCountByNumber"   #Notes: Change soon
            #job_input["block_number"] = "0x123456"
            #job_input["job_proposal"] = 0
        except:
            print(f"Cannot parse JSON string job_input: {report['job_input']}")
            continue
        print(f"job_input:{job_input}")

        # if not enough data
        if len(job_input) != 4:
            print(f"Not enough job input.")
            continue    

        # if job_proposal is difference
        if job_input["job_proposal"] != massbit_proposal_id:
            # do nothing
            print(f"job_proposal is difference.")
            continue

        #print(f"job_input:{job_input}")
        my_result = get_job_result(job_input["url"],job_input["method"],job_input["block_number"])
        #print(f"my_result:{my_result}")
        # Hash the result
        my_result = hash_output(my_result)
        #print(f"my_result hash: {my_result}")

        # Vote report
        vote_report(worker_id,my_result, report)

    print("Finished check and vote report.")

def connect_massbit(massbit_url,file_custom_type):

    custom_type_registry = load_type_registry_file(file_custom_type)

    try:
        substrate = SubstrateInterface(
            url=massbit_url,
            ss58_format=42,
            type_registry_preset='substrate-node-template',
            type_registry=custom_type_registry
        )
 
    except ConnectionRefusedError:
        print("⚠️ No local Substrate node running, try running 'start_local_substrate_node.sh' first")
        exit()

    return substrate

# Register worker extrinsic
def call_extrinsic_register_worker(account_name,proposal_id,ip,substrate):
    # Set block_hash to None for chaintip
    block_hash = "0x308d048f481d30a24dbbd8b2a728691296b51ad653dab8e30c237e05a1fddeff"

    # Account
    keypair = Keypair.create_from_uri(f'//{account_name}')
    print("Account address: {}",keypair)

    call = substrate.compose_call(
        call_module='MassbitModule',
        call_function='register_worker',
        call_params={
            'ip': ip,
            'job_proposal_id': massbit_proposal_id
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)

    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print("Extrinsic '{}' sent and included in block '{}'".format(receipt.extrinsic_hash, receipt.block_hash))

    except SubstrateRequestException as e:
        print("Failed to send: {}".format(e))


# Vote report extrinsic
def call_extrinsic_vote_report(voted_worker_id,job_report_id, verify_agree):

    # Account
    keypair = Keypair.create_from_uri(f'//{account_name}')
    print("Account address: {}",keypair)

    call = substrate.compose_call(
        call_module='MassbitModule',
        call_function='vote_job_report',
        call_params={
            'voted_worker_id': voted_worker_id,
            'job_report_id': job_report_id,
            'verify_agree': verify_agree
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)

    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print("Extrinsic '{}' sent and included in block '{}'".format(receipt.extrinsic_hash, receipt.block_hash))

    except SubstrateRequestException as e:
        print("Failed to send: {}".format(e))


# Get env info
ip = get('https://api.ipify.org').text
print('My public IP address is: {}'.format(ip))

if "IS_TEST" in os.environ:
    IS_TEST = True
else:
    print("Cannot find IS_TEST env variable")
    IS_TEST = False

if "MASSBIT_ACCOUNT" in os.environ:
    account_name = os.environ["MASSBIT_ACCOUNT"]
else:
    print("Cannot find MASSBIT_ACCOUNT env variable")
    account_name = "Alice"

if "MASSBIT_PROPOSAL_ID" in os.environ:
    massbit_proposal_id = os.environ["MASSBIT_PROPOSAL_ID"]
else:
    print("Cannot find MASSBIT_PROPOSAL_ID env variable")
    massbit_proposal_id = "1"

if "MASSBIT_HTTPS" in os.environ:
    massbit_https = os.environ["MASSBIT_HTTPS"]
else:
    print("Cannot find MASSBIT_HTTPS env variable")
    massbit_https = "https://dev-api.massbit.io/"

if "MASSBIT_WSS" in os.environ:
    massbit_wss = os.environ["MASSBIT_WSS"]
else:
    print("Cannot find MASSBIT_WSS env variable")
    massbit_wss = "wss://dev-api.massbit.io/websocket"



if __name__ == "__main__":

    # Connect to massbit substrate
    substrate = connect_massbit(massbit_wss,file_custom_type)

    # Register worker to massbit
    call_extrinsic_register_worker(account_name,massbit_proposal_id,ip,substrate)

    # Get worker list
    workers = get_workers(massbit_https)

    # Get worker IP
    worker_id = workers[ip]

    print(f"worker_id:{worker_id}")

    # Start check and vote result
    scheduler = BackgroundScheduler()
    job = scheduler.add_job(check_reports_and_vote, 'interval', seconds=CHECK_REPORT_PERIOD)
    scheduler.start()

    while(True):
        sleep(0.1)
        pass
    
    # Start flask server for change provider behavior (Demo only)
    #app.run()