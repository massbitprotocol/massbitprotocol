*** Settings ***
Documentation          Check if our services are still running in production

Library                SSHLibrary
Suite Setup            Open Connection And Log In
Suite Teardown         Close All Connections

*** Variables ***
${WORK_DIRECTORY}      work/massbitprotocol

*** Test Cases ***
Check if Substrate Adapter is still indexing
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 index-manager.log
    Should Contain     ${output}          [Substrate-Adapter]

Check if Solana Adapter is still indexing
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 index-manager.log
    Should Contain     ${output}          [Solana-Adapter]

Check if Chain Reader is still receiving Solana Data
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 chain-reader.log
    Should Contain     ${output}          [chain_reader::solana_chain - tokio-runtime-worker]

Check if Chain Reader is still receiving Substrate Data
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 chain-reader.log
    Should Contain     ${output}          [chain_reader::substrate_chain - tokio-runtime-worker]

Check if Chain Reader is still receiving Ethereum Data
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 chain-reader.log
    Should Contain     ${output}          [chain_reader::ethereum_chain - tokio-runtime-worker]

# After about 30 minutes - 1 hour. There will be a folder log where you ran the test.
#Download logs
#    ${output}=         Get Directory      ${WORK_DIRECTORY}/log

*** Keywords ***
Open Connection And Log In
   Open Connection     ${SSH_HOST}            port=${SSH_PORT}
   Login               ${SSH_USERNAME}        ${SSH_PASSWORD}