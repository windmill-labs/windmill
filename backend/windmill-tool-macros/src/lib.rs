use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn windmill_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function
    let func: ItemFn = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    // Parse attribute arguments using simple string parsing for now
    let attr_string = attr.to_string();
    
    let mut name: Option<String> = None;
    let mut description = String::new();
    let mut path: Option<String> = None;

    // Very basic parsing - split by commas and look for key="value" pairs
    for part in attr_string.split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "name" => name = Some(value.to_string()),
                "description" => description = value.to_string(),
                "path" => path = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let name = name.unwrap_or_else(|| func_name.to_string());
    let path = path.unwrap_or_else(|| "/unknown".to_string());

    let expanded = quote! {
        #func

        // Submit the tool to the inventory registry
        ::inventory::submit! {
            ::windmill_tool_registry::EndpointTool {
                name: ::std::borrow::Cow::Borrowed(#name),
                description: ::std::borrow::Cow::Borrowed(#description),
                method: ::http::Method::GET,
                path: ::std::borrow::Cow::Borrowed(#path),
                handler: |args_json: ::serde_json::Value| {
                    ::std::boxed::Box::pin(async move { 
                        // For now, return a simple mock response
                        ::serde_json::json!({
                            "message": "Tool called",
                            "tool": #name,
                            "args": args_json
                        })
                    })
                },
            }
        }
    };

    TokenStream::from(expanded)
} 