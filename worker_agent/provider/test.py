import requests
import json

method = "eth_getBlockTransactionCountByNumber"
block_number = "0x123456"
url = "http://54.252.153.209/"
payload = "{\n    \"jsonrpc\": \"2.0\",\n    \"method\": \""+ method + "\",\n    \"params\": [\""+ block_number +"\"],\n    \"id\": 1\n}"

print(f"payload:{payload}")
headers = {
    'content-type': "application/json",
    'cache-control': "no-cache",
    'postman-token': "9958d5aa-dc79-112c-df38-e1c106a726d7"
    }
response = requests.request("POST", url, data=payload, headers=headers)

print(response.text)
response = json.loads(response.text)

# result = {}
# if "result" in response:
#     result["block_number"] = int(response["result"]["number"])
#     result["block_hash"] = int(response["result"]["hash"])
#     print(result)
