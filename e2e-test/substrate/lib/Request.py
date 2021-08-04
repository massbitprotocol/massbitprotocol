import requests
import json

def post_request(url, payload):
    print(payload)
    x = requests.post(url, json=payload)
    print(x.text)
    return json.loads(x.text)

def load_json(path):
    f = open(path,)
    data = json.load(f)
    f.close()
    return data
