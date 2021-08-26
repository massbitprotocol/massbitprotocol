*** Settings ***
Library           SeleniumLibrary

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3000
${DASHBOARD}      http://localhost:8088
${BROWSER}        Firefox

*** Keywords ***
Open Browser To Login Page
    Open Browser    ${DASHBOARD}    ${BROWSER}
    Maximize Browser Window
    Set Selenium Speed    ${DELAY}
    Login Page Should Be Open

*** Test Cases ***
Valid Login
    Open Browser To Login Page