tmux list-sessions | awk 'BEGIN{FS=":"}{print $1}' | xargs -n 1 tmux kill-session -t
sleep 5
cd massbitprotocol/deployment/binary
#tmux new-session -d -s "chain-reader" "./chain-reader 2>&1 | tee log/console-chain-reader.log"
tmux new-session -d -s "chain-reader" "./chain-reader"
