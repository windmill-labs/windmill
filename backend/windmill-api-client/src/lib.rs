include!("./codegen.rs");

pub fn create_client(base_url: &str, token: String) -> Client {
    let mut val = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
        .expect("header creation");
    val.set_sensitive(true);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::AUTHORIZATION, val);
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .expect("client build");
    Client::new_with_client(&format!("{}/api", base_url.trim_end_matches('/')), client)
}
