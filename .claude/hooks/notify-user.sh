#!/bin/bash
# Notify user when Claude requires input (works on macOS and Linux)

TITLE="Claude Code"
MESSAGE="Claude is waiting for your input"

# Detect OS and send notification
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS - use osascript for notifications
    osascript -e "display notification \"$MESSAGE\" with title \"$TITLE\" sound name \"Glass\"" 2>/dev/null || true
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux - try notify-send (most common), then kdialog, then zenity
    if command -v notify-send &>/dev/null; then
        notify-send "$TITLE" "$MESSAGE" 2>/dev/null || true
    elif command -v kdialog &>/dev/null; then
        kdialog --passivepopup "$MESSAGE" 5 --title "$TITLE" 2>/dev/null || true
    elif command -v zenity &>/dev/null; then
        zenity --notification --text="$TITLE: $MESSAGE" 2>/dev/null || true
    fi
fi

exit 0
