import type { Flow, NewScript } from '$lib/gen'
import { initFlowState } from '$lib/components/flows/flowState'
import { flowDraftSig } from './flowDraftSig'
import { applyDraftToRuntimeRawApp, runtimeRawAppToDraft, type RawAppDraft } from './appDraftCodec'
import type { RawAppRuntimeValue } from './sessionRuntime.svelte'
import type { StateStore } from '$lib/utils'
import type { DraftSyncCodec } from './useUserDraftSync.svelte'

// Outbound debounce, uniform across kinds (script was previously immediate;
// unified to 150ms so a typing burst coalesces into one persist like flow/raw_app).
const DEBOUNCE_MS = 150

// Each codec closes over one editor cell's store, so two live editors of the
// same kind sync to their own drafts without crossing.
export function makeFlowCodec(
	store: StateStore<Flow>,
	stateStore: { val: Record<string, any> },
	// The session's workspace, so schema rebuilds after an AI write resolve
	// path-referenced scripts/subflows against it rather than the nav workspace.
	workspace?: string
): DraftSyncCodec<Flow> {
	return {
		itemKind: 'flow',
		sig: flowDraftSig,
		debounceMs: DEBOUNCE_MS,
		applyDraftToStore(incoming) {
			const current = store.val
			if (!current) return
			store.val = {
				...current,
				value: incoming.value,
				schema: incoming.schema ?? current.schema,
				summary: incoming.summary ?? current.summary,
				description: incoming.description ?? current.description
			}
			// stateStore is keyed by module_id; after an AI write the set of
			// module ids may differ, so rebuild the UI state. This wipes per-module
			// test args / preview output — a known v1 trade-off.
			void initFlowState(store.val, stateStore, workspace, store.val.path ?? '')
		},
		storeToDraft() {
			return store.val
		}
	}
}

// `NewScript` has no `draft_path` of its own; the session editor parks a rename
// there so the home/Drafts lists (which read `draft_path`) show the typed name.
type ScriptDraft = NewScript & { draft_path?: string }

export function makeScriptCodec(
	store: { val: NewScript | undefined },
	// The draft's storage key (the URL path). A never-deployed script is parked
	// here at `…/draft_<uuid>` while the user's typed name lives in `script.path`.
	storagePath: () => string
): DraftSyncCodec<ScriptDraft> {
	return {
		itemKind: 'script',
		// Must include every field write_script can set — not just content —
		// else a summary-only/language-only change yields an identical signature
		// and the inbound/outbound sync skips it (the chat's change is then
		// invisible in the open editor and clobbered by the next content save).
		// `draft_path` carries a rename: ScriptBuilder binds the Path widget
		// straight to `script.path` (no separate draft field like flow/raw_app),
		// so without it here a rename moves no signature and never autosaves.
		sig: (d) =>
			JSON.stringify({
				content: d.content ?? '',
				summary: d.summary,
				language: d.language,
				draft_path: d.draft_path
			}),
		debounceMs: DEBOUNCE_MS,
		applyDraftToStore(incoming) {
			const script = store.val
			if (!script) return
			if (typeof incoming.content !== 'string') return
			script.content = incoming.content
			if (incoming.language) script.language = incoming.language
			if (incoming.summary !== undefined) script.summary = incoming.summary
		},
		storeToDraft(current) {
			const script = store.val
			if (!script) return undefined
			// Merge over the existing entry so fields the preview doesn't edit
			// (set by the chat) survive a content-only save.
			const merged: ScriptDraft = { ...(current ?? script), ...script }
			// Surface a rename to the home/Drafts lists, which read `draft_path`
			// (the typed `script.path` is the draft *value*'s path, not its storage
			// key). Mirror flow/raw_app: set it only when the typed path differs
			// from the storage key, and drop it once it matches again so a revert
			// doesn't leave a stale friendly name behind.
			const typed = script.path
			if (typed && typed !== storagePath()) merged.draft_path = typed
			else delete merged.draft_path
			return merged
		}
	}
}

export function makeRawAppCodec(store: {
	val: RawAppRuntimeValue | undefined
}): DraftSyncCodec<RawAppDraft> {
	return {
		itemKind: 'raw_app',
		sig: (d) => JSON.stringify(d),
		debounceMs: DEBOUNCE_MS,
		applyDraftToStore(incoming) {
			const current = store.val
			if (!current) return
			store.val = applyDraftToRuntimeRawApp(current, incoming)
		},
		storeToDraft() {
			const raw = store.val
			if (!raw) return undefined
			return runtimeRawAppToDraft(raw)
		}
	}
}
