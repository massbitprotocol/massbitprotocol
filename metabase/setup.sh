#!/bin/sh
sleep 20
echo "Setup the DB"
curl --location --request GET 'localhost:3000/api/session/properties' 2>/dev/null | jq -r '."setup-token"' > token.txt
# Add script to read from .txt and call /api/setup