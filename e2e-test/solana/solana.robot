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
#####################
# Test-solana-block #
#####################
#Deploy test-solana-block, then check if data exists in DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  block
#
#    # Compile request
#    ${object} =  Read Index Example  ../../user-example/solana/test-solana-block/src
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile
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
#    ...  ${CODE_COMPILER}/deploy
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#    # Check that there is a table with data in it
#    Wait Until Keyword Succeeds
#    ...  12x
#    ...  5 sec
#    ...  Pooling Database Data
#    ...  SELECT * FROM block FETCH FIRST ROW ONLY


############################
## Test-solana-transaction #
############################
#Deploy test-solana-transaction, then check if data exists in DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  transaction
#
#    # Compile request
#    ${object} =  Read Index Example  ../../user-example/solana/test-solana-transaction/src
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile
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
#    ...  ${CODE_COMPILER}/deploy
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#
#    # Check that there is a table with data in it
#    Wait Until Keyword Succeeds
#    ...  12x
#    ...  5 sec
#    ...  Pooling Database Data
#    ...  SELECT * FROM transaction FETCH FIRST ROW ONLY


#############################
## Test-solana-log-messages #
#############################
Deploy test-solana-log-messages, then check if data exists in DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  solana_log_messages

    # Compile request
    ${object} =  Read Index Example  ../../user-example/solana/test-solana-log-messages/src
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    Wait Until Keyword Succeeds
    ...  60x
    ...  10 sec
    ...  Pooling Status
    ...  ${compile_res["payload"]}

    # Deploy
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success
    sleep  20 seconds  # Wait for indexing

    # Check that there is a table with data in it
#    Check If Exists In Database  SELECT * FROM solana_log_messages FETCH FIRST ROW ONLY
    # Check that there is a table with data in it
    Wait Until Keyword Succeeds
    ...  12x
    ...  5 sec
    ...  Pooling Database Data
    ...  SELECT * FROM solana_log_messages_ts FETCH FIRST ROW ONLY


############################
## Test-solana-five-tables #
############################
#Deploy test-solana-five-tables, then check if data exists in DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  transaction_instruction
#    Delete Table If Exists  transaction-account
#    Delete Table If Exists  instruction_detail
#    Delete Table If Exists  transaction
#    Delete Table If Exists  block
#
#    # Compile request
#    ${object} =  Read Index Example  ../../user-example/solana/five-tables/src
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile
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
#    ...  ${CODE_COMPILER}/deploy
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#    sleep  20 seconds  # Wait for indexing
#
#    # Check that there is a table with data in it
#    Check If Exists In Database  SELECT * FROM transaction_instruction FETCH FIRST ROW ONLY
#    Check If Exists In Database  SELECT * FROM transaction_account FETCH FIRST ROW ONLY
#    Check If Exists In Database  SELECT * FROM instruction_detail FETCH FIRST ROW ONLY
#    Check If Exists In Database  SELECT * FROM transaction FETCH FIRST ROW ONLY
#    Check If Exists In Database  SELECT * FROM block FETCH FIRST ROW ONLY
#
#
#############################
## Test-solana-index-serum #
#############################
#Deploy test-solana-index-serum, then check if data exists in DB
#    # Configuration
#    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
#
#    # Remove table if exists
#    Delete Table If Exists  serum_instruction_detail
#    Delete Table If Exists  serum_transaction_instruction
#    Delete Table If Exists  serum_transaction_account
#    Delete Table If Exists  serum_transaction
#    Delete Table If Exists  serum_block
#
#    # Compile request
#    ${object} =  Read Index Example  ../../user-example/solana/index-serum/src
#    ${compile_res}=  Request.Post Request
#    ...  ${CODE_COMPILER}/compile
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
#    ...  ${CODE_COMPILER}/deploy
#    ...  ${json}
#    Should be equal  ${deploy_res["status"]}  success
#    sleep  20 seconds  # Wait for indexing
#
#    # Check that there is a table with data in it
#    Check If Exists In Database  SELECT * FROM serum_transaction FETCH FIRST ROW ONLY
#    Check If Exists In Database  SELECT * FROM serum_transaction_account FETCH FIRST ROW ONLY


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
