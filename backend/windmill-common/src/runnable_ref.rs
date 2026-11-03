//! The one parser for a script or flow named by path. A check and the resolution after it read
//! this value, so they cannot disagree on what a string means. Save-time guards that only refuse
//! a spelling may read the raw value: they can refuse more, never run something the parse did not
//! name.

use crate::error::{Error, Result};
use crate::utils::{parse_hub_id, RunnableKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnableRef {
    kind: RunnableKind,
    path: String,
}

impl RunnableRef {
    /// A bare path with a separate flag, as the run routes take it.
    /// One leading `/` is tolerated, as the run routes have always stripped it. A second one is
    /// refused: a stored path normalized once and stripped again when read must not change meaning.
    /// A Hub path (`hub/<id>/…`, `hub/flows/<id>/…` for a flow) must carry a positive numeric id.
    pub fn from_path(path: &str, is_flow: bool) -> Result<Self> {
        let kind = if is_flow {
            RunnableKind::Flow
        } else {
            RunnableKind::Script
        };
        let canonical = path.strip_prefix('/').unwrap_or(path);
        if canonical.is_empty() || canonical.starts_with('/') {
            return Err(Error::BadRequest(format!(
                "invalid {kind} path '{path}': expected a workspace or hub path"
            )));
        }
        let hub_prefix = match kind {
            RunnableKind::Script => "hub/",
            RunnableKind::Flow => "hub/flows/",
        };
        if canonical.starts_with(hub_prefix) {
            parse_hub_id(canonical, hub_prefix)?;
        }
        Ok(Self { kind, path: canonical.to_string() })
    }

    pub fn is_flow(&self) -> bool {
        self.kind == RunnableKind::Flow
    }

    /// The path without its leading `/`.
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_spelling() {
        let bare = |s: &str, f: bool| RunnableRef::from_path(s, f);
        for (name, got, is_flow, path) in [
            ("script", bare("u/me/x", false), false, "u/me/x"),
            ("flow", bare("f/a/b", true), true, "f/a/b"),
            ("one slash", bare("/u/me/x", false), false, "u/me/x"),
            ("hub script", bare("hub/12/x", false), false, "hub/12/x"),
            ("/hub script", bare("/hub/9/x", false), false, "hub/9/x"),
            (
                "hub flow",
                bare("hub/flows/76/x", true),
                true,
                "hub/flows/76/x",
            ),
            // A flow's Hub prefix is `hub/flows/`: `hub/abc` is a workspace flow path.
            (
                "hub/ flow not flows/",
                bare("hub/abc", true),
                true,
                "hub/abc",
            ),
        ] {
            let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!((got.is_flow(), got.path()), (is_flow, path), "{name}");
        }

        for (name, got) in [
            ("empty", bare("", false)),
            ("only a slash", bare("/", false)),
            ("two slashes", bare("//u/me/x", false)),
            ("two slashes hub", bare("//hub/1/x", false)),
            ("hub non-numeric", bare("hub/abc", false)),
            ("hub zero", bare("hub/0/x", false)),
            ("hub flow path as a script", bare("hub/flows/1/x", false)),
            ("hub flow non-numeric", bare("hub/flows/abc", true)),
            ("hub flow zero", bare("hub/flows/0", true)),
        ] {
            assert!(got.is_err(), "{name}: {got:?}");
        }
    }
}
