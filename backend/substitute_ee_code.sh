#!/usr/bin/env bash
set -euo pipefail
script_dirpath="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dirpath="$(cd "${script_dirpath}/.." && pwd)"

REVERT="NO"
COPY="NO"
MOVE_NEW_FILES="NO"
EE_CODE_DIR="../windmill-ee-private/"
DIR_EXPLICIT="NO"

while [[ $# -gt 0 ]]; do
  case $1 in
    -r|--revert)
      # Removes all _ee.rs files from the backend directory. Symlinks are deleted.
      # Regular files with content differing from windmill-ee-private are moved back
      # to the EE repo so nothing is lost.
      REVERT="YES"
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
    -m|--move-new-files)
      # This moves all new EE files from the public repository to the private repository.
      MOVE_NEW_FILES="YES"
      shift # past argument
      ;;
    -d|--dir)
      # Path to the local directory of the windmill-ee-private repository. By defaults, it
      # assumes it is cloned next to the Windmill OSS repo.
      EE_CODE_DIR="$2"
      DIR_EXPLICIT="YES"
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

# Fallback to ~/windmill-ee-private if the default location doesn't exist
if [ ! -d "${EE_CODE_DIR}" ]; then
  EE_CODE_DIR="${HOME}/windmill-ee-private"
fi

# Unless --dir was explicitly set, try to find an EE worktree on the same branch
if [ "$DIR_EXPLICIT" == "NO" ] && [ -d "${HOME}/windmill-ee-private" ]; then
  current_branch=$(git -C "${root_dirpath}" branch --show-current 2>/dev/null || true)
  if [ -n "$current_branch" ]; then
    ee_worktree=$(git -C "${HOME}/windmill-ee-private" worktree list 2>/dev/null \
      | awk -v branch="[${current_branch}]" '$NF == branch {print $1; exit}')
    if [ -n "$ee_worktree" ] && [ -d "$ee_worktree" ]; then
      EE_CODE_DIR="$ee_worktree"
    fi
  fi
fi

echo "EE code directory = ${EE_CODE_DIR} | Revert = ${REVERT}"

if [ ! -d "${EE_CODE_DIR}" ]; then
  echo "Windmill EE repo not found, please clone it next to this repository (or use the --dir option) and try again"
  echo ">   git clone git@github.com:windmill-labs/windmill-ee-private.git"
  echo ""
  exit 0
fi

if [ "$REVERT" == "YES" ]; then
  backend_dirpath="${root_dirpath}/backend/"
  for ce_file in $(find "${root_dirpath}/backend" -name "*_ee.rs"); do
    if [ -L "${ce_file}" ]; then
      rm "${ce_file}"
      echo "Deleted symlink '${ce_file}'"
    else
      ee_file="${ce_file/${backend_dirpath}/}"
      ee_file="${EE_CODE_DIR}${ee_file}"
      if [ ! -f "${ee_file}" ] || ! diff -q "${ce_file}" "${ee_file}" > /dev/null 2>&1; then
        mkdir -p "$(dirname "${ee_file}")"
        mv "${ce_file}" "${ee_file}"
        echo "Moved '${ce_file}' -->> '${ee_file}'"
      else
        rm "${ce_file}"
        echo "Deleted '${ce_file}' (identical to EE)"
      fi
    fi
  done
  exit 0
fi

if [ "$MOVE_NEW_FILES" == "NO" ]; then
  # This replaces all files in current repo with alternative EE files in windmill-ee-private
  for ee_file in $(find "${EE_CODE_DIR}" -name "*ee.rs"); do
  ce_file="${ee_file/${EE_CODE_DIR}/}"
  ce_file="${root_dirpath}/backend/${ce_file}"
    if [ "$COPY" == "YES" ]; then
      cp "${ee_file}" "${ce_file}"
      echo "File copied '${ee_file}' -->> '${ce_file}'"
    else
      ln -s "${ee_file}" "${ce_file}" || true
      echo "Symlink created '${ee_file}' -->> '${ce_file}'"
    fi
  done
fi

if [ "$MOVE_NEW_FILES" == "YES" ]; then
  for ce_file in $(find "${root_dirpath}"/backend/windmill-*/src/ -name "*ee.rs"); do
    backend_dirpath="${root_dirpath}/backend/"
    ee_file="${ce_file/${backend_dirpath}/}"
    ee_file="${EE_CODE_DIR}${ee_file}"
    if [ ! -f "${ee_file}" ]; then
      mv "${ce_file}" "${ee_file}"
      if [ ! "$REVERT" == "YES" ]; then
        ln -s "${ee_file}" "${ce_file}"
      fi
      echo "File moved '${ce_file}' -->> '${ee_file}'"
    fi
  done
fi
