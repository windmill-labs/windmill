use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ClientNotification, ClientRequest, Extensions, JsonObject, NumberOrString, ProgressToken,
    ServerNotification, ServerRequest,
};

pub trait GetMeta {
    fn get_meta_mut(&mut self) -> &mut Meta;
    fn get_meta(&self) -> &Meta;
}

pub trait GetExtensions {
    fn extensions(&self) -> &Extensions;
    fn extensions_mut(&mut self) -> &mut Extensions;
}

macro_rules! variant_extension {
    (
        $Enum: ident {
            $($variant: ident)*
        }
    ) => {
        impl GetExtensions for $Enum {
            fn extensions(&self) -> &Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &v.extensions,
                    )*
                }
            }
            fn extensions_mut(&mut self) -> &mut Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &mut v.extensions,
                    )*
                }
            }
        }
        impl GetMeta for $Enum {
            fn get_meta_mut(&mut self) -> &mut Meta {
                self.extensions_mut().get_or_insert_default()
            }
            fn get_meta(&self) -> &Meta {
                self.extensions().get::<Meta>().unwrap_or(Meta::static_empty())
            }
        }
    };
}

variant_extension! {
    ClientRequest {
        PingRequest
        InitializeRequest
        CompleteRequest
        SetLevelRequest
        GetPromptRequest
        ListPromptsRequest
        ListResourcesRequest
        ListResourceTemplatesRequest
        ReadResourceRequest
        SubscribeRequest
        UnsubscribeRequest
        CallToolRequest
        ListToolsRequest
    }
}

variant_extension! {
    ServerRequest {
        PingRequest
        CreateMessageRequest
        ListRootsRequest
    }
}

variant_extension! {
    ClientNotification {
        CancelledNotification
        ProgressNotification
        InitializedNotification
        RootsListChangedNotification
    }
}

variant_extension! {
    ServerNotification {
        CancelledNotification
        ProgressNotification
        LoggingMessageNotification
        ResourceUpdatedNotification
        ResourceListChangedNotification
        ToolListChangedNotification
        PromptListChangedNotification
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct Meta(pub JsonObject);
const PROGRESS_TOKEN_FIELD: &str = "progressToken";
impl Meta {
    pub fn new() -> Self {
        Self(JsonObject::new())
    }

    pub(crate) fn static_empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<Meta> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Default::default)
    }

    pub fn get_progress_token(&self) -> Option<ProgressToken> {
        self.0.get(PROGRESS_TOKEN_FIELD).and_then(|v| match v {
            Value::String(s) => Some(ProgressToken(NumberOrString::String(s.to_string().into()))),
            Value::Number(n) => n
                .as_u64()
                .map(|n| ProgressToken(NumberOrString::Number(n as u32))),
            _ => None,
        })
    }

    pub fn set_progress_token(&mut self, token: ProgressToken) {
        match token.0 {
            NumberOrString::String(ref s) => self.0.insert(
                PROGRESS_TOKEN_FIELD.to_string(),
                Value::String(s.to_string()),
            ),
            NumberOrString::Number(n) => self
                .0
                .insert(PROGRESS_TOKEN_FIELD.to_string(), Value::Number(n.into())),
        };
    }

    pub fn extend(&mut self, other: Meta) {
        for (k, v) in other.0.into_iter() {
            self.0.insert(k, v);
        }
    }
}

impl Deref for Meta {
    type Target = JsonObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
