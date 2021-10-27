### General Account
- Call
```json
  {
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getAccountInfo",
  "params": [
    "AQYXcjg1ttjfnVA7VUi1qNnePNKjCZo53M8VfvaT9Eq9",
    {
      "encoding": "jsonParsed"
    }
  ]
}
```
- Return
```json
{
  "jsonrpc": "2.0",
  "result": {
    "accountType": "Account",
    "context": {
      "slot": 103579499
    },
    "value": {
      "data": [
        "",
        "base64"
      ],
      "executable": false,
      "lamports": 11430470,
      "owner": "11111111111111111111111111111111",
      "rentEpoch": 239
    }
  },
  "id": 1
}
```

### Program account
- Link
https://solscan.io/account/SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ/
- Call
```json
{"jsonrpc": "2.0", "method": "getAccountInfo", "params": ["SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ","jsonParsed"], "id":1 }
```
- Return
```json
{
    "jsonrpc": "2.0",
    "result": {
      "accountType": "ProgramAccount",
        "context": {
            "slot": 103553847
        },
        "value": {
            "data": {
                "parsed": {
                    "info": {
                        "programData": "54aePuBcYcf8G3CDrWWd2MEiw6Q7UGy2kjgQBDhqoMdt"
                    },
                    "type": "program"
                },
                "program": "bpf-upgradeable-loader",
                "space": 36
            },
            "executable": true,
            "lamports": 1141440,
            "owner": "BPFLoaderUpgradeab1e11111111111111111111111",
            "rentEpoch": 185
        }
    },
    "id": 1
}
```

### Token Account
- Call
```json
  {
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getAccountInfo",
  "params": [
    "2C82bL2X7y5PwbsnxMQjAQW7CC8dcqtrh4ZyGAd5NBpZ",
    {
      "encoding": "jsonParsed"
    }
  ]
}
```
- Return
```json
{
  "jsonrpc": "2.0",
  "result": {
    "accountType": "TokenAccount",
    "context": {
      "slot": 103580015
    },
    "value": {
      "data": {
        "parsed": {
          "info": {
            "isNative": false,
            "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "owner": "AQYXcjg1ttjfnVA7VUi1qNnePNKjCZo53M8VfvaT9Eq9",
            "state": "initialized",
            "tokenAmount": {
              "amount": "0",
              "decimals": 6,
              "uiAmount": 0,
              "uiAmountString": "0"
            }
          },
          "type": "account"
        },
        "program": "spl-token",
        "space": 165
      },
      "executable": false,
      "lamports": 2039280,
      "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "rentEpoch": 239
    }
  },
  "id": 1
}
```

### Mint account
- Call
```json
  {
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getAccountInfo",
  "params": [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    {
      "encoding": "jsonParsed"
    }
  ]
}
```
- Return
```json
{
  "jsonrpc": "2.0",
  "result": {
    "accountType": "MintAccount",
    "context": {
      "slot": 103580291
    },
    "value": {
      "data": {
        "parsed": {
          "info": {
            "decimals": 6,
            "freezeAuthority": "3sNBr7kMccME5D55xNgsmYpZnzPgP2g12CixAajXypn6",
            "isInitialized": true,
            "mintAuthority": "2wmVCSfPxGPjrnMMn7rchp4uaeoTqN39mXFC2zhPdri9",
            "supply": "2485000019865882"
          },
          "type": "mint"
        },
        "program": "spl-token",
        "space": 82
      },
      "executable": false,
      "lamports": 76771211333,
      "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "rentEpoch": 239
    }
  },
  "id": 1
}
```
### Invalid address
- Response
```json
{
    "jsonrpc": "2.0",
    "error": {
        "code": -32602,
        "message": "ClientError { request: Some(GetAccountInfo), kind: RpcError(RpcResponseError { code: -32602, message: \"Invalid param: WrongSize\", data: Empty }) }"
    },
    "id": 1
}
```

### Notes:
    - Use the `accountType` field in `result` for determine the account type.
    - There are 4 account types: `ProgramAccount`, `Account`, `TokenAccount` and `MintAccount`
