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
    spec.paths.paths.retain(|_, value| {
        if let openapiv3::ReferenceOr::Item(pv) = value {
            if let Some(post) = pv.post.as_ref() {
                !post.request_body.as_ref().map_or(false, |body| match body {
                    openapiv3::ReferenceOr::Item(request_body) => {
                        request_body.content.contains_key("multipart/form-data")
                    }
                    openapiv3::ReferenceOr::Reference { .. } => false,
                })
            } else {
                true
            }
        } else {
            true
        }
    });

    let mut generator = progenitor::Generator::default();

    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("codegen.rs");

    fs::write(out_file, content).unwrap();
}
