*** Settings ***
Library           SeleniumLibrary

*** Variables ***
${CODE_COMPILER}  http://localhost:5000
${INDEX_MANAGER}  http://localhost:3000
${DASHBOARD}      http://localhost:8088
${BROWSER}        Firefox


*** Test Cases ***
Valid Homepage
    Open Browser            ${DASHBOARD}    ${BROWSER}
    Title Should Be         Massbit Dashboard
    Click Element           css:li.nav-item:nth-of-type(2)
    Page Should Contain     Solana
    Page Should Contain     Ethereum
    Page Should Contain     BSC
    Page Should Contain     Matic
    Close Browser

Valid Ethereum createIndexer
    Open Browser            ${DASHBOARD}/#/createIndexer/ethereum   ${BROWSER}
    Title Should Be         Massbit Dashboard

    # has list of examples
    Page Should Contain     test-ethereum-block Templates
    Page Should Contain     test-ethereum-transaction Templates
    Page Should Contain     quickswap Templates
    Page Should Contain     test-block Templates
    Page Should Contain     test-event Templates

    # has configure options
    Page Should Contain     abis
    Page Should Contain     configs
    Page Should Contain     mappings

    # has import github feature
    Page Should Contain     Import Github

    # has compile code feature
    Page Should Contain     Compile code
    Close Browser
