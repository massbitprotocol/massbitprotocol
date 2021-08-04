# E2E Test for Substrate and Solana template

## Prerequisites
```
pip install robotframework-requests
pip install robotframework-databaselibrary
```
And make sure you have started all the services 

## Run a Substrate test
```
robot --variable JSON_PAYLOAD:payload/[add_payload_file_here].json substrate.robot
```
Example
```
robot --variable JSON_PAYLOAD:payload/extrinsic.json substrate.robot
robot --variable JSON_PAYLOAD:payload/block.json substrate.robot
robot --variable JSON_PAYLOAD:payload/event.json substrate.robot
```

## Log
Open log.html in your browser