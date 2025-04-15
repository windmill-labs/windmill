use std::{
    any::TypeId, borrow::Cow, collections::HashMap, future::Ready, marker::PhantomData, sync::Arc,
};

use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio_util::sync::CancellationToken;

use crate::{
    RoleServer,
    model::{CallToolRequestParam, CallToolResult, ConstString, IntoContents, JsonObject},
    service::RequestContext,
};
/// A shortcut for generating a JSON schema for a type.
pub fn schema_for_type<T: JsonSchema>() -> JsonObject {
    let schema = schemars::r#gen::SchemaGenerator::default().into_root_schema_for::<T>();
    let object = serde_json::to_value(schema).expect("failed to serialize schema");
    match object {
        serde_json::Value::Object(object) => object,
        _ => panic!("unexpected schema value"),
    }
}

/// Call [`schema_for_type`] with a cache
pub fn cached_schema_for_type<T: JsonSchema + std::any::Any>() -> Arc<JsonObject> {
    thread_local! {
        static CACHE_FOR_TYPE: std::sync::RwLock<HashMap<TypeId, Arc<JsonObject>>> = Default::default();
    };
    CACHE_FOR_TYPE.with(|cache| {
        if let Some(x) = cache
            .read()
            .expect("schema cache lock poisoned")
            .get(&TypeId::of::<T>())
        {
            x.clone()
        } else {
            let schema = schema_for_type::<T>();
            let schema = Arc::new(schema);
            cache
                .write()
                .expect("schema cache lock poisoned")
                .insert(TypeId::of::<T>(), schema.clone());
            schema
        }
    })
}

/// Deserialize a JSON object into a type
pub fn parse_json_object<T: DeserializeOwned>(input: JsonObject) -> Result<T, crate::Error> {
    serde_json::from_value(serde_json::Value::Object(input)).map_err(|e| {
        crate::Error::invalid_params(
            format!("failed to deserialize parameters: {error}", error = e),
            None,
        )
    })
}
pub struct ToolCallContext<'service, S> {
    request_context: RequestContext<RoleServer>,
    service: &'service S,
    name: Cow<'static, str>,
    arguments: Option<JsonObject>,
}

impl<'service, S> ToolCallContext<'service, S> {
    pub fn new(
        service: &'service S,
        CallToolRequestParam { name, arguments }: CallToolRequestParam,
        request_context: RequestContext<RoleServer>,
    ) -> Self {
        Self {
            request_context,
            service,
            name,
            arguments,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub trait FromToolCallContextPart<'a, S>: Sized {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error>;
}

pub trait IntoCallToolResult {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error>;
}
impl IntoCallToolResult for () {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        Ok(CallToolResult::success(vec![]))
    }
}

impl<T: IntoContents> IntoCallToolResult for T {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        Ok(CallToolResult::success(self.into_contents()))
    }
}

impl<T: IntoContents, E: IntoContents> IntoCallToolResult for Result<T, E> {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        match self {
            Ok(value) => Ok(CallToolResult::success(value.into_contents())),
            Err(error) => Ok(CallToolResult::error(error.into_contents())),
        }
    }
}

pin_project_lite::pin_project! {
    #[project = IntoCallToolResultFutProj]
    pub enum IntoCallToolResultFut<F, R> {
        Pending {
            #[pin]
            fut: F,
            _marker: PhantomData<R>,
        },
        Ready {
            #[pin]
            result: Ready<Result<CallToolResult, crate::Error>>,
        }
    }
}

impl<F, R> Future for IntoCallToolResultFut<F, R>
where
    F: Future<Output = R>,
    R: IntoCallToolResult,
{
    type Output = Result<CallToolResult, crate::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            IntoCallToolResultFutProj::Pending { fut, _marker } => {
                fut.poll(cx).map(IntoCallToolResult::into_call_tool_result)
            }
            IntoCallToolResultFutProj::Ready { result } => result.poll(cx),
        }
    }
}

impl IntoCallToolResult for Result<CallToolResult, crate::Error> {
    fn into_call_tool_result(self) -> Result<CallToolResult, crate::Error> {
        self
    }
}

pub trait CallToolHandler<'a, S, A> {
    type Fut: Future<Output = Result<CallToolResult, crate::Error>> + Send + 'a;
    fn call(self, context: ToolCallContext<'a, S>) -> Self::Fut;
}

pub type DynCallToolHandler<S> = dyn Fn(ToolCallContext<'_, S>) -> BoxFuture<'_, Result<CallToolResult, crate::Error>>
    + Send
    + Sync;
/// Parameter Extractor
pub struct Parameter<K: ConstString, V>(pub K, pub V);

/// Parameter Extractor
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Parameters<P>(pub P);

impl<P: JsonSchema> JsonSchema for Parameters<P> {
    fn schema_name() -> String {
        P::schema_name()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        P::json_schema(generator)
    }
}

/// Callee Extractor
pub struct Callee<'a, S>(pub &'a S);

impl<'a, S> FromToolCallContextPart<'a, S> for CancellationToken {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((context.request_context.ct.clone(), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for Callee<'a, S> {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((Callee(context.service), context))
    }
}

pub struct ToolName(pub Cow<'static, str>);

impl<'a, S> FromToolCallContextPart<'a, S> for ToolName {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((Self(context.name.clone()), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for &'a S {
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        Ok((context.service, context))
    }
}

impl<'a, S, K, V> FromToolCallContextPart<'a, S> for Parameter<K, V>
where
    K: ConstString,
    V: DeserializeOwned,
{
    fn from_tool_call_context_part(
        context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let arguments = context
            .arguments
            .as_ref()
            .ok_or(crate::Error::invalid_params(
                format!("missing parameter {field}", field = K::VALUE),
                None,
            ))?;
        let value = arguments.get(K::VALUE).ok_or(crate::Error::invalid_params(
            format!("missing parameter {field}", field = K::VALUE),
            None,
        ))?;
        let value: V = serde_json::from_value(value.clone()).map_err(|e| {
            crate::Error::invalid_params(
                format!(
                    "failed to deserialize parameter {field}: {error}",
                    field = K::VALUE,
                    error = e
                ),
                None,
            )
        })?;
        Ok((Parameter(K::default(), value), context))
    }
}

impl<'a, S, P> FromToolCallContextPart<'a, S> for Parameters<P>
where
    P: DeserializeOwned,
{
    fn from_tool_call_context_part(
        mut context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let arguments = context.arguments.take().unwrap_or_default();
        let value: P =
            serde_json::from_value(serde_json::Value::Object(arguments)).map_err(|e| {
                crate::Error::invalid_params(
                    format!("failed to deserialize parameters: {error}", error = e),
                    None,
                )
            })?;
        Ok((Parameters(value), context))
    }
}

impl<'a, S> FromToolCallContextPart<'a, S> for JsonObject {
    fn from_tool_call_context_part(
        mut context: ToolCallContext<'a, S>,
    ) -> Result<(Self, ToolCallContext<'a, S>), crate::Error> {
        let object = context.arguments.take().unwrap_or_default();
        Ok((object, context))
    }
}

impl<'s, S> ToolCallContext<'s, S> {
    pub fn invoke<H, A>(self, h: H) -> H::Fut
    where
        H: CallToolHandler<'s, S, A>,
    {
        h.call(self)
    }
}

#[allow(clippy::type_complexity)]
pub struct AsyncAdapter<P, Fut, R>(PhantomData<(fn(P) -> Fut, fn(Fut) -> R)>);
pub struct SyncAdapter<P, R>(PhantomData<fn(P) -> R>);

macro_rules! impl_for {
    ($($T: ident)*) => {
        impl_for!([] [$($T)*]);
    };
    // finished
    ([$($Tn: ident)*] []) => {
        impl_for!(@impl $($Tn)*);
    };
    ([$($Tn: ident)*] [$Tn_1: ident $($Rest: ident)*]) => {
        impl_for!(@impl $($Tn)*);
        impl_for!([$($Tn)* $Tn_1] [$($Rest)*]);
    };
    (@impl $($Tn: ident)*) => {
        impl<'s, $($Tn,)* S, F, Fut, R> CallToolHandler<'s, S, AsyncAdapter<($($Tn,)*), Fut, R>> for F
        where
            $(
                $Tn: FromToolCallContextPart<'s, S> + 's,
            )*
            F: FnOnce($($Tn,)*) -> Fut + Send + 's,
            Fut: Future<Output = R> + Send + 's,
            R: IntoCallToolResult + Send + 's,
            S: Send + Sync,
        {
            type Fut = IntoCallToolResultFut<Fut, R>;
            #[allow(unused_variables, non_snake_case)]
            fn call(
                self,
                context: ToolCallContext<'s, S>,
            ) -> Self::Fut {
                $(
                    let result = $Tn::from_tool_call_context_part(context);
                    let ($Tn, context) = match result {
                        Ok((value, context)) => (value, context),
                        Err(e) => return IntoCallToolResultFut::Ready {
                            result: std::future::ready(Err(e)),
                        },
                    };
                )*
                IntoCallToolResultFut::Pending {
                    fut: self($($Tn,)*),
                    _marker: PhantomData
                }
            }
        }

        impl<'s, $($Tn,)* S, F, R> CallToolHandler<'s, S, SyncAdapter<($($Tn,)*), R>> for F
        where
            $(
                $Tn: FromToolCallContextPart<'s, S> + 's,
            )*
            F: FnOnce($($Tn,)*) -> R + Send + 's,
            R: IntoCallToolResult + Send + 's,
            S: Send + Sync,
        {
            type Fut = Ready<Result<CallToolResult, crate::Error>>;
            #[allow(unused_variables, non_snake_case)]
            fn call(
                self,
                context: ToolCallContext<'s, S>,
            ) -> Self::Fut {
                $(
                    let result = $Tn::from_tool_call_context_part(context);
                    let ($Tn, context) = match result {
                        Ok((value, context)) => (value, context),
                        Err(e) => return std::future::ready(Err(e)),
                    };
                )*
                std::future::ready(self($($Tn,)*).into_call_tool_result())
            }
        }
    };
}
impl_for!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15);
pub struct ToolBoxItem<S> {
    #[allow(clippy::type_complexity)]
    pub call: Box<DynCallToolHandler<S>>,
    pub attr: crate::model::Tool,
}

impl<S: Send + Sync + 'static + Clone> ToolBoxItem<S> {
    pub fn new<C>(attr: crate::model::Tool, call: C) -> Self
    where
        C: Fn(ToolCallContext<'_, S>) -> BoxFuture<'_, Result<CallToolResult, crate::Error>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            call: Box::new(call),
            attr,
        }
    }
    pub fn name(&self) -> &str {
        &self.attr.name
    }
}

#[derive(Default)]
pub struct ToolBox<S> {
    #[allow(clippy::type_complexity)]
    pub map: std::collections::HashMap<Cow<'static, str>, ToolBoxItem<S>>,
}

impl<S> ToolBox<S> {
    pub fn new() -> Self {
        Self {
            map: std::collections::HashMap::new(),
        }
    }
    pub fn add(&mut self, item: ToolBoxItem<S>) {
        self.map.insert(item.attr.name.clone(), item);
    }

    pub fn remove<H, A>(&mut self, name: &str) {
        self.map.remove(name);
    }

    pub async fn call(
        &self,
        context: ToolCallContext<'_, S>,
    ) -> Result<CallToolResult, crate::Error> {
        let item = self
            .map
            .get(context.name())
            .ok_or_else(|| crate::Error::invalid_params("tool not found", None))?;
        (item.call)(context).await
    }

    pub fn list(&self) -> Vec<crate::model::Tool> {
        self.map.values().map(|item| item.attr.clone()).collect()
    }
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! tool_box {
    (@pin_add $callee: ident, $attr: expr, $f: expr) => {
        $callee.add(ToolBoxItem::new($attr, |context| Box::pin($f(context))));
    };
    ($server: ident { $($tool: ident),* $(,)?} ) => {
        $crate::tool_box!($server { $($tool),* }  tool_box);
    };
    ($server: ident { $($tool: ident),* $(,)?} $tool_box: ident) => {
        fn $tool_box() -> &'static $crate::handler::server::tool::ToolBox<$server> {
            use $crate::handler::server::tool::{ToolBox, ToolBoxItem};
            static TOOL_BOX: std::sync::OnceLock<ToolBox<$server>> = std::sync::OnceLock::new();
            TOOL_BOX.get_or_init(|| {
                let mut tool_box = ToolBox::new();
                $crate::paste!{
                    $(
                        $crate::tool_box!(@pin_add tool_box, $server::[< $tool _tool_attr>](), $server::[<$tool _tool_call>]);
                    )*
                }
                tool_box
            })
        }
    };
    (@derive) => {
        $crate::tool_box!(@derive tool_box);
    };

    (@derive $tool_box:ident) => {
        async fn list_tools(
            &self,
            _: Option<$crate::model::PaginatedRequestParam>,
            _: $crate::service::RequestContext<$crate::service::RoleServer>,
        ) -> Result<$crate::model::ListToolsResult, $crate::Error> {
            Ok($crate::model::ListToolsResult {
                next_cursor: None,
                tools: Self::tool_box().list(),
            })
        }

        async fn call_tool(
            &self,
            call_tool_request_param: $crate::model::CallToolRequestParam,
            context: $crate::service::RequestContext<$crate::service::RoleServer>,
        ) -> Result<$crate::model::CallToolResult, $crate::Error> {
            let context = $crate::handler::server::tool::ToolCallContext::new(self, call_tool_request_param, context);
            Self::$tool_box().call(context).await
        }
    }
}
