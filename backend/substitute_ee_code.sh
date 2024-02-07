#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

opt=${1:-}

if [ -d "${root_dirpath}/../windmill-ee-private/" ]; then
  echo "Windmill EE repo not found, please clone it next to this repository and try again"
  echo ">   git clone git@github.com:windmill-labs/windmill-ee-private.git"
  echo ""
  exit 0
fi

if [ "$opt" == "revert" ]; then
  for ee_file in $(find "${root_dirpath}/../windmill-ee-private/" -name "*.rs"); do
    ce_file="${ee_file/${root_dirpath}\/..\/windmill-ee-private\//.}"
    git restore --staged ${ce_file} || true
    git restore ${ce_file} || true
  done
else
  # This replaces all files in current repo with alternative EE files in windmill-ee-private
  for ee_file in $(find "${root_dirpath}/../windmill-ee-private/" -name "*.rs"); do
    ce_file="${ee_file/${root_dirpath}\/..\/windmill-ee-private\//.}"
    if [[ -f "${ce_file}" ]]; then
      rm "${ce_file}"
      ln -s "${ee_file}" "${ce_file}"
      echo "Symlink created '${ee_file}' -->> '${ce_file}'"
    else
      echo "File ${ce_file} is not a file, ignoring"
    fi
  done
fi
