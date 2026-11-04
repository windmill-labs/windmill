//! The one parser for a script or flow named by path. A check, the resolution after it and preset
//! handler identity read this value, so they cannot disagree on what a string means. Save-time
//! guards that only refuse a spelling may read the raw value: they can refuse more, never run
//! something the parse did not name.

use crate::error::{Error, Result};
use crate::utils::{parse_hub_id, RunnableKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnableRef {
    kind: RunnableKind,
    path: String,
}

impl RunnableRef {
    /// `script/<path>` or `flow/<path>`: schedule and workspace handlers.
    pub fn from_prefixed(s: &str) -> Result<Self> {
        if let Some(rest) = s.strip_prefix("script/") {
            Self::from_path(rest, false)
        } else if let Some(rest) = s.strip_prefix("flow/") {
            Self::from_path(rest, true)
        } else {
            Err(Error::BadRequest(format!(
                "path must start with script/ or flow/ (got {s})"
            )))
        }
    }

    /// `$script:<path>` or `$flow:<path>`, as a WebSocket trigger's URL names its resolver.
    /// `None` for anything else, a plain URL or an unknown `$` prefix.
    pub fn from_ws_url(url: &str) -> Option<Result<Self>> {
        if let Some(rest) = url.strip_prefix("$script:") {
            Some(Self::from_path(rest, false))
        } else {
            url.strip_prefix("$flow:")
                .map(|rest| Self::from_path(rest, true))
        }
    }

    /// A setting that takes the prefixed form or a bare script path: the global error handler,
    /// the Slack and Teams command scripts.
    pub fn from_setting(s: &str) -> Result<Self> {
        if s.starts_with("script/") || s.starts_with("flow/") {
            Self::from_prefixed(s)
        } else {
            Self::from_path(s, false)
        }
    }

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

    /// The canonical `script/<path>` or `flow/<path>` form.
    pub fn prefixed(&self) -> String {
        format!("{}/{}", self.kind, self.path)
    }

    /// The preset handler Windmill ships on the hub that this names, `None` for user code.
    pub fn preset_handler(&self) -> Option<&'static str> {
        if self.kind != RunnableKind::Script {
            return None;
        }
        PRESET_HANDLER_PATHS.into_iter().find(|p| *p == self.path)
    }
}

/// Every handler version the frontend has offered as a preset (`frontend/src/lib/hubPaths.json`
/// and its history, which stored schedules still point at). Matched whole: a hub script is
/// resolved by its numeric id alone, so the name after it proves nothing about its code.
const PRESET_HANDLER_PATHS: [&str; 26] = [
    "hub/6512/workspace-or-schedule-error-handler-slack",
    "hub/5792/workspace-or-schedule-error-handler-slack",
    "hub/9079/workspace-or-schedule-error-handler-slack",
    "hub/9206/workspace-or-schedule-error-handler-slack",
    "hub/19741/workspace-or-schedule-error-handler-slack",
    "hub/28241/workspace-or-schedule-error-handler-slack",
    "hub/28794/workspace-or-schedule-error-handler-slack",
    "hub/30057/workspace-or-schedule-error-handler-slack",
    "hub/2431/slack/schedule-error-handler-slack",
    "hub/2430/slack/schedule-recovery-handler-slack",
    "hub/9067/slack/schedule-recovery-handler-slack",
    "hub/9080/slack/schedule-recovery-handler-slack",
    "hub/28239/slack/schedule-recovery-handler-slack",
    "hub/28791/slack/schedule-recovery-handler-slack",
    "hub/30054/slack/schedule-recovery-handler-slack",
    "hub/9069/slack/schedule-success-handler-slack",
    "hub/9072/slack/schedule-success-handler-slack",
    "hub/28220/slack/schedule-success-handler-slack",
    "hub/28240/slack/schedule-success-handler-slack",
    "hub/28793/slack/schedule-success-handler-slack",
    "hub/30056/slack/schedule-success-handler-slack",
    "hub/11598/workspace-or-schedule-error-handler-teams",
    "hub/19742/workspace-or-schedule-error-handler-teams",
    "hub/11593/schedule-recovery-handler-teams",
    "hub/11596/schedule-success-handler-teams",
    "hub/19795/workspace-or-error-handler-email",
];

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

        let p = |s: &str| RunnableRef::from_prefixed(s).map(|r| (r.is_flow(), r.prefixed()));
        assert_eq!(
            p("script//u/me/x").unwrap(),
            (false, "script/u/me/x".into())
        );
        assert_eq!(p("flow/f/a/b").unwrap(), (true, "flow/f/a/b".into()));
        assert_eq!(
            p("script/script/hub/1/x").unwrap(),
            (false, "script/script/hub/1/x".into()),
            "exactly one prefix is stripped"
        );
        assert_eq!(
            RunnableRef::from_setting("u/me/x").unwrap().prefixed(),
            "script/u/me/x"
        );
        for bad in ["script///u/me/x", "resource/u/me/x", "u/me/x"] {
            assert!(p(bad).is_err(), "{bad}");
        }

        let ws = |s: &str| RunnableRef::from_ws_url(s).expect("runnable url");
        assert_eq!(ws("$script:/u/me/x").unwrap().path(), "u/me/x");
        assert!(ws("$flow:f/a/b").unwrap().is_flow());
        assert!(ws("$script://u/me/x").is_err());
        assert!(RunnableRef::from_ws_url("wss://example.com").is_none());
        assert!(RunnableRef::from_ws_url("$other:u/me/x").is_none());
    }

    #[test]
    fn only_pinned_hub_scripts_are_presets() {
        let preset = |s: &str| RunnableRef::from_prefixed(s).unwrap().preset_handler();
        assert!(preset("script/hub/28791/slack/schedule-recovery-handler-slack").is_some());
        assert!(preset("script//hub/6512/workspace-or-schedule-error-handler-slack").is_some());
        // The hub resolves by id: any other id is arbitrary hub code whatever its name.
        assert!(preset("script/hub/13968/workspace-or-schedule-error-handler-slack").is_none());
        assert!(preset("script/u/me/workspace-or-schedule-error-handler-slack").is_none());
        assert!(preset("flow/hub/6512/workspace-or-schedule-error-handler-slack").is_none());
    }

    /// A preset version bumped in the frontend without being pinned here would silently run as
    /// the schedule's owner and lose access to the Slack bot token.
    #[test]
    fn frontend_presets_are_pinned() {
        let hub_paths: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../../../frontend/src/lib/hubPaths.json")).unwrap();
        for (name, path) in hub_paths {
            // Variable expiration handlers run as their own group whatever their path.
            if path.contains("-handler-") && !path.contains("variable-expiration") {
                let r = RunnableRef::from_path(&path, false).unwrap();
                assert!(r.preset_handler().is_some(), "{name}: {path} is not pinned");
            }
        }
    }
}
