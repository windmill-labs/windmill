#!/bin/bash

# Define the file to modify
FILE="src/lib/components/apps/editor/AppEditorHeader.svelte"

# Function to toggle commenting of a line
toggle_line_comment() {
    local line="$1"
    if grep -q "^//$line" "$FILE"; then
        # Uncomment the line
        sed -i "s|^//$line|$line|" "$FILE"
    else
        # Comment the line
        sed -i "s|^$line|//$line|" "$FILE"
    fi
}

# Function to toggle commenting of the first matching block
toggle_first_block_comment() {
    local start_marker="$1"
    local end_marker="$2"

    # Escape the end marker for use in sed
    local escaped_end_marker=$(echo "$end_marker" | sed 's|/|\\/|g')

    # Check if the block is already commented
    if sed -n "/$start_marker/,/$escaped_end_marker/p" "$FILE" | grep -q "^<!--"; then
        # Uncomment the block
        sed -i "/$start_marker/,/$escaped_end_marker/{
            s|^<!-- ||
            s| -->$||
        }" "$FILE"
    else
        # Comment the block
        sed -i "/$start_marker/,/$escaped_end_marker/{
            s|^|<!-- |
            s|$| -->|
        }" "$FILE"
    fi
}

# Define the line and block
LINE="	import UnsavedConfirmationModal "
BLOCK_START="<UnsavedConfirmationModal"
BLOCK_END="/>"

# Toggle the line comment
toggle_line_comment "$LINE"

# Toggle the first block comment
toggle_first_block_comment "$BLOCK_START" "$BLOCK_END"

echo "Toggled line and first block comments in $FILE."
