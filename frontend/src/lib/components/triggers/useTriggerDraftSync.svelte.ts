import { untrack } from 'svelte'
import { UserDraft, localDraftDiffers, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { notifyRestoredFromLocal } from '$lib/userDraftToast'

type Cfg = Record<string, any>

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
	 * Restore-on-open: if a local autosave diverges from the just-loaded
	 * backend config, overlay it and toast a "Reset to deployed" action.
	 * Call right after the backend load, before clearing `drawerLoading`.
	 */
	maybeRestore(path: string): Promise<void>
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
 * Both effect bodies are `untrack`ed and gated by `localDraftDiffers`
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

	function discard(path: string, fallback: Cfg | undefined): void {
		UserDraft.discard(opts.itemKind, path, fallback, {
			workspace: opts.workspace() ?? undefined
		})
	}

	// apply-effect: external handle.draft → form.
	$effect(() => {
		const d = handle?.draft
		if (opts.drawerLoading() || d == null) return
		untrack(() => {
			if (localDraftDiffers(d, opts.getCfg() as Cfg)) {
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
			if (localDraftDiffers(cfg, deployed)) {
				if (localDraftDiffers(cfg, h.draft)) h.draft = cfg
			} else {
				discard(opts.path(), deployed)
			}
		})
	})

	return {
		get draft() {
			return handle?.draft
		},
		async maybeRestore(path: string) {
			const d = handle?.draft
			if (!localDraftDiffers(d, opts.getCfg() as Cfg)) return
			// Snapshot the just-loaded backend config so "Reset to deployed"
			// can re-apply it, then overlay the local autosave.
			const deployedCfg = structuredClone($state.snapshot(opts.getCfg())) as Cfg
			await opts.applyCfg(d)
			notifyRestoredFromLocal(false, true, {
				onResetToDeployed: async () => {
					discard(path, deployedCfg)
					await opts.applyCfg(deployedCfg)
				}
			})
		},
		discard
	}
}
