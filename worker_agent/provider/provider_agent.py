from scalecodec.type_registry import load_type_registry_file
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException
import os
import json
from requests import get


# import logging
# logging.basicConfig(level=logging.DEBUG)

file_custom_type = "worker_agent/provider/custom_type.json"

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


def register_to_massbit(account_name,proposal_id,ip,substrate):
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
            'job_proposal_id': 0
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)

    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print("Extrinsic '{}' sent and included in block '{}'".format(receipt.extrinsic_hash, receipt.block_hash))

    except SubstrateRequestException as e:
        print("Failed to send: {}".format(e))

if __name__ == "__main__":

    # Get env info
    ip = get('https://api.ipify.org').text
    print('My public IP address is: {}'.format(ip))

    if "MASSBIT_ACCOUNT" in os.environ:
        account_name = os.environ["MASSBIT_ACCOUNT"]
    else:
        print("Cannot find MASSBIT_ACCOUNT env variable")
        account_name = "Alice"

    if "PROPOSAL_ID" in os.environ:
        proposal_id = os.environ["PROPOSAL_ID"]
    else:
        print("Cannot find PROPOSAL_ID env variable")
        proposal_id = 0

    if "MASSBIT_URL" in os.environ:
        massbit_url = os.environ["MASSBIT_URL"]
    else:
        print("Cannot find MASSBIT_URL env variable")
        massbit_url = "wss://dev-api.massbit.io/websocket"

    # Connect to massbit substrate
    substrate = connect_massbit(massbit_url,file_custom_type)

    # Register worker to massbit
    register_to_massbit(account_name,proposal_id,ip,substrate)