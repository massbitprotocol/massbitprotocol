#!/bin/sh
# This script is to start the metabase app and start a new tmux with timer to setup a new account
tmux new -d -s setup ./setup-mb-account.sh
/app/run_metabase.sh
