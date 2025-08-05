use std::borrow::Cow;
use std::pin::Pin;
use std::future::Future;

use http::Method;
use serde_json::Value;

// Type alias for a boxed async handler returning a JSON value
pub type HandlerFuture = Pin<Box<dyn Future<Output = Value> + Send + 'static>>;

pub struct EndpointTool {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub method: Method,
    pub path: Cow<'static, str>,
    pub handler: fn(Value) -> HandlerFuture,
}

inventory::collect!(EndpointTool);

pub fn all_tools() -> Vec<&'static EndpointTool> {
    inventory::iter::<EndpointTool>.into_iter().collect()
} 