import { untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { UserDraft, normalizeDraftForCompare, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { setLocalDraftHint } from '$lib/localDraftHints.svelte'

type Cfg = Record<string, any>

/**
 * Whether `a` differs meaningfully from `b` after normalizing both
 * sides through the shared draft comparator (`normalizeDraftForCompare`:
 * JSON round-trip — freshly-built configs keep `undefined`-valued keys,
 * so a raw `deepEqual` reports `{ a: undefined }` ≠ `{}` — plus the
 * ignored deploy-directive fields like `permissioned_as`). Returns
 * `false` when `a` is nullish (treats "no draft" as "no divergence").
 * Typed as a guard: a `true` result narrows `a` to non-nullish `V`.
 */
function cfgDiffers<V>(a: V | undefined | null, b: V | undefined): a is V {
	if (a === undefined || a === null) return false
	return !deepEqual(
		normalizeDraftForCompare(a),
		b === undefined ? undefined : normalizeDraftForCompare(b)
	)
}

export interface TriggerDraftSyncOptions {
	/** UserDraft item kind for this trigger, e.g. `'trigger_postgres'`. */
	itemKind: UserDraftItemKind
	/** Reactive editor path (the trigger being edited). */
	path: () => string
	/** Reactive workspace ($workspaceStore). */
	workspace: () => string | undefined
	/** Reactive loading flag — both effects are inert while true. */
	drawerLoading: () => boolean
	/** form → config: the editor's `getXCfg()` (or its `$derived`). */
	getCfg: () => Cfg | undefined
	/** config → form: the editor's `loadXConfig` / `loadScheduleCfg`. May be async. */
	applyCfg: (cfg: Cfg) => void | Promise<void>
	/**
	 * The deployed baseline the editor's dirty check compares against —
	 * `originalConfig` for TriggerCrud editors, `initialConfig` for Schedule.
	 */
	deployed: () => Cfg | undefined
}

export interface TriggerDraftSync {
	/** The local autosave for the active (workspace, path), if any. */
	readonly draft: Cfg | undefined
	/**
	 * Whether the form currently diverges from the deployed baseline — i.e. a
	 * live local draft exists. Drives the "unsaved changes" banner. False while
	 * loading and for brand-new triggers (no deployed baseline yet).
	 */
	readonly hasDraft: boolean
	/** The deployed baseline the dirty check compares against. */
	readonly deployed: Cfg | undefined
	/** The current form config (the live local draft). */
	readonly current: Cfg | undefined
	/**
	 * Restore-on-open: if a local autosave diverges from the just-loaded
	 * backend config, overlay it onto the form. Call right after the backend
	 * load, before clearing `drawerLoading`, so the drawer opens on the draft
	 * without a deployed→draft flash. The banner (via `hasDraft`) surfaces it.
	 */
	maybeRestore(): Promise<void>
	/**
	 * Drop the local draft for `path` and reset the form to the deployed
	 * baseline. Backs the banner's "Discard" action.
	 */
	resetToDeployed(path: string): Promise<void>
	/**
	 * Clear the draft for `path` and reset the handle's in-memory cell to
	 * `fallback`. Use after a successful deploy, passing the just-saved cfg —
	 * `discard` (not `UserDraft.remove`) so the apply-effect doesn't bounce
	 * the form back to the now-stale draft.
	 */
	discard(path: string, fallback: Cfg | undefined): void
}

/**
 * Shared local-autosave wiring for the trigger editors. Holding a live
 * `UserDraft` handle is what makes an external `UserDraft.save('trigger_x',
 * …)` (another tab, a programmatic write) propagate into the open editor.
 *
 * - **apply-effect**: reflects external `handle.draft` changes into the form.
 * - **persist-effect**: writes form edits back through the handle, dropping
 *   the draft when the form is back at the deployed baseline.
 *
 * Both effect bodies are `untrack`ed and gated by `cfgDiffers`
 * idempotence so they can't feed back into each other. Must be called once
 * during component init (it registers `useMany` + two `$effect`s).
 */
export function useTriggerDraftSync(opts: TriggerDraftSyncOptions): TriggerDraftSync {
	const handles = UserDraft.useMany<Cfg>(() => {
		const p = opts.path()
		const ws = opts.workspace()
		return p && ws ? [{ itemKind: opts.itemKind, path: p, workspace: ws }] : []
	})
	const handle = $derived(handles[0])

	// Live "is there a local draft?" — the form diverges from the deployed
	// baseline. Gated on `!drawerLoading` (the baseline isn't settled yet
	// mid-load) and on a non-null baseline (a brand-new trigger has none, so
	// "unsaved changes" / discard-to-deployed is meaningless there).
	const hasDraft = $derived(
		!opts.drawerLoading() &&
			opts.deployed() != null &&
			cfgDiffers(opts.getCfg() as Cfg, opts.deployed() as Cfg)
	)

	function discard(path: string, fallback: Cfg | undefined): void {
		// `UserDraft.discard` POSTs `value: null`, which routes through
		// `UserDraftDbSyncer.postSave` and clears the list-page `*` hint
		// (the syncer owns the hint). No explicit clear needed here.
		UserDraft.discard(opts.itemKind, path, fallback, {
			workspace: opts.workspace() ?? undefined
		})
	}

	// apply-effect: external handle.draft → form.
	$effect(() => {
		const d = handle?.draft
		if (opts.drawerLoading() || d == null) return
		untrack(() => {
			if (cfgDiffers(d, opts.getCfg() as Cfg)) {
				void opts.applyCfg(d)
			}
		})
	})

	// persist-effect: form edits → handle; drop the draft when back at the
	// deployed baseline.
	$effect(() => {
		if (opts.drawerLoading() || !opts.path()) return
		const cfg = opts.getCfg()
		if (cfg == null) return
		untrack(() => {
			const h = handle
			if (!h) return
			const deployed = opts.deployed()
			if (cfgDiffers(cfg, deployed)) {
				if (cfgDiffers(cfg, h.draft)) h.draft = cfg
			} else {
				discard(opts.path(), deployed)
			}
		})
	})

	// The list-page `*` hint is OWNED by UserDraftDbSyncer (set on save,
	// cleared on delete). This effect only CLEARS it: while the drawer is
	// settled (not loading) and the form is at the deployed baseline (no
	// draft), drop any stale hint — that's what makes a draft discarded
	// from another tab disappear on reopen. Never SET here (the syncer is
	// the single source). NOT cleared on teardown.
	$effect(() => {
		const ws = opts.workspace()
		const p = opts.path()
		const loading = opts.drawerLoading()
		const dirty = hasDraft
		untrack(() => {
			if (!ws || !p || loading) return
			if (!dirty) setLocalDraftHint(ws, opts.itemKind, p, false)
		})
	})

	return {
		get draft() {
			return handle?.draft
		},
		get hasDraft() {
			return hasDraft
		},
		get deployed() {
			return opts.deployed()
		},
		get current() {
			return opts.getCfg()
		},
		async maybeRestore() {
			const d = handle?.draft
			if (!cfgDiffers(d, opts.getCfg() as Cfg)) return
			// Overlay the local autosave on the just-loaded backend config.
			await opts.applyCfg(d)
		},
		async resetToDeployed(path: string) {
			const deployedCfg = structuredClone($state.snapshot(opts.deployed())) as Cfg
			discard(path, deployedCfg)
			await opts.applyCfg(deployedCfg)
		},
		discard
	}
}
