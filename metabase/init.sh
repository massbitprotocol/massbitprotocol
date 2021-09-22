#!/bin/sh
tmux new -d -s setup ./setup.sh
/app/run_metabase.sh
