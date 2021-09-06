*** Settings ***
Documentation          Check if our services are still running in production

Library                SSHLibrary
Suite Setup            Open Connection And Log In
Suite Teardown         Close All Connections

*** Variables ***
${HOST}                6.tcp.ngrok.io
${SSH_USERNAME}        massbit
${WORK_DIRECTORY}      work/massbitprotocol

*** Test Cases ***
Check if Substrate Adapter is still indexing
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 index-manager.log
    Should Contain     ${output}          Substrate

Check if Solana Adapter is still indexing
    ${output}=         Execute Command    cd ${WORK_DIRECTORY}/log && tail -100 index-manager.log
    Should Contain     ${output}          Solana

*** Keywords ***
Open Connection And Log In
   Open Connection     ${HOST}                port=18268
   Login               ${SSH_USERNAME}        ${SSH_PASSWORD}