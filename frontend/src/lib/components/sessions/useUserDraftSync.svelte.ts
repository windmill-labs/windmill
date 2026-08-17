import { untrack } from 'svelte'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'

/**
 * Per-kind projection between a `UserDraft` draft and a session editor's
 * runtime store. Carries the kind's behavioral quirks (flow's `initFlowState`
 * rebuild, script's merge-save) so {@link useUserDraftSync} stays generic.
 */
export interface DraftSyncCodec<Draft> {
	itemKind: UserDraftItemKind
	/**
	 * Inbound: write an incoming draft into the runtime store (and run any
	 * side effects, e.g. flow's `initFlowState`). Reads the store internally;
	 * a no-op when the store isn't populated.
	 */
	applyDraftToStore(draft: Draft): void
	/**
	 * Outbound: derive the draft to persist from the current store, or
	 * `undefined` when the store isn't populated. `current` is the existing
	 * UserDraft entry (script's merge-save needs it; flow/raw_app ignore it).
	 */
	storeToDraft(current: Draft | undefined): Draft | undefined
	/** Signature over a draft, comparable across both directions; drives de-dup. */
	sig(draft: Draft): string
	/** Outbound debounce; coalesces a typing burst into one persist. */
	debounceMs: number
}

export interface UserDraftSyncOptions<Draft> {
	/** Reactive editor path (the target being edited). */
	path: () => string
	/** Reactive workspace id (the session's, possibly forked, workspace). */
	workspace: () => string | undefined
	/**
	 * Reactive inert-gate: both effects no-op unless the runtime has settled on
	 * this exact path (`slot.loadedPath === path`). Replaces the old per-view
	 * `loadedX !== path` guards.
	 */
	ready: () => boolean
	/**
	 * Reactive codec. Retargeting a mounted editor (breadcrumb / in-editor link)
	 * changes `path` without remounting, and the codec closes over one
	 * `(kind, path)` cell's store — so it must be re-read per path, not captured
	 * once, or the sync would read/write the previously-targeted cell.
	 */
	codec: () => DraftSyncCodec<Draft>
}

/**
 * Bidirectional sync between a session editor's runtime store and the shared
 * `UserDraft` cell for `(workspace, kind, path)`. Holding a *live* handle
 * (`useMany`) is what lets the chat's writes (`write_script`, `patch_flow_json`,
 * …) reach the open preview: the chat persists through the backend syncer and
 * reflects into this in-tab cell, and the inbound effect below mirrors that into
 * the editor. Without a mounted handle there is no cell, so the chat falls back
 * to reading/writing the backend directly.
 *
 * - **inbound** (`handle.draft → store`): reflects external writes into the editor.
 * - **outbound** (`store → handle`, debounced): persists editor edits.
 *
 * One-way-reactive discipline: inbound tracks ONLY the handle's draft (reading
 * the store via `untrack`); outbound tracks ONLY the store (reading UserDraft via
 * `untrack`). Without that asymmetry a keystroke would re-fire the inbound effect
 * with the pre-keystroke value and revert the edit. `lastInboundSig` de-dups the
 * echo so an outbound save doesn't bounce back through inbound.
 *
 * Must be called once during component init (registers `useMany` + two `$effect`s).
 */
export function useUserDraftSync<Draft>(opts: UserDraftSyncOptions<Draft>): void {
	const handles = UserDraft.useMany<Draft>(() => {
		const p = opts.path()
		const ws = opts.workspace()
		return p && ws ? [{ itemKind: opts.codec().itemKind, path: p, workspace: ws }] : []
	})
	let lastInboundSig: string | undefined = $state(undefined)

	// inbound: handle.draft → store. Re-runs when the handle's draft changes
	// (chat write / another session's edit) or the target retargets (new codec).
	// The store read happens inside applyDraftToStore under untrack so the
	// editor's own mutations don't refire.
	$effect(() => {
		const codec = opts.codec()
		const incoming = handles[0]?.draft
		if (incoming == null) return
		const sig = codec.sig(incoming)
		untrack(() => {
			if (!opts.ready()) return
			// Centralized sig-based echo de-dup for all kinds. The flow/raw_app
			// originals already de-duped on `lastInboundSig`; the script original
			// instead compared `incoming === script.content` (store equality), and
			// `lastInboundSig` is intentionally not reset on a target swap. Both are
			// observably equivalent here: `applyDraftToStore` is idempotent (assigning
			// an unchanged value is a no-op under Svelte reactivity), so a redundant
			// re-apply only advances the sig, never reverts an edit or fires a save.
			if (sig === lastInboundSig) return
			lastInboundSig = sig
			codec.applyDraftToStore(incoming)
		})
	})

	// outbound: store → handle (debounced). Re-runs on any tracked store
	// mutation. `lastInboundSig` (read tracked here) makes the echo from an
	// inbound apply a no-op, terminating the loop.
	let outboundTimer: ReturnType<typeof setTimeout> | undefined
	// The latest scheduled-but-unwritten save (captures its own path/workspace),
	// so a target swap or unmount can flush it instead of dropping the last
	// `debounceMs` of edits. See the flush effect below.
	let pendingFlush: (() => void) | undefined
	$effect(() => {
		if (!opts.ready()) return
		const codec = opts.codec()
		const draft = codec.storeToDraft(undefined)
		if (draft == null) return
		const sig = codec.sig(draft)
		if (sig === lastInboundSig) return
		const path = opts.path()
		const workspace = opts.workspace()
		if (!path || !workspace) return
		const save = () => {
			if (outboundTimer) {
				clearTimeout(outboundTimer)
				outboundTimer = undefined
			}
			pendingFlush = undefined
			untrack(() => {
				const current = UserDraft.get<Draft>(codec.itemKind, path, { workspace })
				if (current && codec.sig(current) === sig) return
				const toSave = codec.storeToDraft(current) ?? draft
				UserDraft.save<Draft>(codec.itemKind, path, toSave, { workspace })
			})
		}
		pendingFlush = save
		if (outboundTimer) clearTimeout(outboundTimer)
		outboundTimer = setTimeout(save, codec.debounceMs)
	})

	// Flush a pending debounced write when the target path/workspace changes
	// (breadcrumb swap) or on unmount. Tracks ONLY path/workspace — a normal
	// typing burst (store mutation) re-runs the outbound effect above, not this
	// one, so it never flushes mid-burst and the debounce is preserved. Without
	// this, switching within the debounce window would silently drop the last
	// edits (scripts previously saved immediately, so this is a regression guard
	// for the new uniform debounce as well as a fix for flow/raw_app).
	$effect(() => {
		opts.path()
		opts.workspace()
		return () => pendingFlush?.()
	})
}
