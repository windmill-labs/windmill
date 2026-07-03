import { untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { UserDraft, normalizeDraftForCompare, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { setLocalDraftHint } from '$lib/localDraftHints.svelte'

type Cfg = Record<string, any>

/**
 * Whether `a` differs from `b` after `normalizeDraftForCompare` (JSON
 * round-trip to drop `undefined`-valued keys, plus ignored deploy-directive
 * fields). Nullish `a` returns `false` ("no draft" = "no divergence"). A
 * `true` result narrows `a` to non-nullish `V`.
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
	/**
	 * Whether a deployed baseline exists (banner can appear). Reactive, unlike
	 * reading `deployed` directly (a plain `let`), so callers can reserve its slot.
	 */
	readonly hasBaseline: boolean
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

	// Reactive "banner is possible" — depends on `drawerLoading()` so it
	// re-evaluates (and re-reads the plain-`let` baseline) once the load settles.
	const hasBaseline = $derived(!opts.drawerLoading() && opts.deployed() != null)

	/** `auto: true` marks a discard from the reactive persist-effect (not an
	 * explicit user action), so it respects the "Enable auto-save" toggle — else
	 * with autosave off the editor would delete server drafts while writing none. */
	function discard(path: string, fallback: Cfg | undefined, auto = false): void {
		// `UserDraft.discard` POSTs `value: null`, which clears the list-page
		// `*` hint via the syncer. No explicit clear needed.
		UserDraft.discard(opts.itemKind, path, fallback, {
			workspace: opts.workspace() ?? undefined,
			auto
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

	/** Deferred + revalidated auto-discard. "At baseline but a draft exists"
	 * also occurs transiently during programmatic churn (list pages fire
	 * `openEdit` twice per row click), where an immediate discard would delete a
	 * draft the user never touched. Re-checking after a settle window keeps the
	 * legit revert case while the churn case reconciles before the timer fires. */
	let discardTimer: ReturnType<typeof setTimeout> | undefined
	function scheduleAutoDiscard() {
		if (discardTimer) return
		discardTimer = setTimeout(() => {
			discardTimer = undefined
			if (opts.drawerLoading()) return
			const cfg = opts.getCfg()
			const deployed = opts.deployed()
			const h = handle
			if (!h || cfg == null) return
			if (!cfgDiffers(cfg, deployed) && cfgDiffers(h.draft, deployed)) {
				discard(opts.path(), deployed, true)
			}
		}, 600)
	}

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
			} else if (cfgDiffers(h.draft, deployed)) {
				// Only when a draft actually exists to drop: `h.draft` equals
				// `deployed` right after a discard or the post-load seed.
				scheduleAutoDiscard()
			}
		})
	})

	// The list-page `*` hint is owned by UserDraftDbSyncer. This effect only
	// CLEARS it (never sets): while settled and at baseline, drop any stale
	// hint, so a draft discarded in another tab disappears on reopen.
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
		get hasBaseline() {
			return hasBaseline
		},
		get deployed() {
			return opts.deployed()
		},
		get current() {
			return opts.getCfg()
		},
		async maybeRestore() {
			const d = handle?.draft
			if (cfgDiffers(d, opts.getCfg() as Cfg)) {
				// Overlay the local autosave on the just-loaded backend config.
				await opts.applyCfg(d)
			}
			// Adopt the post-load form state as the cell's baseline without
			// POSTing, consuming the entry's one-shot first-write seed guard.
			// Trigger drawers never write the cell programmatically on open, so
			// without this the guard would swallow the user's first edit.
			const ws = opts.workspace()
			const p = opts.path()
			const cfg = opts.getCfg()
			if (ws && p && cfg != null) {
				UserDraft.seed(opts.itemKind, p, structuredClone($state.snapshot(cfg)) as Cfg, {
					workspace: ws
				})
			}
		},
		async resetToDeployed(path: string) {
			const deployedCfg = structuredClone($state.snapshot(opts.deployed())) as Cfg
			discard(path, deployedCfg)
			await opts.applyCfg(deployedCfg)
		},
		discard
	}
}
