# `policy.on_behalf_of_email`

An app's identity is `policy.on_behalf_of`; the address beside it is a function of that
principal. The policy stores only the principal. The address is derived wherever it is needed:
at execution (`get_on_behalf_of`, `$ctx:author`) and on the reads that return a deployed policy
(`attach_on_behalf_of_email`), where it is a response-only field the write path ignores.

## The gate

**This cannot ship until `MIN_KEEP_ALIVE_VERSION` (`windmill-common/src/min_version.rs`) has
passed 1.810.**

`get_on_behalf_of` gained its derive-when-absent fallback in **1.810**. Every replica before that
*requires* the key and errors outright without it, so a rolling deploy that runs both would 400
every anonymous, publisher and guest app the moment one of them saves. The write is what holds
the key in place, so stopping it is only safe once no replica older than 1.810 can be live.

`MIN_KEEP_ALIVE_VERSION` is 1.420.0, and a compile-time rule keeps it at least 50 minor versions
behind current, so it cannot reach 1.810 until releases pass ~1.860.

There is no `MIN_VERSION_*` constant for this and it does not need one: those gate behavior at
runtime or trip the build when a constraint expires, and nothing here does either.

## Why the address is not derived on a draft read

A deployed policy's principal was written by the server, from an identity `resolve_on_behalf_of`
validated. A draft's is whatever its editor last typed. Deriving one would answer "which address
is `u/x`?" for any principal a caller cares to name — including a superadmin outside their own
workspaces, whom `resolve_username_to_email` reaches instance-wide. That was a live defect while
every read derived, and is why the derivation is attached per read path rather than to `Policy`
itself.
