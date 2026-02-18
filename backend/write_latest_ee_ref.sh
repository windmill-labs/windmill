#!/bin/bash

# Navigate to the target directory
cd ../../windmill-ee-private || cd ~/windmill-ee-private || { echo "Directory not found"; exit 1; }

# Get the current commit hash
commit_hash=$(git rev-parse HEAD)

# Navigate back to the original directory
cd - || exit

# Write the commit hash to ./ee-repo-ref.txt
echo -n "$commit_hash" > ./ee-repo-ref.txt

# Confirmation message
echo "Commit hash $commit_hash written to ee-repo-ref.txt"
