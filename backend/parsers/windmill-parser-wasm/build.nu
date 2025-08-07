#!/usr/bin/env nu
const targets = [
  {
    ident: "ts", # OUT_DIR inferred from this
    desc: "Bun and Deno"
    features: "ts-parser",
    env: "default", # available: default, tree-sitter
  }, {
    ident: "regex",
    desc: "Sql languages, Graphql and Bash/Powershell"
    features: "sql-parser,graphql-parser,bash-parser",
    env: "default",
  }, {
    ident: "py",
    desc: "Python",
    features: "py-parser",
    env: "default",
  }, {
    ident: "go",
    desc: "Go (Go-lang)",
    features: "go-parser",
    env: "default",
  }, {
    ident: "php",
    desc: "Php",
    features: "php-parser",
    env: "default",
  }, {
    ident: "rust",
    desc: "Rust",
    features: "rust-parser",
    env: "default",
  }, {
    ident: "yaml",
    desc: "Ansible",
    features: "ansible-parser",
    env: "default",
  }, {
    ident: "csharp",
    desc: "C#",
    features: "csharp-parser",
    env: "tree-sitter",
  }, {
    ident: "nu",
    desc: "Nu (Nushell)",
    features: "nu-parser",
    env: "tree-sitter",
  }, {
    ident: "java",
    desc: "Java",
    features: "java-parser",
    env: "tree-sitter",
  }, {
    ident: "ruby",
    desc: "Ruby",
    features: "ruby-parser",
    env: "tree-sitter",
  },
  # ^^^ Add new entry here ^^^
];
# NOTE: This is legacy command for building all, but it is not more used
# #-# full pkg
# OUT_DIR="pkg"
# wasm-pack build --release --target web --out-dir $OUT_DIR --all-features \
# 	-Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort

# Build all separately
def 'main all' [ --no-opt(-n), --cli] {
  if ($no_opt) {
    if ($cli) {
      $targets | each { main $in.ident -n --cli }
    } else {
      $targets | each { main $in.ident -n }
    }
  } else {
    if ($cli) {
      $targets | each { main $in.ident --cli }
    } else {
      $targets | each { main $in.ident }
    }
  }
}

# Build specific language
def main [
  target: string, # Language, e.g.: py, ts, rust
  --no-opt(-n), # Compile in debug mode for dev
  --cli # Compile and place binaries in cli.
] {
  let t = $targets | where ident == $target;

  if ($t | is-empty) {
    let pretty = $targets | each {|t| $"• ($t.ident) - ($t.desc)"} | str join "\n";
    panic $"Target '($target)' does not exist\n\nAvailable targets:\n($pretty)"
  } else {
    let t = $t | get 0;
    mut profile = "";
    mut tar = "";

    if ($cli) {
      $env.OUT_DIR = $"../../../cli/wasm/($t.ident)"
      $tar = "deno"
    } else {
      $env.OUT_DIR = $"pkg-($t.ident)"
      $tar = "web"
    }
    if ($no_opt) {
      $profile = "--no-opt"
    } else {
      $profile = "--release"
    }
    print $"Building in ($profile) mode ($env.OUT_DIR)"
    match $t.env {
      "default" => {
        wasm-pack build ($profile) --target ($tar) --out-dir $env.OUT_DIR --features ($t.features) -Z build-std=panic_abort,std -Z build-std-features=panic_immediate_abort
      },
      "tree-sitter" => {
        $env.CFLAGS_wasm32_unknown_unknown = $"-I(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin"
        $env.RUSTFLAGS = "-Zwasm-c-abi=spec"
        wasm-pack build ($profile) --target ($tar) --out-dir $env.OUT_DIR --features $t.features
      },
      _ => { panic $"Unknown env template: ($t.env)" },
    }

    if ($cli) {
      rm $"($env.OUT_DIR)/.gitignore"
    } else {
      let p = $"($env.OUT_DIR)/package.json"
      open $p | update name $"windmill-parser-wasm-($t.ident)" | save -f $p
    }
  }
}

