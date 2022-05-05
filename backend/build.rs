use std::fs::File;
use std::io::Write;

use deno_core::{JsRuntime, RuntimeOptions};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let options = RuntimeOptions {
        will_snapshot: true,
        ..Default::default()
    };
    let mut runtime = JsRuntime::new(options);

    let mut snap = File::create("v8.snap").expect("can create snap file");
    snap.write_all(&runtime.snapshot())
        .expect("can write content to snap");
}
