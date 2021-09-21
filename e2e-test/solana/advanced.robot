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
###########################
# Test-solana-five-tables #
###########################
Deploy test-solana-five-tables, then check if data was inserted into DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  sgd0.__diesel_schema_migrations
    Delete Table If Exists  sgd0.transaction_instruction
    Delete Table If Exists  sgd0.transaction_account
    Delete Table If Exists  sgd0.instruction_detail
    Delete Table If Exists  sgd0.five_table_block
    Delete Table If Exists  sgd0.five_table_transaction

    # Compile request
    ${object} =  Read So Example  ../../user-example/solana/so/five-tables
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/so
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    Wait Until Keyword Succeeds
    ...  60x
    ...  3 sec
    ...  Pooling Status
    ...  ${compile_res["payload"]}

    # Deploy
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy/so
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success

    # Check that there is a table with data in it
    sleep  30 seconds  # Sleep then check all tables instead of pooling data from each one
    Check If Exists In Database  SELECT * FROM sgd0.transaction_instruction FETCH FIRST ROW ONLY
    Check If Exists In Database  SELECT * FROM sgd0.transaction_account FETCH FIRST ROW ONLY
    Check If Exists In Database  SELECT * FROM sgd0.instruction_detail FETCH FIRST ROW ONLY
    Check If Exists In Database  SELECT * FROM sgd0.five_table_block FETCH FIRST ROW ONLY
    Check If Exists In Database  SELECT * FROM sgd0.five_table_transaction FETCH FIRST ROW ONLY


#############################
## Test-solana-index-serum #
#############################
Deploy test-solana-index-serum, then check if data was inserted into DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  sgd0.__diesel_schema_migrations
    Delete Table If Exists  sgd0.serum_instruction_detail
    Delete Table If Exists  sgd0.serum_transaction_instruction
    Delete Table If Exists  sgd0.serum_transaction_account
    Delete Table If Exists  sgd0.serum_transaction
    Delete Table If Exists  sgd0.serum_block

    # Compile request
    ${object} =  Read So Example  ../../user-example/solana/so/index-serum
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/so
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    Wait Until Keyword Succeeds
    ...  60x
    ...  3 sec
    ...  Pooling Status
    ...  ${compile_res["payload"]}

    # Deploy
    ${json}=  Convert String to JSON  {"compilation_id": "${compile_res["payload"]}"}
    ${deploy_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/deploy/so
    ...  ${json}
    Should be equal  ${deploy_res["status"]}  success

    # Check that there is a table with data in it
    Wait Until Keyword Succeeds
    ...  10x
    ...  3 sec
    ...  Pooling Database Data
    ...  SELECT * FROM sgd0.serum_transaction FETCH FIRST ROW ONLY

    Wait Until Keyword Succeeds
    ...  10x
    ...  3 sec
    ...  Pooling Database Data
    ...  SELECT * FROM sgd0.serum_transaction_account FETCH FIRST ROW ONLY

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
