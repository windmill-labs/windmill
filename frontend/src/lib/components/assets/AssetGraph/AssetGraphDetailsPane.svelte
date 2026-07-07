<script lang="ts">
	import { ScriptService, type Script, type ScriptLang } from '$lib/gen'
	import { resource } from 'runed'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatAssetKind, type AssetWithAltAccessType } from '$lib/components/assets/lib'
	import {
		AlertTriangle,
		ArrowUpRight,
		Code2,
		ExternalLink,
		GitBranch,
		GitFork,
		Loader2,
		PanelRightClose,
		Save,
		SquareFunction,
		Trash2,
		X,
		Pencil
	} from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { tick } from 'svelte'
	import { inferArgs } from '$lib/infer'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type { Schema } from '$lib/common'
	import type { AssetGraphSelection, PipelineMode } from './types'
	import PipelineScriptView from './PipelineScriptView.svelte'
	import {
		parsePipelineAnnotations,
		type ColumnLineage,
		type PipelineAnnotations
	} from './parsePipelineAnnotations'
	import ColumnLineageTrace from './ColumnLineageTrace.svelte'
	import { extractDraftMacros } from './resolveGraph'
	import { assetColumnNodes, type ColumnLineageGraph } from './columnLineageGraph'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import S3FilePreview from '$lib/components/S3FilePreview.svelte'
	import DataTablePreview from './DataTablePreview.svelte'
	import DucklakeAssetPanel from './DucklakeAssetPanel.svelte'
	import { notifyContractWarnings, type SchemaContractGraphContext } from './schemaContracts'
	import AssetRunsPanel from './AssetRunsPanel.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { fade } from 'svelte/transition'
	import { userStore } from '$lib/stores'

	interface Props {
		// Regular selection — loads the script by path for inline editing.
		selection?: AssetGraphSelection | undefined
		// Pre-built draft script not yet persisted. Takes precedence over
		// `selection` when present. Used by the pipeline + menu so a new
		// pipeline script opens inline instead of navigating to /scripts/add.
		draftScript?: Script | undefined
		// Local-dev preview (`/pipeline_dev`): resolve a selected node to its
		// working-tree content instead of fetching the deployed script (there is
		// none — the pipeline is local-only). Returns a read-only `Script` so the
		// pane renders the source + an ENABLED Run button (unlike `draftScript`,
		// which is intentionally not-runnable until deployed). May be async so the
		// caller can infer the args schema for the run form.
		resolveLocalScript?: (path: string) => Script | undefined | Promise<Script | undefined>
		// Bump this whenever `resolveLocalScript`'s backing content changes (a
		// `pipeline dev` live-reload pushes a new bundle). It's in the `scriptRes`
		// key so the open pane re-resolves the selected node's source even though
		// `selection` is unchanged — otherwise the pane sticks on stale content.
		localScriptsVersion?: unknown
		workspace: string
		onclose: () => void
		// Optional: dismiss the pane while preserving the current selection /
		// activeDraft so re-opening (toolbar HideButton) restores the same
		// view. When undefined, the hide button isn't shown.
		onHide?: () => void
		// Called after a draft is saved for the first time so the page can
		// refetch the graph and clear its local draft.
		onDraftSaved?: (savedPath: string) => void
		// Fired after a successful save of a *persisted* script so the
		// page can refetch the asset graph (the new asset rows the
		// deploy just inserted live in `base.edges`, and the live
		// overlay's cache gets cleared during the ScriptEditor remount
		// on the new hash — without a base refresh, the corresponding
		// write edges briefly vanish from the canvas).
		onPersistedSaved?: (savedPath: string) => void
		// Called when the user hits "Discard" on the draft header. Separate
		// from `onclose` so the page can drop the draft from its map vs
		// just dismissing the pane.
		onDiscard?: () => void
		// Emits live-parsed pipeline annotations from the open script so the
		// canvas can overlay unsaved schedule / trigger nodes in real time.
		// Fires whenever the script content changes.
		onAnnotationsChange?: (scriptPath: string | undefined, annotations: PipelineAnnotations) => void
		// Emits live-inferred body assets (read/write usages parsed by
		// inferAssets — e.g. CREATE TABLE in SQL, loadS3File / writeS3File
		// in TS/Python). The page uses the write subset to overlay write
		// edges + synthesize asset nodes for drafts whose body has been
		// edited past the seeded template. Fires on every keystroke that
		// changes the inferred set.
		onAssetsChange?: (
			scriptPath: string | undefined,
			assets: AssetWithAltAccessType[],
			columnLineage?: ColumnLineage[]
		) => void
		// Emits the live editor buffer on every keystroke so the parent can
		// autosave the in-flight content WITHOUT waiting for the pane teardown
		// (`onDraftPersist`). `onDraftPersist` stays the authoritative commit on
		// navigate-away; this is the continuous feed that keeps the draft's
		// server copy current while the user is still typing.
		onContentChange?: (scriptPath: string | undefined, content: string) => void
		// Fires when the user navigates away from a draft (selects another
		// node, closes the pane, etc.) — the parent must persist the new
		// content + write outputs into its drafts Map. Without this,
		// re-cloning from the unchanged Map discards local edits, and the
		// draft's overlay nodes (write edges) revert to the seeded values.
		// Also fires for a *deployed* script left with unsaved body edits
		// (snapshot.script carries the full edited copy, hash included) so
		// the parent can promote it to a draft instead of dropping the work.
		onDraftPersist?: (
			path: string,
			snapshot: {
				content: string
				writes: { kind: AssetWithAltAccessType['kind']; path: string }[]
				// Full edited script — set when the source is a persisted
				// script (needed to seed a brand-new draft entry).
				script?: Script
			}
		) => void
		// Called after the user moves/renames a persisted script via the
		// summary/path popover. The page repoints `selection` at `newPath`
		// and refetches the graph so the runnable node label updates.
		onScriptRenamed?: (oldPath: string, newPath: string) => void
		// Called after the user archives or deletes a persisted script. The
		// page should drop the selection (the script no longer exists in
		// the active graph) and refetch.
		onScriptRemoved?: (path: string) => void
		// Producers of the currently selected asset (script/flow paths that
		// write to it). Used by the runs panel to list job history. Pulled
		// from the graph by the parent page rather than re-derived here so
		// drafts stay consistent with what the canvas already shows.
		selectionProducers?: Array<{
			kind: 'script' | 'flow'
			path: string
			unsaved?: boolean
		}>
		// Pipeline-wide column-lineage graph (built by the parent page from the
		// resolved graph). Drives the transitive column-lineage trace shown for a
		// selected materialized asset.
		selectionColumnGraph?: ColumnLineageGraph
		// Whether the selected ducklake asset's schema can evolve (whole-table
		// `replace` producer). Forwarded to the Schema tab: version history when
		// true, a single fixed-schema view when false. Defaults to true (unknown).
		schemaCanEvolve?: boolean
		// Fork workspaces: data-environment state of the selected ducklake asset
		// ('fork' = materialized here, 'deferred' = reads the parent's current
		// data). Pulled from the graph node by the parent page, like
		// `schemaCanEvolve`. Undefined outside forks.
		selectionForkMaterialization?: 'fork' | 'deferred'
		// Producer-side facts for the editor's live schema-contract diagnostics
		// (ignore suppression + scd2 `_current` fallback), built by the page from
		// the resolved graph and forwarded to ScriptEditor.
		schemaContractContext?: SchemaContractGraphContext
		// Bumped by the parent after dispatching a run so the runs panel
		// re-fetches the listing immediately (rather than waiting on its
		// background poll tick).
		runsRefreshKey?: any
		// Job id of the most recently dispatched run. Forwarded to the
		// runs panel so that clicking play auto-selects the new run.
		runsPendingJobId?: string | undefined
		// Bubbled up from AssetRunsPanel when a watched run reaches a
		// terminal state. The pipeline page uses it to clear the
		// `activeRunnable` overlay (which animates edges around the
		// running script) once execution finishes.
		onRunCompleted?: () => void
		// Runnable currently executing — shared with the canvas. When it
		// matches one of the selected asset's producers, the preview
		// gets a "Recomputing…" banner so the user knows the rendered
		// snapshot is about to be replaced.
		activeRunnable?: { kind: 'script' | 'flow'; path: string } | undefined
		// Forwarded from ScriptEditor when the user hits the Test button —
		// gives the page a chance to mark the script as the active
		// runnable on the canvas (animates its edges) so Test and the
		// canvas Run button feel equivalent.
		onTestStateChange?: (running: boolean) => void
		// Counter pattern (cf. `requestRemoveSignal`) — bumped by the
		// page when the user clicks the canvas Run button on the
		// currently-open script, so we can route the dispatch through
		// ScriptEditor.runTest() and surface the running state in the
		// preview panel (logs/result/cancel) instead of just animating
		// the edges. Counter rather than boolean so back-to-back runs
		// re-fire the effect even if no other prop changed.
		requestRunSignal?: number
		// Folder-scoped non-editable prefix shown next to the suffix
		// editor when the user renames a draft (e.g. `f/<folder>/`). The
		// new path = pathPrefix + suffix.
		pathPrefix?: string
		// Called when the user renames a draft — the parent reseats the
		// path key in its drafts map and updates activeDraftPath. Returns
		// true on success; false on collision/validation failure so the
		// popover can keep itself open and surface the error inline.
		onDraftPathChange?: (oldPath: string, newPath: string) => boolean | string
		// Bumped by the parent (e.g. from the runnable-node action menu) to
		// auto-open the archive/delete confirmation modal for the currently
		// loaded persisted script. No-ops while the pane is showing a draft
		// or is still loading. Counter pattern (not a boolean) so successive
		// triggers re-fire even if the modal was just closed.
		requestRemoveSignal?: number
		// Count of scripts that subscribe (via `// on …`) to assets that the
		// currently-edited script writes. Threaded into ScriptEditor's
		// `customUi.previewPanel` so the Test button renders a split-button
		// cascade option when > 0. The page computes this from the graph
		// edges + triggers + currently-open path.
		downstreamSubscribers?: number
		// Pipeline-only: set when the currently-open script is a valid
		// bounded-run start (schedule / manual root). Surfaces a "Run downstream
		// up to…" entry on the Test split's caret that enters the canvas
		// end-node pick mode rooted at this script. Undefined → no entry.
		onStartBoundedRun?: () => void
		// Sister to `requestRunSignal`. When bumped, the bridge calls
		// `ScriptEditor.runTest({ cascade: true })` — used by the canvas
		// runnable menu's "Run + trigger N downstream" item when the chosen
		// script happens to be the currently-open one.
		requestRunCascadeSignal?: number
		// Bumped by the page when the user clicks a `data_upload` source node.
		// Scrolls the run form's S3 file input into view and pulses a highlight
		// so the UI-first "upload a file to run" affordance is obvious — the
		// input lives below the editor and is otherwise easy to miss.
		focusUploadSignal?: number
		// Page mode. Outside 'edit' the pane is read-only: no editor, no
		// rename/save/archive — scripts render through PipelineScriptView
		// (code + runs), and the only dispatch is the legitimate
		// data-upload run below.
		mode?: PipelineMode
		// Switch the page to edit mode focused on the current selection.
		// Only passed for users who can edit (not operators) — gates the
		// header's Edit button.
		onRequestEdit?: () => void
		// The selected script is a data-upload pipeline entry (page derives
		// this from the displayed graph's triggers) — show the run form in
		// the read-only script view.
		canRunByPath?: boolean
		// Legitimate run dispatch owned by the page: runScriptByPath with
		// NO _wmill_skip_asset_dispatch, so the backend asset-trigger
		// dispatcher cascades downstream for real.
		onRunByPath?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		// Run the open script AND its downstream closure with the form args (dev
		// preview only — the client orchestrates the chain). Unset on the deployed
		// pane, whose single run already cascades via the backend dispatcher.
		onRunCascadeByPath?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		// Persisted run-form args for the open data-upload entry (its staged
		// S3Object), so re-opening the node restores the picked file.
		runFormInitialArgs?: Record<string, any>
		// Emitted when the run form's args (or validity) change (read-only
		// PipelineScriptView OR the edit-mode ScriptEditor test form), so the page
		// can persist a data-upload entry's staged input (drives node readiness).
		// `isValid` is the full-schema validity, not just "file present".
		onRunFormArgsChange?: (path: string, args: Record<string, any>, isValid: boolean) => void
	}
	let {
		selection,
		draftScript,
		resolveLocalScript,
		localScriptsVersion,
		workspace,
		onclose,
		onHide,
		onDraftSaved,
		onPersistedSaved,
		onDiscard,
		onAnnotationsChange,
		onAssetsChange,
		onContentChange,
		onDraftPersist,
		onScriptRenamed,
		onScriptRemoved,
		selectionProducers = [],
		selectionColumnGraph,
		schemaCanEvolve = true,
		selectionForkMaterialization = undefined,
		schemaContractContext = undefined,
		runsRefreshKey,
		runsPendingJobId,
		onRunCompleted,
		activeRunnable,
		onTestStateChange,
		requestRunSignal,
		pathPrefix = '',
		onDraftPathChange,
		requestRemoveSignal,
		downstreamSubscribers = 0,
		onStartBoundedRun,
		requestRunCascadeSignal,
		focusUploadSignal,
		mode = 'edit',
		onRequestEdit,
		canRunByPath = false,
		onRunByPath,
		onRunCascadeByPath,
		runFormInitialArgs,
		onRunFormArgsChange
	}: Props = $props()

	let readOnly = $derived(mode !== 'edit')

	// Root element of the pane — used to scope the S3-input lookup for the
	// data_upload focus effect.
	let paneEl: HTMLElement | undefined = $state(undefined)

	// Held ref to ScriptEditor so we can route a canvas-side Run dispatch
	// through .runTest() — gives the test panel logs/result/cancel for
	// runs initiated from the graph, not just the in-pane Test button.
	let scriptEditorRef: { runTest: (opts?: { cascade?: boolean }) => Promise<unknown> } | undefined =
		$state(undefined)
	// Run-bridge: fire `ScriptEditor.runTest()` exactly once per distinct
	// signal-counter value the page sends (canvas Run targeting the open
	// script), on the editor that actually corresponds to the run target.
	//   1. A plain node selection or a save must never trigger a run — they
	//      bump no counter, so the last-handled guard keeps the effect inert
	//      even though selecting/saving reassigns `scriptEditorRef`.
	//   2. The Run button selects the node first, so the ScriptEditor is
	//      usually still (re)mounting when the counter bumps. We must NOT
	//      run yet — and must NOT mark the signal handled — so the effect
	//      re-runs and fires once the editor settles.
	//   3. `scriptRes` is stale-while-revalidate: right after selection the
	//      PREVIOUS script's ScriptEditor is still mounted. Firing then runs
	//      the wrong script. `editorReadyForTarget` gates on the mounted
	//      editor being settled on the run-target path before we fire.
	let lastHandledRunSig = 0
	let lastHandledCascadeSig = 0
	let runTargetPath = $derived.by(() =>
		isDraft
			? draftScript?.path
			: selection?.kind === 'runnable' && selection.runnable_kind === 'script'
				? selection.path
				: undefined
	)
	let editorReadyForTarget = $derived.by(
		() =>
			!!scriptEditorRef &&
			runTargetPath !== undefined &&
			script?.path === runTargetPath &&
			(isDraft || (!scriptRes.loading && scriptRes.current?.path === runTargetPath))
	)
	$effect(() => {
		const sig = requestRunSignal
		if (sig === undefined || sig === 0 || sig === lastHandledRunSig) return
		if (!editorReadyForTarget) return
		lastHandledRunSig = sig
		void scriptEditorRef?.runTest()
	})
	$effect(() => {
		// Cascade-explicit sister signal: same pattern, but forces the
		// runTest call to opt into the asset-trigger cascade for this run.
		const sig = requestRunCascadeSignal
		if (sig === undefined || sig === 0 || sig === lastHandledCascadeSig) return
		if (!editorReadyForTarget) return
		lastHandledCascadeSig = sig
		void scriptEditorRef?.runTest({ cascade: true })
	})

	// Focus-the-upload bridge: when the page bumps `focusUploadSignal` (user
	// clicked a data_upload source node), pulse the test panel's border once
	// (Test button + args form + logs) so the UI-first "upload a file to run"
	// affordance is obvious. Same gating as the run bridge — wait for the
	// editor to settle on the target so the panel is mounted.
	//
	// The pulse is a throwaway fixed-position overlay appended to <body> rather
	// than a class on the panel itself: the split panes clip descendant
	// outlines/box-shadows via `overflow`, and a tall splitter paints over the
	// panel's lower edge. A `position: fixed` element at the panel's rect (clamped
	// to the viewport) escapes both — it can't be clipped by ancestors and, at a
	// high z-index, paints above everything, so all four borders stay visible
	// even when the panel content overflows below the fold.
	// Read-only sibling of `editorReadyForTarget`: no ScriptEditor mounts
	// outside edit mode, so readiness is just "the pane settled on the
	// target script" — the pulse then finds the [data-run-form] section
	// PipelineScriptView renders instead of the test panel.
	let viewReadyForTarget = $derived.by(
		() =>
			readOnly &&
			runTargetPath !== undefined &&
			script?.path === runTargetPath &&
			(isDraft || (!scriptRes.loading && scriptRes.current?.path === runTargetPath))
	)
	let lastHandledFocusSig = 0
	$effect(() => {
		const sig = focusUploadSignal
		if (sig === undefined || sig === 0 || sig === lastHandledFocusSig) return
		if (!editorReadyForTarget && !viewReadyForTarget) return
		lastHandledFocusSig = sig
		requestAnimationFrame(() => {
			const target = paneEl?.querySelector(
				'[data-test-panel], [data-run-form]'
			) as HTMLElement | null
			if (!target) return
			const r = target.getBoundingClientRect()
			const m = 2
			const top = Math.max(r.top, 0) + m
			const bottom = Math.min(r.bottom, window.innerHeight) - m
			if (bottom <= top) return
			const ov = document.createElement('div')
			ov.className = 'wm-upload-pulse'
			ov.style.cssText = `position:fixed;left:${r.left + m}px;top:${top}px;width:${r.width - m * 2}px;height:${bottom - top}px;pointer-events:none;z-index:9999;border-radius:6px;`
			document.body.appendChild(ov)
			setTimeout(() => ov.remove(), 1500)
		})
	})

	// True when the script that writes to the currently-selected asset is
	// running right now. Drives the "Recomputing…" banner above the
	// preview and pairs with `onRunCompleted` (which bumps
	// `previewRefreshKey`) to reload the preview when the run finishes.
	let producerRunning = $derived(
		!!activeRunnable &&
			selectionProducers.some(
				(p) => p.kind === activeRunnable!.kind && p.path === activeRunnable!.path
			)
	)

	// Bound from ScriptEditor — populated by inferAssets on every code
	// change. Forwarded to the page so the canvas can re-derive write
	// edges as the user edits the body (e.g. renaming a CREATE TABLE
	// target updates the output asset node in real time).
	let liveBodyAssets = $state<AssetWithAltAccessType[] | undefined>(undefined)
	// Body-inferred column lineage (DuckDB SQL AST), bound out of ScriptEditor
	// alongside `liveBodyAssets` and forwarded so the live graph can show
	// inferred column lineage on the edited script before it deploys.
	let liveColumnLineage = $state<ColumnLineage[] | undefined>(undefined)

	// Bumped when the runs panel reports a watched job has reached a
	// terminal state. Drives S3FilePreview's refreshKey so the preview
	// re-checks existence after a producer run finishes — moves the
	// "not yet materialized" empty state to the actual preview without
	// requiring the user to re-click the asset.
	let previewRefreshKey = $state(0)

	// When `draftScript` is provided we bypass the fetch entirely and edit
	// it locally; saving calls ScriptService.createScript to deploy it.
	let scriptRes = resource(
		[() => workspace, () => selection, () => draftScript, () => localScriptsVersion],
		async ([ws, sel, draft]) => {
			if (draft) return undefined
			if (!sel || sel.kind !== 'runnable' || sel.runnable_kind !== 'script') return undefined
			// Local-dev: serve the working-tree script (no deployed row exists).
			const local = await resolveLocalScript?.(sel.path)
			if (local) return local
			return await ScriptService.getScriptByPath({ workspace: ws, path: sel.path })
		}
	)

	// Local mutable copy. For drafts: seeded once from the incoming prop
	// (subsequent typing stays in `script`, not `draftScript`). For fetched
	// scripts: reset whenever the resource yields new data.
	let script = $state<Script | undefined>(undefined)
	$effect.pre(() => {
		if (draftScript) {
			script = structuredClone($state.snapshot(draftScript) as Script)
			return
		}
		const fresh = scriptRes.current
		if (fresh) {
			script = structuredClone($state.snapshot(fresh) as Script)
			return
		}
		// Resource is loading with no cached value. If we already hold the
		// script for the selected path — e.g. a draft the user just deployed,
		// whose now-persisted version is being fetched — keep showing it so the
		// editor doesn't blink through the loading state (the "pane closed then
		// reopened" flicker). Only blank when the selection points elsewhere.
		const selPath =
			selection?.kind === 'runnable' && selection.runnable_kind === 'script'
				? selection.path
				: undefined
		if (script && selPath && script.path === selPath) return
		script = undefined
	})

	// Data-upload capture (edit mode): the ScriptEditor test form binds `args`, so
	// mirror it up to the page — that stages a data-upload entry's uploaded /
	// entered input, driving the node's green "ready" state and seeding the
	// whole-pipeline run. Reset per script in a pre-effect (before the keyed
	// ScriptEditor remounts) so switching nodes never leaks one node's input into
	// the next, seeding from the page's persisted staging so re-opening a node
	// restores its input. Guarded on the path so a staging round-trip
	// (emit → page → runFormInitialArgs) doesn't re-seed and loop. The read-only
	// branch uses PipelineScriptView's own onArgsChange instead.
	let args = $state<Record<string, any>>({})
	let argsSeedPath: string | undefined = undefined
	$effect.pre(() => {
		const p = script?.path
		if (p === argsSeedPath) return
		argsSeedPath = p
		args = runFormInitialArgs ? structuredClone($state.snapshot(runFormInitialArgs)) : {}
	})
	// Bound out of ScriptEditor's test-form SchemaForm — full-schema validity of
	// the edit-mode run form, so the page's readiness check sees whether every
	// required field (not just the S3 file) is satisfied.
	let runFormIsValid = $state(true)
	$effect(() => {
		if (readOnly || !script) return
		onRunFormArgsChange?.(script.path, $state.snapshot(args), runFormIsValid)
	})

	// Persist draft edits back to the parent's drafts Map on transitions
	// (selection change, pane close), not on every keystroke — a per-key
	// sync triggered drafts → activeDraft → draftScript → re-clone → emit
	// loops that exceed Svelte's effect depth limit. The closure captures
	// the active script; cleanup reads its (mutated-by-typing) content
	// AND the latest inferred body writes right before the next clone
	// replaces it. liveBodyAssets is read inside cleanup (not body) so
	// the $effect doesn't re-fire when ScriptEditor updates the inferred
	// asset list.
	$effect(() => {
		// Read-only modes never mutate the draft (no editor mounted), so
		// there's nothing to persist back — and emitting here would clash
		// with the page's mode-switch overlay reset.
		if (!script || readOnly) return
		// Persisted scripts persist-back too — unsaved edits get promoted
		// to a draft by the parent rather than silently dropped (e.g. on a
		// switch to view mode). Wait for the pristine copy so the cleanup
		// can tell edits from no-ops.
		const isDraftRun = draftScript != undefined
		const origAtRegister = isDraftRun ? undefined : scriptRes.current
		if (!isDraftRun && !origAtRegister) return
		const captured = script
		return () => {
			if (!isDraftRun) {
				// Prefer the freshest fetch when it's still this script
				// (cleanup reads are untracked): after the pane's own Save +
				// refetch the buffer matches the new deployed content and
				// nothing is emitted. On a selection switch the resource may
				// already be loading the next script — fall back to the
				// pristine copy captured at registration.
				const latest = scriptRes.current
				const orig = latest && latest.path === captured.path ? latest : origAtRegister
				if (!orig || orig.path !== captured.path) return
				if ((captured.content ?? '') === (orig.content ?? '')) return
				// The effect can re-run mid-save — after createScript resolved
				// but before the refetch lands — with `orig` still holding the
				// pre-deploy version. The buffer isn't "unsaved edits" then, it
				// IS the new deployed head; emitting would resurrect it as a
				// phantom draft identical to what was just deployed.
				if (
					deployedFromPane?.path === captured.path &&
					(captured.content ?? '') === deployedFromPane.content
				)
					return
			}
			const writes = (liveBodyAssets ?? [])
				.filter((a) => {
					const t = a.access_type ?? a.alt_access_type
					return t === 'w' || t === 'rw'
				})
				.map((a) => ({ kind: a.kind, path: a.path }))
			onDraftPersist?.(captured.path, {
				content: captured.content ?? '',
				writes,
				script: isDraftRun ? undefined : (structuredClone($state.snapshot(captured)) as Script)
			})
		}
	})

	let saving = $state(false)
	// What this pane last deployed, so the persist-back cleanup can tell "the
	// buffer equals the new deployed head" from real unsaved edits (plain
	// variable: only read inside the untracked cleanup).
	let deployedFromPane: { path: string; content: string } | undefined = undefined
	let isDraft = $derived(draftScript != undefined)

	// Single trash-bin button opens one modal that exposes both Archive
	// (always available) and Delete permanently (admin-only). Archive is
	// the default destructive action; delete is reserved for irrecoverable
	// cleanup. Modal is lifted from ConfirmationModal's structure but
	// hand-rolled here because we need *two* confirm buttons, not one.
	let removeOpen = $state(false)
	let removing = $state(false)
	let canHardDelete = $derived(!!($userStore?.is_admin || $userStore?.is_super_admin))

	// React to the parent's remove-signal counter and pop the same modal
	// the in-pane trash button uses. Skipped for drafts (the parent calls
	// `onDiscard` directly there). Counter pattern (not boolean) so
	// successive triggers re-fire even if the modal was just closed without
	// acting. The intent is held in `pendingRemoveSignal` until the script
	// is loaded — otherwise a kebab→Delete on a non-selected node consumes
	// the signal before the pane has the script, and the modal never opens.
	// The first observation of the counter is treated as the baseline (not
	// a fresh request) — the parent ships an initial 0 and clicking a
	// script must NOT trigger the delete modal.
	let lastRemoveSignal = $state<number | undefined>(undefined)
	let pendingRemoveSignal = $state<number | undefined>(undefined)
	$effect(() => {
		if (requestRemoveSignal === undefined) return
		const baseline = lastRemoveSignal === undefined
		if (!baseline && requestRemoveSignal === lastRemoveSignal) return
		lastRemoveSignal = requestRemoveSignal
		if (baseline) return
		if (isDraft) return
		pendingRemoveSignal = requestRemoveSignal
	})
	$effect(() => {
		if (pendingRemoveSignal === undefined) return
		if (!script || !script.hash) return
		pendingRemoveSignal = undefined
		removeOpen = true
	})

	async function archive() {
		if (!script || !script.hash) return
		removing = true
		try {
			await ScriptService.archiveScriptByHash({ workspace, hash: script.hash })
			sendUserToast('Script archived')
			const removedPath = script.path
			removeOpen = false
			onScriptRemoved?.(removedPath)
		} catch (e: any) {
			sendUserToast(`Could not archive: ${e.body ?? e.message}`, true)
		} finally {
			removing = false
		}
	}

	async function deleteScript() {
		if (!script || !script.path) return
		removing = true
		try {
			// Path-based delete drops every version at the path. The hash-based
			// equivalent only soft-deletes the active row; for a "delete" the
			// user is asking for, the path-based call is the right semantic.
			await ScriptService.deleteScriptByPath({ workspace, path: script.path })
			sendUserToast('Script deleted')
			const removedPath = script.path
			removeOpen = false
			onScriptRemoved?.(removedPath)
		} catch (e: any) {
			sendUserToast(`Could not delete: ${e.body ?? e.message}`, true)
		} finally {
			removing = false
		}
	}

	// Live-parse the editor buffer so the page can render unsaved schedule /
	// trigger nodes on the canvas. Parsed TS output mirrors the Rust
	// `parse_pipeline_annotations` used at deploy time.
	let liveAnnotations = $derived<PipelineAnnotations>(
		script
			? parsePipelineAnnotations(script.content ?? '')
			: {
					inPipeline: false,
					triggerAssets: [],
					nativeTriggers: [],
					dataTests: [],
					columnLineage: [],
					macros: false,
					useLibs: [],
					muteAssets: [],
					muteAll: false
				}
	)
	// `// macros` library: the defined signatures for the strip above the
	// source. Regex-light extraction (display only; deploy runs the strict
	// Rust parser).
	let macroDefs = $derived(
		script && liveAnnotations.macros ? extractDraftMacros(script.content ?? '') : []
	)
	$effect(() => {
		// Live-overlay emits are an edit-mode concern: in read-only modes
		// the loaded content is the deployed version (already covered by
		// the page's prefetch caches) and the page resets these overlays
		// on mode switch — re-emitting would immediately repopulate them.
		if (readOnly) return
		onAnnotationsChange?.(script?.path, liveAnnotations)
	})
	$effect(() => {
		if (readOnly) return
		onAssetsChange?.(script?.path, liveBodyAssets ?? [], liveColumnLineage)
	})
	$effect(() => {
		if (readOnly) return
		onContentChange?.(script?.path, script?.content ?? '')
	})

	async function save() {
		if (!script) return
		saving = true
		try {
			script.schema = script.schema ?? emptySchema()
			try {
				const result = await inferArgs(script.language, script.content, script.schema as Schema)
				;(script as any).auto_kind = result?.auto_kind || undefined
				script.has_preprocessor = result?.has_preprocessor ?? false
			} catch {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}
			const newHash = await ScriptService.createScript({
				workspace,
				requestBody: {
					...script,
					language: script.language,
					description: script.description ?? '',
					// Brand-new drafts have no prior hash (buildDraft seeds ''
					// — falsy counts as parentless); anything carrying one (a
					// deployed script, or a draft promoted from unsaved edits
					// to a deployed script) chains off it.
					parent_hash: script.hash ? String(script.hash) : undefined,
					// Let the backend resolve the parent to the current head for
					// this path (atomically, under an advisory lock) instead of
					// rejecting a stale parent_hash with a "lineage must be
					// linear" error — the pane is opened from a graph snapshot
					// that can fall behind the deployed head between renders.
					auto_parent: true,
					is_template: false,
					tag: script.tag,
					kind: script.kind as Script['kind'] | undefined,
					lock: undefined,
					// Inferred body assets — without this the backend's
					// `clear_static_asset_usage` + reinsert at deploy time
					// would write zero asset rows for the script (the spread
					// `...script` doesn't carry inferAssets results, those
					// live on `liveBodyAssets`). Result on a deployed script
					// with a CREATE TABLE / writeS3File body: no edges from
					// the script to its outputs in the asset graph until the
					// user re-selects it client-side. Same shape the full
					// /scripts/edit page sends.
					assets: (liveBodyAssets ?? []) as any
				}
			})
			// Chain the NEXT save off the version we just created. Without this
			// a second save re-sends the now-stale `parent_hash`, which the
			// backend rejects as a lineage fork ("no 2 scripts can have the
			// same parent").
			if (typeof newHash === 'string' && newHash) script.hash = newHash
			deployedFromPane = { path: script.path, content: script.content ?? '' }
			sendUserToast(`Saved ${script.path}`)
			// Authoritative save-time schema-contract check (pipelines gap #2b):
			// warn-only, post-commit. Fire-and-forget — must never gate the save.
			notifyContractWarnings(workspace, script.language as ScriptLang, script.content)
			if (isDraft) {
				onDraftSaved?.(script.path)
			} else {
				await scriptRes.refetch()
				onPersistedSaved?.(script.path)
			}
		} catch (e: any) {
			const msg = String(e?.body ?? e?.message ?? e)
			// The deployed head moved off our `parent_hash` since we opened it —
			// someone else (or another tab) deployed in between. Offer an
			// explicit resolution instead of a cryptic lineage error.
			if (/lineage must be linear|same parent_hash/i.test(msg)) {
				conflictOpen = true
			} else {
				sendUserToast(`Save failed: ${msg}`, true)
			}
		} finally {
			saving = false
		}
	}

	// --- Deploy conflict resolution (another deploy landed since we opened) ---
	let conflictOpen = $state(false)
	let resolvingConflict = $state(false)
	async function overwriteWithMine() {
		if (!script) return
		resolvingConflict = true
		try {
			// Re-base onto the current head so the retry is a linear deploy on
			// top of theirs (our content wins).
			const latest = await ScriptService.getScriptByPath({ workspace, path: script.path })
			script.hash = String(latest.hash)
			conflictOpen = false
			await save()
		} catch (e: any) {
			sendUserToast(`Could not resolve conflict: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			resolvingConflict = false
		}
	}
	async function viewLatestVersion() {
		if (!script) return
		resolvingConflict = true
		try {
			const latest = await ScriptService.getScriptByPath({ workspace, path: script.path })
			// Replace the editor buffer with the latest deployed version. The
			// user's unsaved edits are dropped — surfaced clearly in the modal.
			script = structuredClone($state.snapshot(latest) as Script)
			conflictOpen = false
			sendUserToast('Loaded the latest deployed version')
		} catch (e: any) {
			sendUserToast(`Could not load the latest version: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			resolvingConflict = false
		}
	}

	let isScriptView = $derived(
		isDraft || (selection?.kind === 'runnable' && selection.runnable_kind === 'script')
	)

	// A persisted script with no edits vs its loaded version is already at
	// its latest save point — hide Save for it. Drafts are never "saved"
	// (always show Create). Unknown/loading → treat as dirty so Save isn't
	// hidden when there might be changes.
	let atLatestSavePoint = $derived.by(() => {
		if (isDraft || !script) return false
		const orig = scriptRes.current
		if (!orig) return false
		return (
			script.content === orig.content &&
			(script.tag ?? '') === (orig.tag ?? '') &&
			(script.summary ?? '') === (orig.summary ?? '') &&
			(script.description ?? '') === (orig.description ?? '') &&
			JSON.stringify(script.schema ?? null) === JSON.stringify(orig.schema ?? null)
		)
	})

	// Suffix editor for the draft-path popover. Seeded from the current
	// path each time the popover opens so the user starts with what they
	// see, not stale state from an earlier rename.
	let draftPathSuffix = $state('')
	let draftPathError = $state<string | undefined>(undefined)
	let draftPathInput: HTMLInputElement | undefined = $state(undefined)

	function suffixOf(fullPath: string): string {
		return fullPath.startsWith(pathPrefix) ? fullPath.slice(pathPrefix.length) : fullPath
	}

	async function openDraftPathEditor() {
		draftPathSuffix = script ? suffixOf(script.path) : ''
		draftPathError = undefined
		await tick()
		draftPathInput?.focus()
		draftPathInput?.select()
	}

	function confirmDraftPath(close: () => void) {
		if (!script) return
		const suffix = draftPathSuffix.trim()
		if (!suffix) {
			draftPathError = 'Path cannot be empty'
			return
		}
		const newPath = pathPrefix + suffix
		if (newPath === script.path) {
			close()
			return
		}
		const result = onDraftPathChange?.(script.path, newPath)
		if (result === true || result === undefined) {
			script.path = newPath
			draftPathError = undefined
			close()
		} else if (typeof result === 'string') {
			draftPathError = result
		} else {
			draftPathError = 'Path already in use'
		}
	}
</script>

<div class="flex flex-col h-full bg-surface" bind:this={paneEl}>
	<div
		class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10 whitespace-nowrap"
	>
		<div class="flex items-center gap-2 min-w-0">
			{#if isDraft && script}
				{@const draftScriptPath = script.path}
				<Code2 size={16} class="shrink-0 text-tertiary" />
				{#if onDraftPathChange && !readOnly}
					<!-- Inline rename popover for drafts. The persisted-script
					     branch uses SummaryPathDisplay which round-trips through
					     updateItemPathAndSummary; drafts have no server row yet,
					     so we just rekey the parent's drafts map locally. -->
					<Popover
						placement="bottom-start"
						contentClasses="p-3"
						usePointerDownOutside
						on:openChange={(e) => {
							if (e.detail) openDraftPathEditor()
						}}
					>
						{#snippet trigger()}
							<button
								type="button"
								class="flex flex-col min-w-0 text-left px-2 py-1 rounded-md hover:bg-surface-hover transition-colors group"
								title="Edit draft path"
							>
								<span
									class="text-3xs uppercase tracking-wide text-tertiary flex items-center gap-1"
								>
									Draft pipeline script
									<Pencil size={9} class="opacity-0 group-hover:opacity-60 transition-opacity" />
								</span>
								<span class="text-xs font-mono truncate" title={draftScriptPath}
									>{draftScriptPath}</span
								>
							</button>
						{/snippet}
						{#snippet content({ close })}
							<div class="flex flex-col gap-2 w-[420px]">
								<span class="text-2xs font-normal text-secondary">Path</span>
								<div
									class="flex items-stretch border rounded-md bg-surface overflow-hidden focus-within:ring-2 focus-within:ring-blue-400"
								>
									{#if pathPrefix}
										<span
											class="flex items-center px-2 bg-surface-secondary text-tertiary text-sm font-mono border-r select-none"
										>
											{pathPrefix}
										</span>
									{/if}
									<input
										bind:this={draftPathInput}
										bind:value={draftPathSuffix}
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.preventDefault()
												confirmDraftPath(close)
											} else if (e.key === 'Escape') {
												e.preventDefault()
												close()
											}
										}}
										class="flex-1 min-w-0 px-2 py-1.5 text-sm font-mono bg-transparent focus:outline-none"
										placeholder="my_script"
									/>
								</div>
								{#if draftPathError}
									<span class="text-2xs text-red-500">{draftPathError}</span>
								{/if}
								<div class="flex justify-end">
									<Button
										variant="accent"
										unifiedSize="sm"
										disabled={!draftPathSuffix.trim()}
										onClick={() => confirmDraftPath(close)}
									>
										Rename
									</Button>
								</div>
							</div>
						{/snippet}
					</Popover>
				{:else}
					<div class="flex flex-col min-w-0">
						<span class="text-3xs uppercase tracking-wide text-tertiary">
							Draft pipeline script
						</span>
						<span class="text-xs font-mono truncate" title={script.path}>{script.path}</span>
					</div>
				{/if}
			{:else if selection?.kind === 'asset'}
				<AssetGenericIcon
					assetKind={selection.asset_kind}
					size="16px"
					class="shrink-0 text-blue-600 dark:text-blue-400"
				/>
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">
						{formatAssetKind({ kind: selection.asset_kind })}
					</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'script'}
				<Code2 size={16} class="shrink-0 text-tertiary" />
				{#if readOnly}
					<div class="flex flex-col min-w-0">
						<span class="text-3xs uppercase tracking-wide text-tertiary">
							{script?.summary ? script.summary : 'Script'}
						</span>
						<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
					</div>
				{:else}
					<!-- Mirrors the rename UX from /scripts/get/[hash]: clicking the
					     path/summary opens an inline popover that calls
					     updateItemPathAndSummary. We forward `onSaved` upward so
					     the parent page can repoint its selection and refetch the
					     graph. Bound to the local `script` state so summary edits
					     reflect immediately without waiting on the resource. -->
					<SummaryPathDisplay
						summary={script?.summary ?? ''}
						path={selection.path}
						labels={script?.labels ?? []}
						kind="script"
						onSaved={(newPath) => {
							const oldPath = selection?.kind === 'runnable' ? selection.path : ''
							if (script) {
								script.path = newPath
							}
							onScriptRenamed?.(oldPath, newPath)
						}}
					/>
				{/if}
			{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow'}
				<GitBranch size={16} class="shrink-0 text-tertiary" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Flow</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			{#if readOnly && onRequestEdit}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Pencil }}
					onclick={onRequestEdit}
					title="Open this script in the pipeline editor"
				>
					Edit
				</Button>
			{/if}
			{#if !readOnly && isDraft && onDiscard}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: Trash2 }}
					onclick={onDiscard}
					iconOnly
					title="Discard draft"
				/>
			{/if}
			<!-- Action order, left → right: trash, external link, save, close.
			     Save sits closest to Close so the primary commit action is
			     anchored at the right edge of the bar; trash lives at the
			     far left so destructive ops are visually separated from
			     navigation/commit. Mirrors the draft Discard placement. -->
			{#if !readOnly && !isDraft && isScriptView && script?.hash}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: Trash2 }}
					onclick={() => (removeOpen = true)}
					iconOnly
					title="Archive or delete"
				/>
			{/if}
			{#if !readOnly && !isDraft && selection?.kind === 'runnable'}
				<Button
					variant="subtle"
					unifiedSize="sm"
					href={selection.runnable_kind === 'flow'
						? `${base}/flows/edit/${selection.path}`
						: `${base}/scripts/edit/${selection.path}`}
					target="_blank"
					startIcon={{ icon: ExternalLink }}
					iconOnly
					title="Open in full editor"
				/>
			{/if}
			{#if !readOnly && isScriptView && script && !atLatestSavePoint}
				{@const isCreate = isDraft && !script?.hash}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					onclick={save}
					disabled={saving}
					title={isCreate
						? 'Deploy this new script to the workspace'
						: 'Deploy your changes to this script'}
				>
					{saving ? 'Deploying…' : 'Deploy'}
				</Button>
			{/if}
			{#if onHide}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: PanelRightClose }}
					onclick={onHide}
					iconOnly
					title="Hide panel"
				/>
			{/if}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: X }}
				onclick={onclose}
				iconOnly
				title="Close"
			/>
		</div>
	</div>

	<div class="flex-1 min-h-0 relative">
		{#if selection?.kind === 'asset' && !isDraft}
			<!-- Vertical split: top pane is kind-specific (S3 has a content
			     preview; other kinds fall back to a navigational hint until
			     they grow their own previews); bottom pane is the runs panel,
			     which is generic — runs are keyed by producer script path,
			     not by asset kind, so it's useful for every asset. -->
			<Splitpanes horizontal class="!h-full">
				<Pane size={55} minSize={20}>
					<div class="flex flex-col h-full">
						{#if producerRunning}
							<!-- The asset's producing script is executing right now;
							     the snapshot below is stale until the run finishes,
							     at which point onRunCompleted bumps previewRefreshKey
							     and the preview component refetches automatically. -->
							<div
								class="shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs bg-blue-50 dark:bg-blue-950/40 border-b border-blue-200 dark:border-blue-900/60 text-blue-700 dark:text-blue-300"
							>
								<Loader2 size={12} class="animate-spin shrink-0" />
								<span class="truncate">
									Recomputing — preview will refresh when {activeRunnable?.path} finishes
								</span>
							</div>
						{/if}
						<div class="flex-1 min-h-0 relative">
							{#if selection.asset_kind === 's3object'}
								<S3FilePreview
									fileKey={selection.path}
									showMetadata
									class="h-full"
									refreshKey={previewRefreshKey}
								/>
							{:else if selection.asset_kind === 'datatable'}
								<DataTablePreview
									path={selection.path}
									class="h-full"
									refreshKey={previewRefreshKey}
								/>
							{:else if selection.asset_kind === 'ducklake'}
								<!-- Key on path so switching ducklake assets resets the panel's
								     selected snapshot / tab instead of carrying state across. -->
								{#key selection.path}
									<div class="flex flex-col h-full overflow-auto">
										{#if selectionForkMaterialization === 'deferred'}
											<!-- Fork data environment: this asset has no fork copy yet;
											     jobs here read the PARENT's live table through a defer
											     view. Partition grid / history below reflect the (empty)
											     fork namespace, hence the explicit callout. -->
											<div
												class="shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs bg-amber-50 dark:bg-amber-950/40 border-b border-amber-200 dark:border-amber-900/60 text-amber-700 dark:text-amber-300"
											>
												<ArrowUpRight size={12} class="shrink-0" />
												<span class="truncate">
													Deferred to parent workspace — reads its current data (not
													snapshot-pinned). Materialize this node in the fork to iterate on it.
												</span>
											</div>
										{:else if selectionForkMaterialization === 'fork'}
											<div
												class="shrink-0 flex items-center gap-2 px-3 py-1.5 text-2xs bg-emerald-50 dark:bg-emerald-950/40 border-b border-emerald-200 dark:border-emerald-900/60 text-emerald-700 dark:text-emerald-300"
											>
												<GitFork size={12} class="shrink-0" />
												<span class="truncate">
													Materialized in this fork — reads and writes use the fork's isolated
													namespace; the parent workspace's table is untouched.
												</span>
											</div>
										{/if}
										{#if selectionColumnGraph && assetColumnNodes(selectionColumnGraph, selection.asset_kind, selection.path).length > 0}
											<div class="border-b shrink-0">
												<ColumnLineageTrace
													graph={selectionColumnGraph}
													assetKind={selection.asset_kind}
													assetPath={selection.path}
													targetLabel={selection.path}
												/>
											</div>
										{/if}
										<div class="flex-1 min-h-0">
											<DucklakeAssetPanel path={selection.path} {workspace} {schemaCanEvolve} />
										</div>
									</div>
								{/key}
							{:else}
								<div class="p-3 text-xs text-secondary">
									No inline preview yet for {selection.asset_kind}. Use the producer/consumer arrows
									in the graph to navigate. Runs of the upstream script are below.
								</div>
							{/if}
						</div>
					</div>
				</Pane>
				<Pane size={45} minSize={20}>
					<AssetRunsPanel
						producers={selectionProducers}
						refreshKey={runsRefreshKey}
						pendingJobId={runsPendingJobId}
						onRunCompleted={() => {
							previewRefreshKey += 1
							onRunCompleted?.()
						}}
					/>
				</Pane>
			</Splitpanes>
		{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow' && !isDraft}
			<div class="p-3 text-xs text-secondary">
				{#if readOnly}
					Flows have no inline view here. Open <a
						class="text-blue-600 hover:underline"
						href="{base}/flows/get/{selection.path}">the flow page</a
					> for details and runs.
				{:else}
					Flows are not editable inline. Use the open-in-editor button above.
				{/if}
			</div>
		{:else if !isDraft && scriptRes.loading && !script}
			<div class="absolute inset-0 flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={16} class="animate-spin" />
				<span class="text-xs">Loading script…</span>
			</div>
		{:else if !isDraft && scriptRes.error}
			<div class="p-3 text-xs text-red-500">
				Failed to load: {scriptRes.error.message}
			</div>
		{:else if script && readOnly}
			<!-- Read-only modes: no Monaco/ScriptEditor (operators are
			     backend-blocked from previews anyway) — highlighted source,
			     the legitimate data-upload run form when applicable, and the
			     same runs panel as the asset branch. -->
			{#key script.path}
				<div class="flex flex-col h-full">
					{@render macroLibStrip()}
					<div class="flex-1 min-h-0">
						<PipelineScriptView
							{script}
							{isDraft}
							canRun={canRunByPath}
							onRun={onRunByPath}
							onRunCascade={onRunCascadeByPath}
							downstreamCount={downstreamSubscribers}
							{runsRefreshKey}
							{runsPendingJobId}
							initialArgs={runFormInitialArgs}
							onArgsChange={onRunFormArgsChange}
							onRunCompleted={() => {
								previewRefreshKey += 1
								onRunCompleted?.()
							}}
						/>
					</div>
				</div>
			{/key}
		{:else if script}
			<!-- Key on path alone, NOT hash: deploying a draft or re-saving turns
			     hash ''→<hash> for the *same* script, and a hash-based key would
			     needlessly remount the editor (a visible flash) even though the
			     content is unchanged. Path uniquely identifies the script (and its
			     language never changes in place), so switching scripts still
			     remounts while a same-script save doesn't. History is disabled
			     here, so there's no revert-to-old-hash case needing a reset. -->
			{#key script.path}
				<div class="flex flex-col h-full">
					{@render macroLibStrip()}
					<div class="flex-1 min-h-0 relative">
						<ScriptEditor
							bind:this={scriptEditorRef}
							showCaptures={false}
							noSyncFromGithub
							requireValidAssets
							{schemaContractContext}
							lang={script.language}
							path={script.path}
							tag={script.tag}
							fixedOverflowWidgets={false}
							previewLayout="bottom"
							customUi={{
								previewPanel: {
									disableHistory: true,
									disableTracing: true,
									disableTriggerCaptures: true,
									disableJsonView: true,
									// Drop the full args column (most pipeline scripts take
									// no inputs), render the LogPanel full-width with
									// logs|result side by side, and float the Test/Cancel
									// button onto the editor band above. But when the
									// script *does* declare inputs (e.g. a partitioned
									// script that needs a `partition` arg to run), show a
									// compact SchemaForm between the Test button and the
									// logs so the run can actually be parameterised.
									hideArgs: true,
									argsAboveLogs: true,
									logsResultSideBySide: true,
									downstreamSubscribers,
									onBoundedRun: onStartBoundedRun,
									// Selecting a script node should immediately show
									// "what happened last time it ran" — pulling the
									// latest top-level completed job into the preview
									// pane is far more useful than an empty placeholder.
									loadLastRunOnMount: true
								}
							}}
							bind:code={script.content}
							bind:schema={script.schema}
							bind:assets={liveBodyAssets}
							bind:inferredColumnLineage={liveColumnLineage}
							{onTestStateChange}
							bind:args
							onIsValidChange={(v) => (runFormIsValid = v)}
						/>
					</div>
				</div>
			{/key}
		{/if}
	</div>
</div>

{#snippet macroLibStrip()}
	{#if macroDefs.length > 0}
		<!-- `// macros` library: the defined signatures, always visible above the
		     source so consumers can see what the lib provides at a glance. -->
		<div class="shrink-0 px-3 py-1.5 border-b bg-surface-secondary flex items-start gap-2">
			<SquareFunction size={12} class="shrink-0 mt-0.5 text-violet-600 dark:text-violet-400" />
			<div class="flex flex-wrap gap-x-3 gap-y-0.5 text-2xs font-mono text-secondary min-w-0">
				<!-- Index-keyed: a live buffer can transiently hold two defs with
				     the same name mid-edit, which would crash a name-keyed each. -->
				{#each macroDefs as m, i (i)}
					<span class="truncate" title={m.is_table ? 'table macro' : 'scalar macro'}>
						{m.name}({m.params}){m.is_table ? ' → table' : ''}
					</span>
				{/each}
			</div>
		</div>
	{/if}
{/snippet}

{#if removeOpen}
	<!-- Single combined modal. Archive is the always-available default;
	     Delete shows only for workspace/super admins because it's
	     irreversible from the UI. Layout mirrors ConfirmationModal so the
	     two confirmation surfaces feel consistent. -->
	<div
		transition:fade={{ duration: 100 }}
		class="fixed top-0 bottom-0 left-0 right-0 z-[9999]"
		role="dialog"
	>
		<div class="fixed inset-0 bg-gray-500 bg-opacity-75"></div>
		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl sm:my-8 sm:w-full sm:max-w-lg sm:p-6"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-800/50"
						>
							<AlertTriangle class="text-red-500 dark:text-red-400" />
						</div>
						<div class="ml-4 flex-1">
							<h3 class="text-lg font-medium text-primary">Archive or delete this script?</h3>
							<div class="mt-2 text-sm text-secondary flex flex-col gap-2">
								<p>
									<span class="font-mono text-xs">{script?.path ?? ''}</span>
								</p>
								<p>
									<span class="font-medium">Archive</span> removes the script from the pipeline graph
									and stops every trigger from firing it (schedule, webhook, asset event, …). History
									is preserved and the script can be unarchived later.
								</p>
								{#if canHardDelete}
									<p>
										<span class="font-medium text-red-700 dark:text-red-400">Delete</span> permanently
										drops every version and draft at this path. Restorable by a workspace admin from
										the trashbin within 3 days, otherwise irrecoverable.
									</p>
								{/if}
							</div>
						</div>
					</div>
					<div class="flex items-center gap-2 flex-row-reverse mt-4">
						<Button
							disabled={removing}
							onclick={archive}
							variant="accent"
							color="red"
							size="sm"
							destructive
						>
							{#if removing}
								<Loader2 class="animate-spin" />
							{/if}
							<span class="min-w-20">Archive</span>
						</Button>
						<Button
							disabled={removing}
							onclick={() => (removeOpen = false)}
							variant="default"
							size="sm"
						>
							Cancel
						</Button>
						{#if canHardDelete}
							<!-- Sits at the far left of the row (flex-row-reverse) to
							     visually separate the irreversible option from the
							     safer Archive. No flex-1 wrapper — the button sizes
							     to its label. -->
							<Button
								disabled={removing}
								onclick={deleteScript}
								variant="contained"
								color="red"
								size="sm"
								destructive
							>
								Delete permanently
							</Button>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

{#if conflictOpen}
	<!-- Deploy conflict: the path's deployed head moved off our parent_hash
	     since we opened it (another user / tab deployed in between). -->
	<div
		transition:fade={{ duration: 100 }}
		class="fixed top-0 bottom-0 left-0 right-0 z-[9999]"
		role="dialog"
	>
		<div class="fixed inset-0 bg-gray-500 bg-opacity-75"></div>
		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class="relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl sm:my-8 sm:w-full sm:max-w-lg sm:p-6"
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-amber-100 dark:bg-amber-800/50"
						>
							<AlertTriangle class="text-amber-500 dark:text-amber-400" />
						</div>
						<div class="ml-4 flex-1">
							<h3 class="text-lg font-medium text-primary"
								>This script changed since you opened it</h3
							>
							<div class="mt-2 text-sm text-secondary flex flex-col gap-2">
								<p>
									<span class="font-mono text-xs">{script?.path ?? ''}</span> was deployed by someone
									else (or another tab) while you were editing, so saving on top of your version would
									fork its lineage.
								</p>
								<p>
									<span class="font-medium">Keep my version</span> deploys your changes on top of
									the latest.
									<span class="font-medium">View latest</span> loads the newly-deployed version, discarding
									your unsaved edits.
								</p>
							</div>
						</div>
					</div>
					<div class="flex items-center gap-2 flex-row-reverse mt-4">
						<Button
							disabled={resolvingConflict}
							onclick={overwriteWithMine}
							variant="accent"
							size="sm"
						>
							{#if resolvingConflict}
								<Loader2 class="animate-spin" />
							{/if}
							<span class="min-w-20">Keep my version</span>
						</Button>
						<Button
							disabled={resolvingConflict}
							onclick={() => (conflictOpen = false)}
							variant="default"
							size="sm"
						>
							Cancel
						</Button>
						<Button
							disabled={resolvingConflict}
							onclick={viewLatestVersion}
							variant="contained"
							size="sm"
						>
							View latest version
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Border-color pulse for the throwaway overlay appended to <body> when the
	   user clicks a data_upload source node (hence :global). border-box keeps
	   the 3px border inside the overlay's measured rect, so it traces the test
	   panel's bounds exactly. */
	:global(.wm-upload-pulse) {
		animation: wm-upload-pulse 1.4s ease-out 1;
		border: 3px solid transparent;
		box-sizing: border-box;
	}
	@keyframes wm-upload-pulse {
		0% {
			border-color: rgba(217, 70, 239, 0);
		}
		20% {
			border-color: rgba(217, 70, 239, 0.85);
		}
		100% {
			border-color: rgba(217, 70, 239, 0);
		}
	}
</style>
