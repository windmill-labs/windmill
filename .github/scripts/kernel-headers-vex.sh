#!/usr/bin/env bash
# Writes an OpenVEX document marking every linux-libc-dev CVE found in the given Trivy JSON
# reports as not_affected. linux-libc-dev holds only the kernel's userspace API headers, which
# build-essential needs to compile C; a container runs on its host's kernel, so a kernel CVE
# reported against this package never applies to the image. The product carries no version or
# qualifiers so one statement covers every architecture and package version.
#
# usage: kernel-headers-vex.sh <document id> <trivy.json>... > vex.openvex.json
set -euo pipefail

id="$1"
shift

jq -s --arg id "$id" --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '
  [.[].Results[]? | (.Vulnerabilities // [])[] | select(.PkgName == "linux-libc-dev") | .VulnerabilityID]
  | unique
  | {
      "@context": "https://openvex.dev/ns/v0.2.0",
      "@id": $id,
      author: "Windmill Labs",
      timestamp: $ts,
      version: 1,
      statements: map({
        vulnerability: { name: . },
        products: [{ "@id": "pkg:deb/debian/linux-libc-dev" }],
        status: "not_affected",
        justification: "vulnerable_code_not_present",
        impact_statement: "linux-libc-dev contains only the Linux userspace API headers used to compile C code. The image ships no kernel: containers run on the host kernel, which this package does not affect."
      })
    }
' "$@"
