#!/usr/bin/env nu

let cli_cache = "/tmp/windmill/cache_nomount/bun/"
let bundle_cache = "/tmp/windmill/cache/bun/"

# Clean CLI package cache
def "main clean" [] {
    rm -rf ($cli_cache ++ "windmill-cli@*")
    rm -rf ($cli_cache ++ "windmill-cli/")
    print "Cleaned CLI cache"
}

# Clear bundle cache (forces hub scripts to re-bundle)
def "main clear-bundles" [] {
    rm -rf ($bundle_cache ++ "*")
    print "Cleared bundle cache"
}

# Patch CLI cache with local build
def "main patch" [] {
    print "Patching CLI cache..."

    let versions = (ls $cli_cache | where name =~ "windmill-cli@" | get name)

    if ($versions | is-empty) {
        print "No CLI versions found in cache"
        return
    }

    for path in $versions {
        rm -rf ($path ++ "/esm")
        ^cp -r npm/esm ($path ++ "/esm")
        ^cp npm/package.json ($path ++ "/package.json")
        print $"Patched ($path | path basename)"
    }

    print "Done!"
}

# Build CLI, patch cache, and clear bundles
def main [
    --patch(-p)  # Only patch existing cache (skip build)
    --clean(-c)  # Clean CLI cache first
] {
    if $clean {
        main clean
    }

    if $patch {
        main patch
    } else {
        print "Building CLI..."
        bun run build
        main patch
    }

    # Always clear bundle cache so hub scripts use patched CLI
    main clear-bundles
}
