# Rewrites a pre-2.11 Windmill Caddyfile to the form caddy >= 2.9 with native
# caddy-l4 requires. Two rewrites, both no-ops on a current Caddyfile so this is
# idempotent:
#
#   layer4 { :25 { proxy { to X } } }  ->  layer4 { :25 { route { proxy { upstream X } } } }
#   bind {$ADDRESS}                    ->  bind {$ADDRESS:0.0.0.0 ::}
#
# The second is not cosmetic: an empty bind makes caddy >= 2.9 drop the whole
# HTTP site, so without it a legacy Caddyfile boots clean and serves nothing on
# :80 while :25 keeps working.
#
# Only the layer4 block and that one literal bind token are touched; a proxy
# already nested in a route block is left alone.

BEGIN { depth = 0; inL4 = 0; l4depth = 0; inRoute = 0; rdepth = 0; inProxy = 0; pdepth = 0 }
{
  line = $0

  if (line ~ /^[[:space:]]*bind[[:space:]]+\{\$ADDRESS\}[[:space:]]*$/) {
    sub(/\{\$ADDRESS\}/, "{$ADDRESS:0.0.0.0 ::}", line)
    print line
    next
  }

  if (!inL4 && line ~ /^[[:space:]]*layer4[[:space:]]*\{/) { inL4 = 1; l4depth = depth; print; depth++; next }
  if (inL4 && line ~ /^[[:space:]]*route[[:space:]]*\{/) { inRoute = 1; rdepth = depth; print; depth++; next }

  if (inL4 && !inRoute && !inProxy && line ~ /^[[:space:]]*proxy[[:space:]]*\{[[:space:]]*$/) {
    match(line, /^[[:space:]]*/); ind = substr(line, 1, RLENGTH)
    print ind "route {"
    print ind "\tproxy {"
    inProxy = 1; pdepth = depth; depth++
    next
  }

  # `to a b` is two load-balanced upstreams, whereas `upstream a b` is a single
  # upstream with two dial addresses. Emit one `upstream` per address to keep
  # the adapted JSON identical.
  if (inProxy && line ~ /^[[:space:]]*to[[:space:]]+/) {
    match(line, /^[[:space:]]*/); ind = substr(line, 1, RLENGTH)
    nf = split(line, parts, /[[:space:]]+/)
    for (i = 1; i <= nf; i++) if (parts[i] == "to") break
    for (j = i + 1; j <= nf; j++) if (parts[j] != "") print "\t" ind "upstream " parts[j]
    next
  }

  n = gsub(/\{/, "{", line); m = gsub(/\}/, "}", line); depth += n - m

  if (inProxy && depth == pdepth) {
    match(line, /^[[:space:]]*/); ind = substr(line, 1, RLENGTH)
    print "\t" line
    print ind "}"
    inProxy = 0
    next
  }

  if (inRoute && depth <= rdepth) inRoute = 0
  if (inL4 && depth <= l4depth) inL4 = 0

  if (inProxy) print "\t" line; else print line
}
