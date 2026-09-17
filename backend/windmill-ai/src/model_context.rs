//! What a model's context window holds, for the code that has to keep a conversation
//! inside it.

/// Assumed window for a model that is not in the table. Conservative on purpose: a
/// guess that is too small only compacts earlier, which is recoverable, while one that
/// is too large overflows and the provider raises the context error itself.
pub const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// Context windows of the models we know, most specific entry first — the first name
/// found in the bare model id wins, so vendor-namespaced and date-suffixed ids
/// (`anthropic.claude-sonnet-4-6-…-v1:0`, `gpt-5.2-2026-01-01`) still resolve.
/// Conservative family fallbacks sit below the explicit entries.
///
/// **Mirrors `MODEL_CONTEXT_WINDOWS` in
/// `frontend/src/lib/components/copilot/modelConfig.ts`**, which the AI session's own
/// compaction reads. Add a model to one and it must go in the other, or the same model
/// compacts at one size in a chat and another in an agent step.
const MODEL_CONTEXT_WINDOWS: &[(&str, usize)] = &[
    // Anthropic — Sonnet/Opus 4.6+ ship a 1M window at standard pricing (GA);
    // Haiku, older Claude models (3.x, 4.0, 4.1, 4.5) and date-suffixed Claude 4
    // base ids (claude-sonnet-4-20250514) fall through to 200K
    ("claude-fable-5", 1_000_000),
    ("claude-mythos-5", 1_000_000),
    ("claude-opus-5", 1_000_000),
    ("claude-sonnet-5", 1_000_000),
    ("claude-opus-4-8", 1_000_000),
    ("claude-opus-4-7", 1_000_000),
    ("claude-opus-4-6", 1_000_000),
    ("claude-sonnet-4-6", 1_000_000),
    ("claude", 200_000),
    // OpenAI — gpt-5 covers the base family (-mini / -nano) and the 5.1/5.2
    // revisions, all 400K; 5.4/5.5 moved to 1M and 5.6 to 1.05M
    ("gpt-5-6", 1_050_000),
    ("gpt-5-5", 1_000_000),
    ("gpt-5-4", 1_000_000),
    ("gpt-5", 400_000),
    ("gpt-4-1", 1_000_000),
    ("gpt-4o", 128_000),
    ("o4-mini", 200_000),
    ("o3", 200_000),
    // Google — the 2.5 / 3 / 3.1 Gemini families are all 1M
    ("gemini-3-1", 1_000_000),
    ("gemini-3", 1_000_000),
    ("gemini-2-5", 1_000_000),
    // DeepSeek — the V4 family (pro / flash) is 1M. The deepseek-chat /
    // deepseek-reasoner aliases were retired 2026-07-24 but can still sit in a
    // saved selection, so they keep resolving to the window they had.
    ("deepseek-v4", 1_000_000),
    ("deepseek-chat", 1_000_000),
    ("deepseek-reasoner", 1_000_000),
    ("deepseek", 128_000),
    // Alibaba — Qwen3-Max is 256K. No qwen family fallback: variant windows range
    // from 8K (character models) to 1M, too wide for even a conservative guess
    ("qwen3-max", 256_000),
    // Others — Mistral Medium 3.5 is 256K, reachable under both its version and
    // the `-latest` alias. There is deliberately no `mistral-medium` family row:
    // pinned older snapshots are 128K, and over-claiming a window overflows it.
    ("mistral-medium-3-5", 256_000),
    ("mistral-medium-latest", 256_000),
    ("llama", 128_000),
    ("codestral", 32_000),
];

/// The bare model id an entry is matched against: lowercased, the leading `~` and any
/// vendor path segment dropped, the `:`-suffixed route removed, and `.` collapsed to
/// `-`. Version separators differ by route to the same model — Anthropic writes
/// `claude-opus-4-8`, OpenRouter `anthropic/claude-opus-4.8` — so collapsing both sides
/// keeps one entry covering every route.
fn bare_model_id(model: &str) -> String {
    let normalized = model.trim().to_lowercase();
    let normalized = normalized.strip_prefix('~').unwrap_or(&normalized);
    let last = normalized.rsplit('/').next().unwrap_or(normalized);
    let base = match last.find(':') {
        Some(0) | None => last,
        Some(colon) => &last[..colon],
    };
    base.replace('.', "-")
}

/// The window this model is known to hold, `None` for one not in the table.
pub fn known_model_context_window(model: &str) -> Option<usize> {
    let id = bare_model_id(model);
    MODEL_CONTEXT_WINDOWS
        .iter()
        .find(|(name, _)| matches_entry(&id, name))
        .map(|(_, window)| *window)
}

/// The window to plan against: the model's where it is known, the conservative
/// assumption otherwise.
pub fn model_context_window(model: &str) -> usize {
    known_model_context_window(model).unwrap_or(DEFAULT_CONTEXT_WINDOW)
}

/// An entry matches anywhere in the id, so a vendor prefix or a date suffix does not
/// hide it. One ending on a version digit must not run into a longer version:
/// `gpt-4-1` would otherwise claim `gpt-4-1106-preview`. A family fallback ending on a
/// letter gets no such guard — a version welded straight onto the name (`llama3-1`) is
/// what it exists to catch.
fn matches_entry(id: &str, name: &str) -> bool {
    let guard_digits = name.ends_with(|c: char| c.is_ascii_digit());
    id.match_indices(name).any(|(at, _)| {
        !guard_digits
            || !id[at + name.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entry has to cover every route to the same model: the vendor path and dot
    /// versions OpenRouter uses, Bedrock's prefixed and `-v1:0`-suffixed ids, and the
    /// date suffixes the providers' own ids carry.
    #[test]
    fn every_route_to_a_model_resolves_to_one_window() {
        for id in [
            "claude-opus-4-8",
            "anthropic/claude-opus-4.8",
            "~anthropic/claude-opus-4.8",
            "anthropic.claude-opus-4-8-20260101-v1:0",
        ] {
            assert_eq!(known_model_context_window(id), Some(1_000_000), "{id}");
        }
        // Falls through the explicit rows to the family fallback.
        assert_eq!(
            known_model_context_window("claude-3-5-haiku"),
            Some(200_000)
        );
    }

    /// The guard that keeps an entry ending on a version digit from claiming a longer
    /// version of the same family.
    #[test]
    fn a_version_entry_does_not_claim_a_longer_version() {
        assert_eq!(known_model_context_window("gpt-4.1"), Some(1_000_000));
        assert_eq!(known_model_context_window("gpt-4-1106-preview"), None);
        assert_eq!(known_model_context_window("gpt-5-mini"), Some(400_000));
        assert_eq!(known_model_context_window("gpt-5.6"), Some(1_050_000));
    }

    /// An unknown model still gets a number, so nothing downstream has to carry a
    /// "no limit" case that would let a conversation grow unbounded.
    #[test]
    fn an_unknown_model_falls_back_to_the_assumed_window() {
        assert_eq!(known_model_context_window("some-local-llm"), None);
        assert_eq!(
            model_context_window("some-local-llm"),
            DEFAULT_CONTEXT_WINDOW
        );
    }
}
