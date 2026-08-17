#!/usr/bin/env bash
# Refresh the vendored documentation snapshot embedded into windmill-api.
#
# The backend self-hosts the docs corpus (see ../src/docs/) so docs search works
# with no runtime egress. This snapshot is pinned to whatever was published on
# windmill.dev when this script was last run — re-run it on each release to keep
# the in-product docs search reasonably fresh, then commit the updated *.gz.
set -euo pipefail

DOCS_ORIGIN="${DOCS_ORIGIN:-https://www.windmill.dev}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

fetch() {
  local name="$1"
  echo "Fetching ${DOCS_ORIGIN}/${name} ..."
  curl -fSL "${DOCS_ORIGIN}/${name}" -o "${DIR}/${name}"
  # -9 max compression; -n omit the original name/timestamp so the artifact is
  # reproducible and diffs only when the docs actually change.
  gzip -9 -n -c "${DIR}/${name}" > "${DIR}/${name}.gz"
  rm -f "${DIR}/${name}"
  echo "  wrote ${name}.gz ($(wc -c < "${DIR}/${name}.gz") bytes)"
}

fetch "llms.txt"
fetch "llms-full.txt"
echo "Done. Commit the updated *.gz files."
