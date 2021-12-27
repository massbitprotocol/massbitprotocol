tmux list-sessions | awk 'BEGIN{FS=":"}{print $1}' | xargs -n 1 tmux kill-session -t;
cd massbitprotocol/deployment/binary
tmux new-session -d -s "indexer-api" "./indexer-api"
tmux new-session -d -s "massbit-graphql" "./massbit-graphql --http-port 3041"



