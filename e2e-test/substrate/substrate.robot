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
########################
# Test-substrate-block #
########################
Deploy test-block, then check if data was inserted into DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  __diesel_schema_migrations
    Delete Table If Exists  substrate_block

    # Compile request
    ${object} =  Read So Example  ../../user-example/substrate/so/test-block
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/so
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

    # Check that there is a table with data in it
    Wait Until Keyword Succeeds
    ...  10x
    ...  5 sec
    ...  Pooling Database Data
    ...  SELECT * FROM substrate_block FETCH FIRST ROW ONLY

########################
# Test-substrate-event #
########################
Deploy test-event, then check if data was inserted into DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  __diesel_schema_migrations
    Delete Table If Exists  substrate_event

    # Compile request
    ${object} =  Read So Example  ../../user-example/substrate/so/test-event
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/so
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

    # Check that there is a table with data in it
    Wait Until Keyword Succeeds
    ...  10x
    ...  5 sec
    ...  Pooling Database Data
    ...  SELECT * FROM substrate_event FETCH FIRST ROW ONLY

############################
# Test-substrate-extrinsic #
############################
Deploy test-extrinsic, then check if data was inserted into DB
    # Configuration
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432

    # Remove table if exists
    Delete Table If Exists  __diesel_schema_migrations
    Delete Table If Exists  substrate_extrinsic

    # Compile request
    ${object} =  Read So Example  ../../user-example/substrate/so/test-extrinsic
    ${compile_res}=  Request.Post Request
    ...  ${CODE_COMPILER}/compile/so
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

    # Check that there is a table with data in it
    Wait Until Keyword Succeeds
    ...  10x
    ...  5 sec
    ...  Pooling Database Data
    ...  SELECT * FROM substrate_extrinsic FETCH FIRST ROW ONLY

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
