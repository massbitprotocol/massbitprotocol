import requests
import json

def post_request_with_header(url):
    headers = {'content-type': 'application/json'}
    x = requests.post(url, json={}, headers=headers)
    return json.loads(x.text)
