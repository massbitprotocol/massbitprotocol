tmux list-sessions | awk 'BEGIN{FS=":"}{print $1}' | xargs -n 1 tmux kill-session -t
cd massbitprotocol/deployment/binary
tmux new-session -d -s "chain-reader" "./chain-reader 2>&1 | tee log/console-chain-reader.log"
