#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

EE_CODE_DIR="../windmill-ee-private/"

while [[ $# -gt 0 ]]; do
  case $1 in
    -d|--dir)
      EE_CODE_DIR="$2"
      shift # past argument
      shift # past value
      ;;
    -*|--*)
      echo "Unknown option $1"
      exit 1
      ;;
    *)
      POSITIONAL_ARGS+=("$1") # save positional arg
      shift # past argument
      ;;
  esac
done

if [[ $EE_CODE_DIR == /* ]]; then
  EE_CODE_DIR="${EE_CODE_DIR}"
else
  EE_CODE_DIR="${root_dirpath}/${EE_CODE_DIR}"
fi
echo "EE code directory = ${EE_CODE_DIR}"

if [ ! -d "${EE_CODE_DIR}" ]; then
  echo "Windmill EE repo not found, nothing to do"
  exit 0
fi

for ee_file in $(find "${EE_CODE_DIR}" -name "*.rs"); do
  ce_file="${ee_file/${EE_CODE_DIR}/.}"
  ce_file="${root_dirpath}/backend/${ce_file}"
  echo "Checking if '${ce_file}' is a symlink"
  if [[ -L "${ce_file}" ]]; then
      echo "File ${ce_file} is a symlink, cannot commit symlinks"
      exit 1
  fi
done
echo "All good!"
