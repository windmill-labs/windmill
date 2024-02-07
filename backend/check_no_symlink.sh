#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

if [ -d "${root_dirpath}/../windmill-ee-private/" ]; then
  echo "Windmill EE repo not found, nothing to do"
  exit 0
fi

for ee_file in $(find "${root_dirpath}/../windmill-ee-private/" -name "*.rs"); do
  ce_file="${ee_file/${root_dirpath}\/..\/windmill-ee-private\//.}"
  echo "Checking if '${ce_file}' is a symlink"
  if [[ -L "${ce_file}" ]]; then
      echo "File ${ce_file} is a symlink, cannot commit symlinks"
      exit 1
  fi
done
echo "All good!"
