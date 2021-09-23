FROM metabase/metabase
RUN apk add jq && apk add tmux  # JQ and tmux are used in the script to create a new metabase account
COPY init.sh .
COPY setup-mb-account.sh .
ENTRYPOINT []
