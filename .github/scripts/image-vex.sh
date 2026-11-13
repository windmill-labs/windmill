#!/usr/bin/env bash
# Writes the OpenVEX document for a release image from its unfiltered Trivy JSON reports.
#
# linux-libc-dev: every CVE found against it is not_affected. It holds only the kernel's userspace
# API headers, which build-essential needs to compile C; a container runs on its host's kernel, so
# a kernel CVE reported against this package never applies to the image. The product carries no
# version or qualifiers so one statement covers every architecture and package version.
#
# setuptools@70.3.0: the copy of pkg_resources pip vendors (pip/_vendor/vendor.txt) in the
# preinstalled Pythons. Only the listed CVEs are covered, each in a setuptools module pip does not
# vendor; a new CVE against that version must be checked before it joins the list. The version is
# pinned so the statements stop applying once pip vendors a different setuptools.
#
# usage: image-vex.sh <document id> <trivy.json>... > vex.openvex.json
set -euo pipefail

id="$1"
shift

jq -s --arg id "$id" --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" '
  ([.[].Results[]? | (.Vulnerabilities // [])[] | select(.PkgName == "linux-libc-dev") | .VulnerabilityID]
    | unique
    | map({
        vulnerability: { name: . },
        products: [{ "@id": "pkg:deb/debian/linux-libc-dev" }],
        status: "not_affected",
        justification: "vulnerable_code_not_present",
        impact_statement: "linux-libc-dev contains only the Linux userspace API headers used to compile C code. The image ships no kernel: containers run on the host kernel, which this package does not affect."
      })) as $kernel
  | ([
      ["CVE-2025-47273", "setuptools.package_index"],
      ["CVE-2026-59890", "setuptools.command.sdist and FileList"],
      ["GHSA-xmj3-9gf7-h52w", "setuptools.wheel"]
    ]
    | map({
        vulnerability: { name: .[0] },
        products: [{ "@id": "pkg:pypi/setuptools@70.3.0" }],
        status: "not_affected",
        justification: "vulnerable_code_not_present",
        impact_statement: ("setuptools 70.3.0 is present only as the pkg_resources module pip vendors. The vulnerable code is in " + .[1] + ", which pip does not vendor.")
      })) as $pip_setuptools
  | {
      "@context": "https://openvex.dev/ns/v0.2.0",
      "@id": $id,
      author: "Windmill Labs",
      timestamp: $ts,
      version: 1,
      statements: ($kernel + $pip_setuptools)
    }
' "$@"
