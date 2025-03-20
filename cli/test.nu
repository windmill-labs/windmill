#!/usr/bin/env nu
# NOTE: Not polished yet
# This script supposed to test wasm parsers in cli
# Currently you would need to create local dir in repo root (it is ignored by git)
# In there init windmill workspace
#
# Start instance with languages features on
# Invoke this script from local: ../cli/test.nu test deno

const languages = [
  "python3",
  "nativets",
  "bun",
  "deno",
  "go",
  "mysql",
  "bigquery",
  "oracledb",
  "snowflake",
  "mssql",
  "postgresql",
  "graphql",
  "bash",
  "powershell",
  # "php",
  "rust",
  "csharp",
  "nu",
  "ansible",
];

def main [] {
  main test deno;
  main build;
  main test node;
}

def 'main test deno' [] {
  main clean;
  $languages | each { |l|
    print $"+ ($l)"
    deno run -A ../cli/main.ts script bootstrap $"f/tests/(random uuid)" $l
  }
  deno run -A ../cli/main.ts script generate-metadata
}

def 'main test node' [] {
  # TODO: Use node
  main clean;
  $languages | each { |l|
    print $"+ ($l)"
    node ../cli/npm/esm/main.js script bootstrap $"f/tests/(random uuid)" $l
  }
  node ../cli/npm/esm/main.js script generate-metadata
}

def 'main clean' [] {
  rm -rf f/tests/*
}

def 'main build' [] {
  ../cli/build.sh
}

# def main [] {
#   main clean;
#   $languages | each { |l|
#     print $"+ ($l)"
#     # deno run -A ../cli/main.ts script bootstrap $"f/tests/(random uuid)" $l
#     #
#     let cmd = r#'
#     import { inferSchema } from '../cli/metadata.ts';

#     await inferSchema("LANG", "", undefined, "f/tests/NAME)")
#     '#
#     | str replace "LANG" $l
#     | str replace "NAME" (random uuid);

#     deno eval $cmd
#     }

#   # deno run -A ../cli/main.ts script generate-metadata
# }

# def 'main clean' [] {
#   rm -rf f/tests/*
# }

