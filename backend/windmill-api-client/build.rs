use std::{
    env,
    fs::{self, File},
    path::Path,
    process::Command,
};

fn main() {
    let src = "../windmill-api/openapi.yaml";
    println!("cargo:rerun-if-changed={}", src);
    Command::new("sh").args(&["bundle.sh"]).status().unwrap();
    let file = File::open("./bundled.json").unwrap();
    let spec = serde_json::from_reader(file).unwrap();
    let mut generator = progenitor::Generator::default();

    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("codegen.rs");

    fs::write(out_file, content).unwrap();
}
