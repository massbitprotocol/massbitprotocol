*** Settings ***
Library  RequestsLibrary

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3030
${IPFS}  http://localhost:5001
${SOLANA_PROXY}  https://mainnet-beta-solana.massbit.io
${HASURA_ENGINE}  http://localhost:8080
${HASURA_CONSOLE}  http://localhost:3000
${DASHBOARD}  http://localhost:8088

*** Test Cases ***
Check code-compiler is up
    ${response}=  GET  ${CODE_COMPILER} 

Check index-manager is up
    ${response}=  GET  ${INDEX_MANAGER}

Check ipfs is up
    ${response}=  GET  ${IPFS}/api/v0/swarm/peers

Check solana-proxy is up
    ${response}=  GET  ${SOLANA_PROXY}

Check hasura-engine is up
    ${response}=  GET  ${HASURA_ENGINE}/healthz

Check hasura-console is up
    ${response}=  GET  ${HASURA_CONSOLE}

Check dashboard is up
    ${response}=  GET  ${DASHBOARD}
