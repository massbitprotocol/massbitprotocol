*** Settings ***
Library  RequestsLibrary
Library  OperatingSystem
Library  lib/Request.py
Library  RPA.JSON
Library  DatabaseLibrary

*** Test Cases ***
######################
# Prerequisite tests #
######################
Check code-compiler is up
    ${response}=    GET  http://localhost:5000  # Refactor this as global variable

##############
# Main tests #
##############
Compile extrinsic & check if it's running
    ${object} =  Load JSON  payload/extrinsic.json
    ${compile_res}=    Request.Post Request  http://localhost:5000/compile  ${object}
    Should be equal   ${compile_res["status"]}  success

    ${status_res}=    GET  http://localhost:5000/compile/status/${compile_res["payload"]}  expected_status=200
    Should be equal   ${status_res.json()}[status]  in-progress
    # Need an API to cancel the request so we can clean up the running compilation progress

Compile and Deploy extrinsic, then check if data exists in DB
    # Compile request
    ${object} =  Load JSON  payload/extrinsic.json
    ${compile_res}=    Request.Post Request  http://localhost:5000/compile  ${object}
    Should be equal   ${compile_res["status"]}  success 

    # Compile status
    Wait Until Keyword Succeeds    40x    10 sec     Pooling Status    ${compile_res["payload"]}

    # Deploy
    ${json}=    Convert String to JSON    {"compilation_id": "${compile_res["payload"]}"}
    ${deploy_res}=    Request.Post Request    http://localhost:5000/deploy  ${json}
    Should be equal   ${deploy_res["status"]}   success
    sleep  10 seconds  # Wait for indexing

    # Check that a table is created with data in it
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
    Check If Exists In Database  SELECT * FROM substrate_extrinsic FETCH FIRST ROW ONLY

*** Keywords ***
Pooling Status
    [Arguments]  ${payload}
    ${status_res} =    GET  http://localhost:5000/compile/status/${payload}  expected_status=200
    Should be equal   ${status_res.json()}[status]  success

