*** Settings ***
Library  RequestsLibrary

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3000

*** Test Cases ***
Check code-compiler is up
    ${response}=  GET  ${CODE_COMPILER} 

Check index-manager is up
    ${response}=  GET  ${INDEX_MANAGER} 
