#!/bin/bash

# Function to allow a user to arbitrarily set the terminal title to anything
# Example: `set-title this is title 1`
set-title() {
    # Set the PS1 title escape sequence; see "Customizing the terminal window title" here:
    # https://wiki.archlinux.org/index.php/Bash/Prompt_customization#Customizing_the_terminal_window_title
    TITLE="\[\e]2;$@\a\]"
    PS1=${PS1_BAK}${TITLE}
}

# Back up original PS1 Prompt 1 string when ~/.bashrc is first sourced upon bash opening
if [[ -z "$PS1_BAK" ]]; then # If length of this str is zero (see `man test`)
    PS1_BAK=$PS1
fi

# Set the title to a user-specified value if and only if TITLE_DEFAULT has been previously set and
# exported by the user. This can be accomplished as follows:
#   export TITLE_DEFAULT="my title"
#   . ~/.bashrc
# Note that sourcing the ~/.bashrc file is done automatically by bash each time you open a new bash
# terminal, so long as it is an interactive (use `bash -i` if calling bash directly) type terminal
if [[ -n "$TITLE_DEFAULT" ]]; then # If length of this is NONzero (see `man test`)
    set-title "$TITLE_DEFAULT"
fi

DEFAULT_TABS_TITLE1="Docker compose"
DEFAULT_TABS_TITLE2="Chain-reader"
DEFAULT_TABS_TITLE3="Indexer manager"
DEFAULT_TABS_TITLE4="Code-compiler"


DEFAULT_TABS_CMD1="docker-compose -f docker-compose.min.yml up"
DEFAULT_TABS_CMD2="cd . && cargo run --bin chain-reader"
DEFAULT_TABS_CMD3="cd . && cargo run --bin index-manager-main" # Use quotes like this if there are spaces in the path
DEFAULT_TABS_CMD4="cd code-compiler/ && python app.py" # Use quotes like this if there are spaces in the path


gnome-terminal --tab -- bash -ic "export TITLE_DEFAULT='$DEFAULT_TABS_TITLE1'; $DEFAULT_TABS_CMD1; exec bash;"
gnome-terminal --tab -- bash -ic "export TITLE_DEFAULT='$DEFAULT_TABS_TITLE2'; $DEFAULT_TABS_CMD2; exec bash;"
gnome-terminal --tab -- bash -ic "export TITLE_DEFAULT='$DEFAULT_TABS_TITLE3'; $DEFAULT_TABS_CMD3; exec bash;"
gnome-terminal --tab -- bash -ic "export TITLE_DEFAULT='$DEFAULT_TABS_TITLE4'; $DEFAULT_TABS_CMD4; exec bash;"

# If length of this is NONzero
if [[ -n "$OPEN_DEFAULT_TABS" ]]; then
    OPEN_DEFAULT_TABS= # reset to an empty string so this only happens ONCE
    open_default_tabs
    exit 0 # close the calling process so only the "default tabs" are left open
fi