#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/worktree-common.sh"

echo "pre-remove" >> /home/farhad/Desktop/windmill/hello.txt
wm_kill_processes_from_env_file "$(pwd)/.env.local"
wm_shared_pre_remove "$(pwd)"
