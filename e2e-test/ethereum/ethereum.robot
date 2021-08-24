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
*** Test Cases ***
###########################
## Test-ethereum-block SO #
###########################
#Deploy test-ethereum-block, then check if data was inserted into DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  __diesel_schema_migrations
#    Delete Table If Exists  ethereum_block_table
#
#    # Compile request
#    ${object} =  Read So Example  ../../user-example/ethereum/so/test-ethereum-block
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile/so
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
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/so
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#    # Check that there is a table with data in it
#    Wait Until Keyword Succeeds
#    ...  10x
#    ...  5 sec
#    ...  Pooling Database Data
#    ...  SELECT * FROM ethereum_block_table FETCH FIRST ROW ONLY
#
#
#################################
## Test-ethereum-transaction SO #
#################################
#Deploy test-ethereum-transaction, then check if data was inserted into DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  __diesel_schema_migrations
#    Delete Table If Exists  ethereum_transaction_table
#
#    # Compile request
#    ${object} =  Read So Example  ../../user-example/ethereum/so/test-ethereum-transaction
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile/so
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
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/so
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#    # Check that there is a table with data in it
#    Wait Until Keyword Succeeds
#    ...  10x
#    ...  5 sec
#    ...  Pooling Database Data
#    ...  SELECT * FROM ethereum_transaction_table FETCH FIRST ROW ONLY
#
#################################
## Test-ethereum-event SO #
#################################
#Deploy test-ethereum-event, then check if data was inserted into DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  __diesel_schema_migrations
#    Delete Table If Exists  ethereum_event_table
#
#    # Compile request
#    ${object} =  Read So Example  ../../user-example/ethereum/so/test-ethereum-event
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile/so
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
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/so
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#    # Check that there is a table with data in it
#    Wait Until Keyword Succeeds
#    ...  10x
#    ...  5 sec
#    ...  Pooling Database Data
#    ...  SELECT * FROM ethereum_event_table FETCH FIRST ROW ONLY
#
#############################
## Test-ethereum-block WASM #
#############################
#Compile and Deploy WASM Test Ethereum Block
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Compile request
#    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/test-block  default
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
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "configs":{"model":"MasterChef"}}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/wasm
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#############################
## Test-ethereum-event WASM #
#############################
#Compile and Deploy WASM Test Ethereum Event
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Compile request
#    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/test-event  default
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
#    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "configs":{"model":"StandardToken"}}
#    ${deploy_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/deploy/wasm
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success


#######################
# Test-quickswap WASM #
#######################
Compile and Deploy WASM Test Quickswap
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Compile request
    ${object} =  Read Wasm Example  ../../user-example/ethereum/wasm/quickswap  mappings
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
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}", "configs":{"model":"Factory"}}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy/wasm
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success

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
