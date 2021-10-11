FROM metabase/metabase
RUN apk add jq && apk add tmux  # JQ and tmux are used to automatically create a new metabase account
COPY init.sh .
COPY setup-mb-account.sh .
ENTRYPOINT []
