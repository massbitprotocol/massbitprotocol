from scalecodec.type_registry import load_type_registry_file
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException
import os
import json

# import logging
# logging.basicConfig(level=logging.DEBUG)


custom_type_registry = load_type_registry_file("worker_agent/provider/custom_type.json")



try:
    substrate = SubstrateInterface(
        url="ws://127.0.0.1:9944",
        #url="ws://13.54.43.185/websocket",
        ss58_format=42,
        type_registry_preset='substrate-node-template',
        type_registry=custom_type_registry
    )
except ConnectionRefusedError:
    print("⚠️ No local Substrate node running, try running 'start_local_substrate_node.sh' first")
    exit()

# Set block_hash to None for chaintip
block_hash = "0x308d048f481d30a24dbbd8b2a728691296b51ad653dab8e30c237e05a1fddeff"

# Account
keypair_alice = Keypair.create_from_uri('//Alice')
print("Alice address: {}",keypair_alice)

keypair_bob = Keypair.create_from_uri('//Bob')
print("Bob address: {}",keypair_bob)

call = substrate.compose_call(
    call_module='MassbitModule',
    call_function='register_worker',
    call_params={
        'ip': "1.2.3.5",
        'job_proposal_id': 0
    }
)

extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair_alice)

try:
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    print("Extrinsic '{}' sent and included in block '{}'".format(receipt.extrinsic_hash, receipt.block_hash))

except SubstrateRequestException as e:
    print("Failed to send: {}".format(e))