*** Settings ***
Library  RequestsLibrary
Library  OperatingSystem
Library  lib/Request.py
Library  RPA.JSON
Library  DatabaseLibrary

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3000
#${JSON_PAYLOAD}   payload/[add_payload_file_here].json   # or pass it in by --variable in the commandline

*** Test Cases ***
######################
# Prerequisite tests #
######################
Check code-compiler is up
    ${response}=  GET  ${CODE_COMPILER} 

Check index-manager is up
    ${response}=  GET  ${INDEX_MANAGER} 

##############
# Main tests #
##############
Compile extrinsic & check if it's running
    # Compile request
    ${object} =  Load JSON  ${JSON_PAYLOAD}
    ${compile_res}=  Request.Post Request  
    ...  ${CODE_COMPILER}/compile  
    ...  ${object}
    Should be equal  ${compile_res["status"]}  success

    # Compile status
    ${status_res}=  GET  
    ...  ${CODE_COMPILER}/compile/status/${compile_res["payload"]}  
    ...  expected_status=200
    # Need an API to cancel the request so we can clean up the running compilation progress
    Should be equal  ${status_res.json()}[status]  in-progress  


Compile and Deploy extrinsic, then check if data exists in DB
    # Compile request
    ${object} =  Load JSON  ${JSON_PAYLOAD}
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

    # Check that a table is created with data in it
    Connect To Database  psycopg2  graph-node  graph-node  let-me-in  localhost  5432
    Check If Exists In Database  SELECT * FROM substrate_extrinsic FETCH FIRST ROW ONLY

*** Keywords ***
Pooling Status
    [Arguments]  ${payload}
    ${status_res} =    GET  ${CODE_COMPILER}/compile/status/${payload}  expected_status=200
    Should be equal   ${status_res.json()}[status]  success

