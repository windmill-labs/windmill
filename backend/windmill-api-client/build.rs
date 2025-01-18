use std::{
    env,
    fs::{self, File},
    path::Path,
    process::Command,
};

use openapiv3::{OpenAPI, Operation};

fn main() {
    let src = "../windmill-api/openapi.yaml";
    println!("cargo:rerun-if-changed={}", src);
    Command::new("sh").args(&["bundle.sh"]).status().unwrap();
    let file = File::open("./bundled.json").unwrap();
    let mut spec: OpenAPI = serde_json::from_reader(file).unwrap();
    // Remove all multipart/form-data endpoints
    for (_, value) in spec.paths.paths.iter_mut() {
        if let openapiv3::ReferenceOr::Item(pv) = value {
            if let Some(post) = pv.post.as_mut() {
                if post.responses.responses.iter().any(|(_, v)| {
                    v.as_item()
                        .is_some_and(|x| x.content.contains_key("multipart/form-data"))
                }) {
                    *post = Operation::default();
                }
            }
        }
    }

    let mut generator = progenitor::Generator::default();

    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("codegen.rs");

    fs::write(out_file, content).unwrap();
}
