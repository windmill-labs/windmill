use serde::Serialize;

use crate::model::IntoContents;

/// Json wrapper
///
/// This is used to tell the SDK to serialize the inner value into json
pub struct Json<T>(pub T);

impl<T> IntoContents for Json<T>
where
    T: Serialize,
{
    fn into_contents(self) -> Vec<crate::model::Content> {
        let result = crate::model::Content::json(self.0);
        debug_assert!(
            result.is_ok(),
            "Json wrapped content should be able to serialized into json"
        );
        match result {
            Ok(content) => vec![content],
            Err(e) => {
                tracing::error!("failed to convert json content: {e}");
                vec![]
            }
        }
    }
}
