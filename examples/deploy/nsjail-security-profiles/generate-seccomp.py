#!/usr/bin/env python3
"""Regenerate windmill-nsjail.seccomp.json from Docker's default seccomp profile.

usage: generate-seccomp.py <moby default.json> <output.json>

Docker's default.json carries `includes`/`excludes` conditions that only Docker
evaluates; a Kubernetes Localhost profile is handed to the runtime as is, where
those conditions are ignored and every rule applies. The output is therefore the
profile as Docker resolves it for a container with no added capability, with the
conditions removed, plus the syscalls nsjail needs.
"""
import json
import sys

# clone is allowed with namespace flags; clone3 keeps returning ENOSYS, which
# makes nsjail fall back to clone.
NSJAIL_SYSCALLS = ["clone", "mount", "pivot_root", "sethostname", "umount2"]
ARCHES = {"amd64", "x32", "x86", "arm", "arm64"}

src, out = sys.argv[1], sys.argv[2]
profile = json.load(open(src))
rules = []
for rule in profile["syscalls"]:
    includes = rule.get("includes", {})
    if includes.get("caps"):
        continue
    if includes.get("arches") and not ARCHES & set(includes["arches"]):
        continue
    rule = {k: v for k, v in rule.items() if k not in ("includes", "excludes", "comment")}
    rule["names"] = [n for n in rule["names"] if n not in NSJAIL_SYSCALLS]
    if rule["names"]:
        rules.append(rule)
rules.append({"names": NSJAIL_SYSCALLS, "action": "SCMP_ACT_ALLOW"})
profile["syscalls"] = rules
with open(out, "w") as f:
    json.dump(profile, f, indent=1)
    f.write("\n")
