*** Settings ***
Library  RequestsLibrary
Library  OperatingSystem
Library  RPA.JSON
Library  DatabaseLibrary
Library  ../core-lib/request.py
Library  ../core-lib/pgconnection.py
Library  ../core-lib/example-reader.py

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3000

*** Test Cases ***
#######################
# Test-ethereum-block #
#######################
Compile and Deploy Test Ethereum Block
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Compile request
    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/test-block  src
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/wasm
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    Wait Until Keyword Succeeds
    ...  60x
    ...  10 sec
    ...  Pooling Status
    ...  ${compile_res["payload"]}

    # Deploy
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "model": "MasterChef"}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy/wasm
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success

#######################
# Test-ethereum-event #
#######################
Compile and Deploy Test Ethereum Event
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Compile request
    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/test-event  src
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/wasm
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    Wait Until Keyword Succeeds
    ...  60x
    ...  10 sec
    ...  Pooling Status
    ...  ${compile_res["payload"]}

    # Deploy
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "model": "StandardToken"}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy/wasm
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success


###################
## Test-quickswap #
###################
#Compile and Deploy Test Ethereum Event
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Compile request
#    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/quickswap  src/mappings
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile/wasm
#    ...  ${object}
#    Should be equal  ${compile_res["status"]}  success
#
#    # Compile status
#    Wait Until Keyword Succeeds
#    ...  60x
#    ...  10 sec
#    ...  Pooling Status
#    ...  ${compile_res["payload"]}
#
#    # Deploy
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "model": "Factory"}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/wasm
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success

###################
# Helper Function #
###################
*** Keywords ***
Pooling Status
    [Arguments]  ${payload}
    ${status_res} =    GET  ${CODE_COMPILER}/compile/status/${payload}  expected_status=200
    Should be equal   ${status_res.json()}[status]  success

Pooling Database Data
    [Arguments]  ${query}
    Check If Exists In Database  ${query}
