#!/bin/sh
# This script is to automatically call /api to init a new account for Metabase because they don't support any method to define this in Docker
sleep 25 # Refactor to wait-for-it script
echo "Setting up the account for Metabase" >> setup-mb-account.log
curl --location --request GET 'localhost:3000/api/session/properties' 2>/dev/null | jq -r '."setup-token"' > token.txt
curl --location --request POST 'localhost:3000/api/setup/' \
  --header 'Content-Type: application/json' \
  --data-raw '{
    "token": "'$(cat token.txt)'",
    "prefs": {
      "site_name": "codelight",
      "site_locale": "en",
      "allow_tracking": "false"
    },
    "database": {
      "engine": "postgres",
      "name": "postgres",
      "details": {
        "host": "postgres",
        "port": null,
        "dbname": "graph-node",
        "user": "graph-node",
        "password": "let-me-in",
        "ssl": false,
        "additional-options": null,
        "tunnel-enabled": false
      },
      "is_full_sync": true,
      "schedules": {}
    },
    "user": {
      "first_name": "codelight",
      "last_name": "admin",
      "email": "admin@codelight.co",
      "password": "Codelight123",
      "site_name": "codelight"
    }
  }' >> setup-mb-account.log
