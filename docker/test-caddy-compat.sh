#!/usr/bin/env bash
# Regression tests for the caddy-l4 legacy-Caddyfile compatibility shim.
#
# The reference for "correct" is the image published before #10106
# (REFERENCE_IMAGE): whatever config it adapts today is what existing
# self-hosters are running, so the shim must reproduce it byte for byte.
#
# Usage: docker/test-caddy-compat.sh [image-tag]
set -euo pipefail

IMAGE="${1:-caddy-l4:test}"
REFERENCE_IMAGE="${REFERENCE_IMAGE:-ghcr.io/windmill-labs/caddy-l4:sha-989c9e6}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass=0
fail=0
ok() { printf '  \033[32mok\033[0m   %s\n' "$1"; pass=$((pass + 1)); }
no() { printf '  \033[31mFAIL\033[0m %s\n' "$1"; fail=$((fail + 1)); }

# SELinux-labelled hosts (Fedora) reject plain bind mounts into containers.
MOUNT_OPTS=()
if command -v getenforce >/dev/null 2>&1 && [ "$(getenforce 2>/dev/null)" != "Disabled" ]; then
	MOUNT_OPTS=(--security-opt label=disable)
fi

adapt() { # adapt <image> <caddyfile> [env...]
	local image="$1" file="$2"
	shift 2
	local env_args=()
	local e
	for e in "$@"; do env_args+=(-e "$e"); done
	# The `caddy` prefix is how the pre-#10106 image is invoked (no ENTRYPOINT,
	# CMD ["caddy", ...]); the shim accepts it too, which this exercises.
	docker run --rm "${MOUNT_OPTS[@]}" "${env_args[@]}" \
		-v "$file:/etc/caddy/Caddyfile:ro" "$image" \
		caddy adapt --config /etc/caddy/Caddyfile 2>/dev/null
}

# The legacy Caddyfile as shipped before #10106, which is what a self-hoster who
# has never touched their config still has on disk.
cat >"$WORK/legacy.Caddyfile" <<'EOF'
{
	layer4 {
		:25 {
			proxy {
				to windmill_server:2525
			}
		}
	}
}

{$BASE_URL} {
        bind {$ADDRESS}
        reverse_proxy /ws/* /ws_mp/* /ws_debug/* http://windmill_extra:3000
        reverse_proxy /* http://windmill_server:8000
}
EOF

echo "==> equivalence with the pre-#10106 image (ADDRESS set, so bind is neutral)"
docker pull -q "$REFERENCE_IMAGE" >/dev/null 2>&1 || true
if ! docker image inspect "$REFERENCE_IMAGE" >/dev/null 2>&1; then
	echo "  skipped: $REFERENCE_IMAGE unavailable"
else
	# Each variant is a plausible user modification of the legacy Caddyfile.
	make_variant() {
		case "$1" in
		stock) cat "$WORK/legacy.Caddyfile" ;;
		custom_smtp) sed 's|to windmill_server:2525|to mailhog:2626|' "$WORK/legacy.Caddyfile" ;;
		multi_upstream) sed 's|to windmill_server:2525|to a:2525 b:2525|' "$WORK/legacy.Caddyfile" ;;
		extra_listener) sed 's|\t\t:25 {|\t\t:587 {\n\t\t\tproxy {\n\t\t\t\tto windmill_server:2525\n\t\t\t}\n\t\t}\n\t\t:25 {|' "$WORK/legacy.Caddyfile" ;;
		extra_directive) sed 's|reverse_proxy /\*|encode gzip\n\treverse_proxy /*|' "$WORK/legacy.Caddyfile" ;;
		esac
	}
	for name in stock custom_smtp multi_upstream extra_listener extra_directive; do
		make_variant "$name" >"$WORK/v.Caddyfile"
		a="$(adapt "$REFERENCE_IMAGE" "$WORK/v.Caddyfile" 'BASE_URL=:80' 'ADDRESS=0.0.0.0')"
		b="$(adapt "$IMAGE" "$WORK/v.Caddyfile" 'BASE_URL=:80' 'ADDRESS=0.0.0.0')"
		if [ -n "$a" ] && [ "$a" = "$b" ]; then ok "$name: adapted config identical"; else no "$name: adapted config differs"; fi
	done
fi

echo "==> the HTTP site must survive an unset ADDRESS (#10113: validate cannot catch this)"
listen="$(adapt "$IMAGE" "$WORK/legacy.Caddyfile" 'BASE_URL=:80' | grep -o '"listen":\[[^]]*\]' | head -1)"
case "$listen" in
*:80*) ok "legacy Caddyfile still binds :80 (${listen})" ;;
*) no "legacy Caddyfile dropped the HTTP site (listen=${listen:-none})" ;;
esac

echo "==> the shipped Caddyfile must be untouched by the shim"
noise="$(docker run --rm "${MOUNT_OPTS[@]}" -e BASE_URL=':80' \
	-v "$REPO_ROOT/Caddyfile:/etc/caddy/Caddyfile:ro" "$IMAGE" \
	adapt --config /etc/caddy/Caddyfile 2>&1 >/dev/null | grep -c 'caddy-compat' || true)"
[ "$noise" -eq 0 ] && ok "no rewrite on the current Caddyfile" || no "shim fired on the current Caddyfile"

echo "==> every --config form must load the rewritten file"
check_form() {
	local label="$1"
	shift
	local out
	out="$(docker run --rm "${MOUNT_OPTS[@]}" -e BASE_URL=':80' \
		-v "$WORK/legacy.Caddyfile:/etc/caddy/Caddyfile:ro" "$IMAGE" "$@" 2>&1 >/dev/null |
		grep -E '"file":' | head -1 || true)"
	case "$out" in
	*caddy-compat*) ok "$label" ;;
	*) no "$label (loaded: ${out:-nothing})" ;;
	esac
}
check_form '--config PATH' adapt --config /etc/caddy/Caddyfile
check_form '--config=PATH' adapt --config=/etc/caddy/Caddyfile
check_form '-c PATH' adapt -c /etc/caddy/Caddyfile
check_form '-c=PATH' adapt -c=/etc/caddy/Caddyfile
check_form '-cPATH' adapt -c/etc/caddy/Caddyfile

echo "==> relative imports must survive the rewrite"
# caddy resolves `import` against the directory of the file holding it, so a
# rewrite that relocates the config would break an exact import and make a glob
# import silently expand to nothing.
mkdir -p "$WORK/withimport"
sed 's|reverse_proxy /\* http://windmill_server:8000|import extra.caddy\n\treverse_proxy /* http://windmill_server:8000|' \
	"$WORK/legacy.Caddyfile" >"$WORK/withimport/Caddyfile"
echo 'reverse_proxy /imported/* http://imported_backend:9999' >"$WORK/withimport/extra.caddy"
printf 'reverse_proxy /globbed/* http://globbed_backend:9998\n' >"$WORK/withimport/glob-one.caddy"
sed -i 's|import extra.caddy|import extra.caddy\n\timport glob-*.caddy|' "$WORK/withimport/Caddyfile"
imported="$(docker run --rm "${MOUNT_OPTS[@]}" -e BASE_URL=':80' \
	-v "$WORK/withimport:/etc/caddy" "$IMAGE" \
	adapt --config /etc/caddy/Caddyfile 2>/dev/null)"
case "$imported" in
*imported_backend:9999*) ok "exact import survives" ;;
*) no "exact import lost after rewrite" ;;
esac
case "$imported" in
*globbed_backend:9998*) ok "glob import survives" ;;
*) no "glob import silently dropped after rewrite" ;;
esac

echo "==> a config the shim cannot fix must still reach caddy's own error"
broken="$(docker run --rm "${MOUNT_OPTS[@]}" --read-only -e BASE_URL=':80' \
	-v "$WORK/legacy.Caddyfile:/etc/caddy/Caddyfile:ro" "$IMAGE" \
	run --config /etc/caddy/Caddyfile --adapter caddyfile 2>&1 | grep -c '^Error:' || true)"
[ "$broken" -ge 1 ] && ok "read-only rootfs still execs caddy and reports its error" || no "read-only rootfs swallowed caddy's error"

echo
echo "passed: $pass   failed: $fail"
[ "$fail" -eq 0 ]
