use proc_macro::TokenStream;
use quote::quote;
use proc_macro2::Span;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn windmill_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut method: Option<String> = None;
    let mut path: Option<String> = None;
    let mut path_params_fn: Option<syn::Ident> = None;
    let mut query_fn: Option<syn::Ident> = None;
    let mut body_fn: Option<syn::Ident> = None;

    let attr_string = attr.to_string();
    for part in attr_string.split(',') {
        let trimmed = part.trim();
        if let Some((k,v)) = trimmed.split_once('=') {
            let key = k.trim();
            let mut val = v.trim();
            println!("val: {}", val);
            if val.starts_with("r#\"") {
                // strip leading r#" and trailing "# (simplistic)
                if let Some(stripped) = val.strip_prefix("r#\"") {
                    if let Some(inner) = stripped.strip_suffix("\"#") {
                        val = inner;
                    }
                }
            }
            let val = val.trim_matches('"');
            match key {
                "name" => name = Some(val.to_string()),
                "description" => description = Some(val.to_string()),
                "method" => method = Some(val.to_string()),
                "path" => path = Some(val.to_string()),
                "path_params_fn" => path_params_fn = Some(syn::Ident::new(val, proc_macro2::Span::call_site())),
                "query_fn" => query_fn = Some(syn::Ident::new(val, proc_macro2::Span::call_site())),
                "body_fn" => body_fn = Some(syn::Ident::new(val, proc_macro2::Span::call_site())),
                _ => {}
            }
        }
    }

    let name_lit = name.expect("name required");
    let path_lit = path.expect("path required");
    let descr_lit = description.unwrap_or_default();
    let method_lit = method.unwrap_or_else(|| "GET".to_string());

    // map to constant when possible to avoid const-eval problems
    let method_tokens = {
        match method_lit.as_str() {
            "GET" => quote! { http::Method::GET },
            "POST" => quote! { http::Method::POST },
            "PUT" => quote! { http::Method::PUT },
            "DELETE" => quote! { http::Method::DELETE },
            "PATCH" => quote! { http::Method::PATCH },
            "HEAD" => quote! { http::Method::HEAD },
            "OPTIONS" => quote! { http::Method::OPTIONS },
            _ => {
                let s = method_lit.clone();
                quote! { http::Method::from_bytes(#s.as_bytes()).unwrap() }
            }
        }
    };

    let pp_schema_tokens = if let Some(s) = path_params_fn {
        quote! { Some(#s) }
    } else { quote! { None } };
    let q_schema_tokens = if let Some(s) = query_fn {
        quote! { Some(#s) }
    } else { quote! { None } };
    let b_schema_tokens = if let Some(s) = body_fn {
        quote! { Some(#s) }
    } else { quote! { None } };

    let expanded = quote! {
        #func

        ::inventory::submit! {
            ::windmill_tool_registry::EndpointTool {
                name: ::std::borrow::Cow::Borrowed(#name_lit),
                description: ::std::borrow::Cow::Borrowed(#descr_lit),
                method: #method_tokens,
                path: ::std::borrow::Cow::Borrowed(#path_lit),
                handler: |_: ::serde_json::Value| -> windmill_tool_registry::HandlerFuture {
                    ::std::boxed::Box::pin(async move { ::serde_json::Value::Null })
                },
                path_params_schema: #pp_schema_tokens,
                query_schema: #q_schema_tokens,
                body_schema: #b_schema_tokens,
            }
        }
    };

    TokenStream::from(expanded)
} 