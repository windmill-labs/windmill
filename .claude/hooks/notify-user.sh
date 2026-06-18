#!/bin/bash
# Notify user when Claude requires input (works on macOS and Linux)

# Check if we're in an SSH session
if [[ -n "$SSH_CLIENT" || -n "$SSH_TTY" || -n "$SSH_CONNECTION" ]]; then
    # SSH session - use terminal bell
    # If using VSCode, enable audible terminal bell for SSH sessions:
    # Add the following to .vscode/settings.json:
    #   "accessibility.signals.terminalBell": {
    #     "sound": "on"
    #   },
    #   "terminal.integrated.enableVisualBell": true
    printf '\a'
else
    # Local session - use native notifications
    if [[ "$OSTYPE" == "darwin"* ]]; then
        osascript -e 'display notification "Claude is waiting for your input" with title "Claude Code" sound name "Glass"' 2>/dev/null || printf '\a'
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        notify-send "Claude Code" "Claude is waiting for your input" 2>/dev/null || printf '\a'
    else
        printf '\a'
    fi
fi

exit 0
