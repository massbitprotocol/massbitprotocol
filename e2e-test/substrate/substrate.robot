*** Settings ***
Library  RequestsLibrary
Library  OperatingSystem
Library  lib/Request.py

*** Test Cases ***
Check code-compiler is up
    ${response}=    GET  http://localhost:5000  # Refactor this as global variable

Create a new compile request and check if it's running
    ${object} =  Load JSON  payload/extrinsic.json
    ${compile_res}=    Request.Post Request  http://localhost:5000/compile  ${object}
    Should be equal   ${compile_res["status"]}  success

    ${status_res}=    GET  http://localhost:5000/compile/status/${compile_res["payload"]}  expected_status=200
    Should be equal   ${status_res.json()}[status]  in-progress
    # Need an API to cancel the request so we can clean up the running compilation progress

Create a new compile request and check if it is success after a while
    ${object} =  Load JSON  payload/extrinsic.json
    ${compile_res}=    Request.Post Request  http://localhost:5000/compile  ${object}
    Should be equal   ${compile_res["status"]}  success

    sleep  1 minutes  # Wait for the compilation

    ${status_res}=    GET  http://localhost:5000/compile/status/${compile_res["payload"]}  expected_status=200
    Should be equal   ${status_res.json()}[status]  success
