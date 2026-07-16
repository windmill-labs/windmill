#!/bin/sh
# The Caddyfile is a bind-mounted file the user owns, so `docker compose pull`
# updates this image but never their config. Caddy >= 2.9 with native caddy-l4
# rejects the syntax we shipped before 2026-07, which would strand every
# existing self-host on a pull. Normalize legacy Caddyfiles here instead.
#
# Rewrite only when the config cannot be used as-is, and never fail closed: on
# any failure, exec caddy against the user's original file so it reports a real
# error against what they wrote. A shim that exits, or that claims a rewrite it
# did not apply, is worse than no shim at all.
set -eu

NORMALIZE_AWK=/usr/local/lib/caddy-compat/normalize.awk
NORMALIZED_DIR=/tmp/caddy-compat
DOCS_HINT="https://github.com/windmill-labs/windmill/blob/main/Caddyfile"

log() { echo "caddy-compat: $*" >&2; }

# The image had no ENTRYPOINT and CMD ["caddy", "run", ...], so an existing
# `command:` override in a user's compose file starts with `caddy`. Accept it
# with or without that leading token.
if [ "${1:-}" = "caddy" ]; then
	shift
fi

# caddy accepts --config X, --config=X, -c X, -c=X and -cX. Record which form
# was used and at which position, so the rewritten path can be substituted back
# in the same form rather than by matching on the string.
config=""
config_idx=0
config_form=""
envfile=""
prev=""
i=0
for arg in "$@"; do
	i=$((i + 1))
	case "$arg" in
	--config=*) config="${arg#--config=}"; config_idx=$i; config_form="eq-long" ;;
	-c=*) config="${arg#-c=}"; config_idx=$i; config_form="eq-short" ;;
	-c?*) config="${arg#-c}"; config_idx=$i; config_form="attached" ;;
	esac
	case "$prev" in
	--config | -c) config="$arg"; config_idx=$i; config_form="space" ;;
	--envfile) envfile="$arg" ;;
	esac
	case "$arg" in
	--envfile=*) envfile="${arg#--envfile=}" ;;
	esac
	prev="$arg"
done

# With no explicit --config, caddy resolves its own default relative to the
# working directory. Guessing at that path risks inspecting a different file
# than the one caddy will load, so leave it alone.
if [ "$config_idx" -eq 0 ] || [ ! -r "$config" ]; then
	exec caddy "$@"
fi

# The shim's own validate has to see the same env expansion caddy will.
validate_config() {
	if [ -n "$envfile" ]; then
		caddy validate --config "$1" --adapter caddyfile --envfile "$envfile" 2>&1
	else
		caddy validate --config "$1" --adapter caddyfile 2>&1
	fi
}

original_err=""
normalized=""

dump_failure() {
	log "---------------- caddy-l4 compatibility shim ----------------"
	log "Your Caddyfile could not be used as-is, and the compatibility"
	log "rewrite did not produce a valid config either. Caddy will now"
	log "run against your original file and report its own error below."
	log ""
	log "Config:     $config"
	log "Normalized: ${normalized:-<not produced>}"
	log "Reference:  $DOCS_HINT"
	if [ -n "$original_err" ]; then
		log ""
		log "Error validating your Caddyfile as written:"
		echo "$original_err" | sed 's/^/caddy-compat:   /' >&2
	fi
	if [ -n "${1:-}" ]; then
		log ""
		log "Error from the compatibility rewrite:"
		echo "$1" | sed 's/^/caddy-compat:   /' >&2
	fi
	log ""
	log "Most likely fix: replace your Caddyfile with the current one from"
	log "the Windmill repo, then re-apply your customizations on top."
	log "-------------------------------------------------------------"
}

# A bare `bind {$ADDRESS}` adapts and validates cleanly, it just silently drops
# the HTTP site at runtime, so validate alone cannot decide this.
needs_rewrite=0
if ! original_err=$(validate_config "$config"); then
	needs_rewrite=1
	log "'$config' is not valid for this caddy version, attempting compatibility rewrite"
else
	original_err=""
fi
if grep -Eq '^[[:space:]]*bind[[:space:]]+\{\$ADDRESS\}[[:space:]]*$' "$config"; then
	needs_rewrite=1
	log "'$config' has a bare 'bind {\$ADDRESS}', which caddy >= 2.9 treats as an"
	log "empty bind and drops the entire HTTP site; attempting compatibility rewrite"
fi

if [ "$needs_rewrite" -eq 0 ]; then
	exec caddy "$@"
fi

# caddy resolves `import` relative to the directory of the file containing it,
# so the rewritten file has to stay next to the original. Falling back to a
# different directory would make an exact import fail and, worse, make a glob
# import silently match nothing and drop whatever it pulled in.
config_dir=$(dirname "$config")
sibling="$config_dir/.caddy-compat.Caddyfile"
# `touch`, not `: >`: a redirection failure on a special builtin is fatal to the
# shell even inside an `if`, which would kill the shim instead of falling
# through to caddy.
if touch "$sibling" 2>/dev/null; then
	normalized="$sibling"
elif grep -Eq '^[[:space:]]*import[[:space:]]' "$config"; then
	log "'$config_dir' is not writable and '$config' uses 'import', whose relative"
	log "paths would not resolve from anywhere else. Refusing to rewrite rather than"
	log "silently drop imported config."
	dump_failure ""
	exec caddy "$@"
elif mkdir -p "$NORMALIZED_DIR" 2>/dev/null; then
	normalized="$NORMALIZED_DIR/Caddyfile"
else
	log "nowhere writable to put the rewritten config, skipping the rewrite"
	dump_failure ""
	exec caddy "$@"
fi

if ! awk_err=$(awk -f "$NORMALIZE_AWK" "$config" 2>&1 >"$normalized"); then
	dump_failure "$awk_err"
	exec caddy "$@"
fi

if ! normalized_err=$(validate_config "$normalized"); then
	dump_failure "$normalized_err"
	exec caddy "$@"
fi

log "'$config' uses a deprecated Windmill Caddyfile syntax and was adapted at"
log "startup. Copy the current Caddyfile from $DOCS_HINT"
log "to silence this; the shim will be removed in a later release."
log "Rewrites applied:"
diff -u "$config" "$normalized" 2>/dev/null | sed 's/^/caddy-compat:   /' >&2 || true

# caddy now reads the rewritten file, so `--watch` follows it rather than the
# bind-mounted original, and `caddy reload --config /etc/caddy/Caddyfile` would
# reload the un-normalized file. Both are non-default; a legacy config that
# needs this shim should be replaced rather than live-edited.
i=0
for arg in "$@"; do
	i=$((i + 1))
	if [ "$i" -eq "$config_idx" ]; then
		case "$config_form" in
		space) set -- "$@" "$normalized" ;;
		eq-long) set -- "$@" "--config=$normalized" ;;
		eq-short) set -- "$@" "-c=$normalized" ;;
		attached) set -- "$@" "-c$normalized" ;;
		esac
	else
		set -- "$@" "$arg"
	fi
	shift
done

exec caddy "$@"
