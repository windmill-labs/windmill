//! The one parser for a script or flow named by path, in every spelling Windmill stores or accepts.
//! Resolution, preset handler matching and the checks on what a path runs read this value, so
//! they cannot disagree on what a string means. Save-time guards that only refuse a spelling may
//! read the raw value: they can refuse more, never run something the parse did not name.

use crate::error::{Error, Result};
use crate::utils::{parse_hub_id, RunnableKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnableRef {
    kind: RunnableKind,
    path: String,
    hub_id: Option<i32>,
}

impl RunnableRef {
    /// `script/<path>` or `flow/<path>`: schedule and workspace handlers, app runnables.
    pub fn from_prefixed(s: &str) -> Result<Self> {
        if let Some(rest) = s.strip_prefix("script/") {
            Self::build(RunnableKind::Script, rest)
        } else if let Some(rest) = s.strip_prefix("flow/") {
            Self::build(RunnableKind::Flow, rest)
        } else {
            Err(Error::BadRequest(format!(
                "path must start with script/ or flow/ (got {s})"
            )))
        }
    }

    /// `$script:<path>` or `$flow:<path>`, a WebSocket trigger URL computed by a runnable.
    /// `None` when `url` is a plain URL.
    pub fn from_ws_url(url: &str) -> Option<Result<Self>> {
        if let Some(rest) = url.strip_prefix("$script:") {
            Some(Self::build(RunnableKind::Script, rest))
        } else {
            url.strip_prefix("$flow:")
                .map(|rest| Self::build(RunnableKind::Flow, rest))
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

    /// A bare path with a separate flag: trigger and schedule targets, run routes.
    pub fn from_path(path: &str, is_flow: bool) -> Result<Self> {
        let kind = if is_flow {
            RunnableKind::Flow
        } else {
            RunnableKind::Script
        };
        Self::build(kind, path)
    }

    /// One leading `/` is tolerated, as the run routes have always stripped it. A second one is
    /// refused: a stored path normalized once and stripped again when read must not change meaning.
    fn build(kind: RunnableKind, rest: &str) -> Result<Self> {
        let path = rest.strip_prefix('/').unwrap_or(rest);
        if path.is_empty() || path.starts_with('/') {
            return Err(Error::BadRequest(format!(
                "invalid {kind} path '{rest}': expected a workspace or hub path"
            )));
        }
        let hub_prefix = match kind {
            RunnableKind::Script => "hub/",
            RunnableKind::Flow => "hub/flows/",
        };
        let hub_id = if path.starts_with(hub_prefix) {
            Some(parse_hub_id(path, hub_prefix)?)
        } else {
            None
        };
        Ok(Self { kind, path: path.to_string(), hub_id })
    }

    pub fn kind(&self) -> RunnableKind {
        self.kind
    }

    pub fn is_flow(&self) -> bool {
        self.kind == RunnableKind::Flow
    }

    /// The path without any prefix or leading `/`.
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn hub_id(&self) -> Option<i32> {
        self.hub_id
    }

    pub fn is_hub(&self) -> bool {
        self.hub_id.is_some()
    }

    /// The preset handler Windmill ships on the hub that this names, `None` for user code.
    pub fn preset_handler(&self) -> Option<&'static str> {
        if self.kind != RunnableKind::Script || !self.is_hub() {
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
        use RunnableKind::{Flow, Script};
        let p = |s: &str| RunnableRef::from_prefixed(s);
        let ws = |s: &str| RunnableRef::from_ws_url(s).expect("runnable url");
        let bare = |s: &str, f: bool| RunnableRef::from_path(s, f);
        let cases: Vec<(&str, Result<RunnableRef>, RunnableKind, &str, Option<i32>)> = vec![
            ("script/", p("script/u/me/x"), Script, "u/me/x", None),
            ("flow/", p("flow/f/a/b"), Flow, "f/a/b", None),
            ("script//", p("script//u/me/x"), Script, "u/me/x", None),
            (
                "hub script",
                p("script/hub/12/x"),
                Script,
                "hub/12/x",
                Some(12),
            ),
            (
                "hub flow",
                p("flow/hub/flows/76/x"),
                Flow,
                "hub/flows/76/x",
                Some(76),
            ),
            (
                "hub/ flow not flows/",
                p("flow/hub/123"),
                Flow,
                "hub/123",
                None,
            ),
            (
                "one prefix only",
                p("script/script/hub/1/x"),
                Script,
                "script/hub/1/x",
                None,
            ),
            ("$script:", ws("$script:u/me/x"), Script, "u/me/x", None),
            ("$flow:", ws("$flow:f/a/b"), Flow, "f/a/b", None),
            (
                "$script:/hub",
                ws("$script:/hub/1/x"),
                Script,
                "hub/1/x",
                Some(1),
            ),
            ("bare script", bare("u/me/x", false), Script, "u/me/x", None),
            ("bare flow", bare("f/a/b", true), Flow, "f/a/b", None),
            (
                "bare /hub",
                bare("/hub/9/x", false),
                Script,
                "hub/9/x",
                Some(9),
            ),
            (
                "setting flow/",
                RunnableRef::from_setting("flow/f/a/b"),
                Flow,
                "f/a/b",
                None,
            ),
            (
                "setting bare",
                RunnableRef::from_setting("u/me/x"),
                Script,
                "u/me/x",
                None,
            ),
        ];
        for (name, got, kind, path, hub_id) in cases {
            let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                (got.kind(), got.path(), got.hub_id()),
                (kind, path, hub_id),
                "{name}"
            );
        }

        for (name, got) in [
            ("empty", bare("", false)),
            ("only a slash", bare("/", false)),
            ("two slashes", bare("//hub/1/x", false)),
            ("two slashes prefixed", p("script///u/me/x")),
            ("two slashes ws", ws("$script://u/me/x")),
            ("unknown prefix", p("resource/u/me/x")),
            ("$script: in a prefixed field", p("$script:u/me/x")),
            ("hub non-numeric", bare("hub/abc", false)),
            ("hub zero", bare("hub/0/x", false)),
            ("hub flow non-numeric", bare("hub/flows/abc", true)),
            ("hub flow zero", bare("hub/flows/0", true)),
        ] {
            assert!(got.is_err(), "{name}: {got:?}");
        }
        assert!(RunnableRef::from_ws_url("wss://example.com").is_none());
    }

    #[test]
    fn only_pinned_hub_scripts_are_presets() {
        let preset = |s: &str| RunnableRef::from_prefixed(s).unwrap().preset_handler();
        assert!(preset("script/hub/28791/slack/schedule-recovery-handler-slack").is_some());
        // The hub resolves by id: any other id is arbitrary hub code whatever its name.
        assert!(preset("script/hub/13968/workspace-or-schedule-error-handler-slack").is_none());
        assert!(preset("script/u/me/workspace-or-schedule-error-handler-slack").is_none());
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
