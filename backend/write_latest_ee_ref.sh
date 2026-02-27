#!/bin/bash

# Detect the current branch name
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

# Try, in order: matching EE worktree, sibling repo, home directory fallback
ee_worktree="$HOME/windmill-ee-private__worktrees/$branch"
if [ -n "$branch" ] && [ -d "$ee_worktree" ]; then
    cd "$ee_worktree"
elif cd ../../windmill-ee-private 2>/dev/null; then
    :
elif cd ~/windmill-ee-private 2>/dev/null; then
    :
else
    echo "Directory not found"; exit 1
fi

# Get the current commit hash
commit_hash=$(git rev-parse HEAD)

# Navigate back to the original directory
cd - || exit

# Write the commit hash to ./ee-repo-ref.txt
echo -n "$commit_hash" > ./ee-repo-ref.txt

# Confirmation message
echo "Commit hash $commit_hash written to ee-repo-ref.txt"
