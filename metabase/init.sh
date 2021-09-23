#!/bin/sh
tmux new -d -s setup ./setup-mb-account.sh
/app/run_metabase.sh
