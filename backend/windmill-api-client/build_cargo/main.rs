use std::{
    fs::{self, File},
    path::Path,
    process::Command,
};

fn main() {
    Command::new("sh").args(&["./bundle.sh"]).status().unwrap();
    let file = File::open("./bundled.json").unwrap();
    let mut spec: openapiv3::OpenAPI = serde_json::from_reader(file).unwrap();
    spec.paths.paths.retain(|key, _| {
        [
            "/w/{workspace}/flows/create",
            "/w/{workspace}/flows/get/{path}",
            "/w/{workspace}/scripts/create",
            "/workspaces/list",
            "/w/{workspace}/schedules/create",
            "/w/{workspace}/schedules/update/{path}",
        ]
        .contains(&key.as_str())
    });

    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = Path::new("../src").to_path_buf();
    out_file.push("codegen.rs");

    fs::write(out_file, content).unwrap();
}
