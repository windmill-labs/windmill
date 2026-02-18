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
  "php",
  "rust",
  "csharp",
  "nu",
  "ansible",
];

def main [] {
  main test languages deno;
  main build;
  main test languages node;
}

def 'main test languages deno' [] {
  main clean;
  print $"Testing Deno"
  $languages | each { |l|
    print $"+ ($l)"
    deno run -A ../cli/src/main.ts script bootstrap $"f/tests/(random uuid)" $l
  }
  deno run -A ../cli/src/main.ts script generate-metadata
  print $"\nDeno has passed!\n"
}

def 'main test languages node' [] {
  main clean;
  print $"Testing Node"
  $languages | each { |l|
    print $"+ ($l)"
    node ../cli/npm/esm/main.js script bootstrap $"f/tests/(random uuid)" $l
  }
  node ../cli/npm/esm/main.js script generate-metadata
  print $"\nNode has passed!\n"
}

def 'main clean' [] {
  print $"Cleaning"
  rm -rf f/tests/*
  mkdir f/tests
}

def 'main build' [] {
  print $"Building..."
  cd ../cli; ./build.sh
  cd ../local
}

alias wm-cli = deno run -A --no-check ../cli/src/main.ts


def 'main setup workspace_deps' [] {
  mkdir f/workspace_deps
  mkdir dependencies

  wm-cli script bootstrap f/workspace_deps/test python3
  "# requirements: test" | save -f f/workspace_deps/test.py
  
  wm-cli script bootstrap f/workspace_deps/demo python3
  "# requirements: demo" | save -f f/workspace_deps/demo.py

  wm-cli script bootstrap f/workspace_deps/default python3
  "# requirements: default" | save -f f/workspace_deps/default.py

  # wm-cli script bootstrap f/workspace_deps/default_and_demo python3
  # "# requirements: default" | save -f f/workspace_deps/default_and_demo.py
  
  "tiny" | save -f dependencies/test.requirements.in
  "tiny" | save -f dependencies/demo.requirements.in
  "tiny" | save -f dependencies/requirements.in
}

# def main [] {
#   main clean;
#   $languages | each { |l|
#     print $"+ ($l)"
#     # deno run -A ../cli/src/main.ts script bootstrap $"f/tests/(random uuid)" $l
#     #
#     let cmd = r#'
#     import { inferSchema } from '../cli/metadata.ts';

#     await inferSchema("LANG", "", undefined, "f/tests/NAME)")
#     '#
#     | str replace "LANG" $l
#     | str replace "NAME" (random uuid);

#     deno eval $cmd
#     }

#   # deno run -A ../cli/src/main.ts script generate-metadata
# }

# def 'main clean' [] {
#   rm -rf f/tests/*
# }

