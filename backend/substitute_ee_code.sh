#!/bin/bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

REVERT="NO"
REVERT_PREVIOUS="NO"
COPY="NO"
EE_CODE_DIR="../windmill-ee-private/"

while [[ $# -gt 0 ]]; do
  case $1 in
    -r|--revert)
      # If EE files have been substituted, this will revert them to their initial content. 
      # This relies on `git restore` so the EE files must not be committed to the repo for 
      # this to work (commit hooks should prevent this from happening, as well as the fact
      # that we're using symlinks by default).
      REVERT="YES"
      shift
      ;;
    --revert-previous)
      # This is a special case of --revert that will revert to the previous commit.
      REVERT="YES"
      REVERT_PREVIOUS="YES"
      echo "Reverting to previous commit"
      shift
      ;;
    -c|--copy)
      # By default, EE files are symlinked. Pass this option to do a real copy instead.
      # This might be necessary if you want to build the Docker Image as Docker COPY seems
      # to not follow symlinks. For local development, symlinks should be preferred as they
      # adds a safeguards EE files can't be commited to the OSS repo.
      COPY="YES"
      shift # past argument
      ;;
    -d|--dir)
      # Path to the local directory of the windmill-ee-private repository. By defaults, it
      # assumes it is cloned next to the Windmill OSS repo.
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
echo "EE code directory = ${EE_CODE_DIR} | Revert = ${REVERT}"


if [ ! -d "${EE_CODE_DIR}" ]; then
  echo "Windmill EE repo not found, please clone it next to this repository (or use the --dir option) and try again"
  echo ">   git clone git@github.com:windmill-labs/windmill-ee-private.git"
  echo ""
  exit 0
fi

if [ "$REVERT" == "YES" ]; then
  for ee_file in $(find ${EE_CODE_DIR} -name "*ee.rs"); do
    ce_file="${ee_file/${EE_CODE_DIR}/}"
    ce_file="${root_dirpath}/backend/${ce_file}"
    if [ "$REVERT_PREVIOUS" == "YES" ]; then
      git checkout HEAD@{2} ${ce_file} || true
    else
      git restore --staged ${ce_file} || true
      git restore ${ce_file} || true
    fi
  done
else
  # This replaces all files in current repo with alternative EE files in windmill-ee-private
  for ee_file in $(find "${EE_CODE_DIR}" -name "*ee.rs"); do
    ce_file="${ee_file/${EE_CODE_DIR}/}"
    ce_file="${root_dirpath}/backend/${ce_file}"
    if [[ -f "${ce_file}" ]]; then
      rm "${ce_file}"
      if [ "$COPY" == "YES" ]; then
        cp "${ee_file}" "${ce_file}"
        echo "File copied '${ee_file}' -->> '${ce_file}'"
      else
        ln -s "${ee_file}" "${ce_file}"
        echo "Symlink created '${ee_file}' -->> '${ce_file}'"
      fi
    else
      echo "File ${ce_file} is not a file, ignoring"
    fi
  done
fi
