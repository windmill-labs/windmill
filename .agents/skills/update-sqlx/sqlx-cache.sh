#!/usr/bin/env bash
# Backup / inspect / restore the SQLx offline cache around `cargo sqlx prepare`.
#
# `prepare` empties backend/.sqlx before regenerating, so any compile failure leaves the
# cache gutted (observed: 2350 -> 142 entries). A `--all-targets` run in a CE checkout
# aborts that way every time. State lives in a per-worktree directory, so sibling
# worktrees running this concurrently cannot overwrite each other's backup.
#
#   sqlx-cache.sh backup    snapshot backend/.sqlx
#   sqlx-cache.sh newq      show the entries prepare added since the snapshot, and stage them
#   sqlx-cache.sh restore   put the snapshot back, grafting the staged entries on top
#
# Inspect what `newq` prints before running `restore` — an entry you don't recognise means
# the run got further than you think.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cache="$repo_root/backend/.sqlx"
state="${TMPDIR:-/tmp}/wm-sqlx-cache/$(basename "$repo_root")"
backup="$state/backup"
added="$state/added"

# `find -printf` is GNU-only; a glob loop stays portable to a macOS checkout and, unlike
# `ls *.json`, does not fail the script under `set -e` when the cache is empty — which is
# exactly the state a failed `prepare` leaves behind.
list_entries() {
  local f
  for f in "$1"/*.json; do
    [ -e "$f" ] || continue
    basename "$f"
  done | sort
}

show_query() {
  if command -v jq >/dev/null 2>&1; then
    jq -r '.query' "$1" 2>/dev/null | head -6
  else
    sed -n 's/^ *"query": "\(.*\)",*$/\1/p' "$1" | head -6
  fi
}

case "${1:-}" in
backup)
  [[ -d $cache ]] || { echo "no cache at $cache" >&2; exit 1; }
  rm -rf "$state"
  mkdir -p "$state"
  cp -r "$cache" "$backup"
  list_entries "$backup" > "$state/before.txt"
  echo "backed up $(wc -l < "$state/before.txt" | tr -d ' ') entries to $backup"
  ;;

newq)
  [[ -d $backup ]] || { echo "no backup — run '$0 backup' first" >&2; exit 1; }
  list_entries "$cache" > "$state/after.txt"
  comm -13 "$state/before.txt" "$state/after.txt" > "$state/new.txt"
  rm -rf "$added"
  mkdir -p "$added"
  n=0
  while read -r f; do
    [[ -n $f ]] || continue
    cp "$cache/$f" "$added/$f"
    n=$((n + 1))
    echo "--- $f"
    show_query "$cache/$f"
  done < "$state/new.txt"
  echo "$n entries added since the backup, staged in $added"
  ;;

restore)
  [[ -d $backup ]] || { echo "no backup — nothing to restore" >&2; exit 1; }
  [[ -d $added ]] || { echo "run '$0 newq' first so the added entries are staged" >&2; exit 1; }
  rm -rf "$cache"
  cp -r "$backup" "$cache"
  n=0
  for f in "$added"/*.json; do
    [[ -e $f ]] || continue
    cp "$f" "$cache/"
    n=$((n + 1))
  done
  echo "restored $(list_entries "$cache" | wc -l | tr -d ' ') entries ($n grafted from this run)"
  ;;

*)
  sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
  ;;
esac
