<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { page } from '$app/state'
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import PipelineGraphEditor from '$lib/components/assets/AssetGraph/PipelineGraphEditor.svelte'
	import {
		useActiveRunnableIds,
		isActiveEvent
	} from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import type {
		PipelineEvent,
		RunnableRunState,
		RunStatus
	} from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import { usePipelineHistory } from '$lib/components/assets/AssetGraph/pipelineHistory.svelte'
	import PipelineActivityPanel from '$lib/components/assets/AssetGraph/PipelineActivityPanel.svelte'
	import PipelinePickerModal from '$lib/components/assets/AssetGraph/PipelinePickerModal.svelte'
	import {
		extractWrites,
		extractReads,
		type AssetWithAltAccessType
	} from '$lib/components/assets/lib'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind,
		PipelineMode
	} from '$lib/components/assets/AssetGraph/types'
	import PipelineModeToggle from '$lib/components/assets/AssetGraph/PipelineModeToggle.svelte'
	import MacroExplorerDrawer from '$lib/components/assets/AssetGraph/MacroExplorerDrawer.svelte'
	import { parsePipelineAnnotations } from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import {
		buildColumnGraph,
		type ColumnLineageGraph
	} from '$lib/components/assets/AssetGraph/columnLineageGraph'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import { buildSchemaContractContext } from '$lib/components/assets/AssetGraph/schemaContracts'
	import {
		computeDownstreamClosure,
		computeInducedSchedule,
		assetProducers
	} from '$lib/components/assets/AssetGraph/graphTraversal'
	import { runCascade, runSelection } from '$lib/components/assets/AssetGraph/cascadeOrchestrator'
	import { DATA_ASSET_KINDS } from '$lib/components/assets/AssetGraph/cascadeRun'
	import {
		boundedSet,
		buildLineageDag,
		buildLineageDownstreamMap,
		isScriptNode,
		nonAutorunTriggerScripts,
		reachableCutting,
		scriptNodeId,
		scriptsOf,
		validFromStarts,
		validStarts
	} from '$lib/components/assets/AssetGraph/boundedCascade'
	import {
		diffDeployedGraph,
		extractCascadeFacts,
		formatDrift,
		type CascadeFacts
	} from '$lib/components/assets/AssetGraph/deployGraphDiff'
	import {
		generatePipelineDraft,
		autoOutputAsset,
		type PipelineOutputKind,
		type DraftTriggerSource
	} from '$lib/components/assets/AssetGraph/pipelineTemplates'
	import {
		createPipelineAiHelpers,
		type PipelineDraft
	} from '$lib/components/assets/AssetGraph/pipelineAiHelpers'
	import { PipelineEditorState } from '$lib/components/assets/AssetGraph/pipelineEditorState.svelte'
	import {
		createPipelineRecording,
		finalizePipelineRecording
	} from '$lib/components/recording/pipelineRecording.svelte'
	import type { PipelineRecording } from '$lib/components/recording/types'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { onMount, tick, untrack } from 'svelte'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import {
		AlertTriangle,
		ArrowLeft,
		ChevronDown,
		Circle,
		Download,
		Folder,
		FolderSearch,
		History,
		Loader2,
		NetworkIcon,
		Play,
		RefreshCw,
		Save,
		SquareFunction,
		Target,
		Telescope
	} from 'lucide-svelte'
	import {
		JobService,
		OpenAPI,
		ScheduleService,
		ScriptService,
		type AssetKind,
		type Script,
		type ScriptLang
	} from '$lib/gen'
	import { resource } from 'runed'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type { Schema } from '$lib/common'
	import { beforeNavigate, goto } from '$app/navigation'
	import { fade } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { inferArgs, inferAssets } from '$lib/infer'
	import PipelineTriggerEditors from '$lib/components/assets/AssetGraph/PipelineTriggerEditors.svelte'

	// Variables and resources are declarative config, not pipeline assets —
	// they're hub-shaped (referenced by most runnables) and would swamp the
	// layout without adding lineage information.
	const DATA_KINDS = DATA_ASSET_KINDS

	let folder = $derived(page.params.folder as string)

	// Externalized editor state (drafts, live overlays, selection), shared with
	// the in-session pipeline preview via PipelineEditorState. Referenced as
	// `pe.*` throughout; the persistence / graph / run logic below stays here.
	const pe = new PipelineEditorState()

	// The in-app folder switcher navigates same-route (`/pipeline/<other>`), which
	// reuses this page component — nothing remounts. Mirror the session preview's
	// retarget guard: reset the editor state on a folder change so folder A's drafts
	// don't display under B (and aren't autosaved into B's bundle), and so
	// hydratedFromDb flips back to false and B's draft bundle re-hydrates.
	$effect(() => {
		const f = folder
		untrack(() => {
			if (pe.folder !== f) {
				if (pe.folder !== undefined) {
					pe.reset()
					// Re-scope the Global chat's pipeline prompt to the new folder. The
					// helper methods already read the reactive folder, but the system
					// message string was built for the old one and is only rebuilt when
					// Global mode is reconfigured — so rebuild it here.
					aiChatManager.rebuildGlobalSystemMessage()
				}
				pe.folder = f
			}
		})
	})

	// Page mode, URL-addressable via `?mode=`. No param = view (the
	// default): deployed-only graph focused on past/live executions.
	// Operators are clamped to view — the derived ignores a hand-typed
	// `?mode=edit` rather than redirecting.
	let isOperator = $derived($userStore?.operator ?? false)
	let urlMode = $derived(page.url.searchParams.get('mode'))
	let mode = $derived<PipelineMode>(isOperator ? 'view' : urlMode === 'edit' ? 'edit' : 'view')
	function setMode(m: PipelineMode, opts?: { replace?: boolean }) {
		// Switching edit→view re-surfaces the activity feed: drop the edit-mode
		// selection / open draft / hidden-pane state so view opens on Activity
		// rather than a stale details pane (mirrors toggleActivity's show path).
		if (m === 'view' && mode === 'edit') {
			pe.selection = undefined
			pe.activeDraftPath = undefined
			panelHidden = false
			pe.liveAnnotations = EMPTY_LIVE_ANNOTATIONS
		}
		const url = new URL(page.url)
		if (m === 'view') url.searchParams.delete('mode')
		else url.searchParams.set('mode', m)
		// Same-pathname navigation — the drafts leave-guard skips it.
		goto(url, { replaceState: opts?.replace ?? false, keepFocus: true, noScroll: true })
	}
	// View-mode drafts overlay: "what View will show once the drafts are
	// deployed". A view variant rather than a third mode — ephemeral (not
	// URL-addressable: drafts live in this browser's localStorage, so a
	// shared link couldn't reproduce it anyway).
	let includeDrafts = $state(false)

	// Asset-kind → syntax-prefix for `// on <ref>` reconstruction. Mirrors
	// ASSET_KINDS in backend/parsers/windmill-parser/src/asset_parser.rs.
	const ASSET_PREFIX: Record<AssetKind, string> = {
		s3object: 's3://',
		resource: '$res:',
		ducklake: 'ducklake://',
		datatable: 'datatable://',
		volume: 'volume://'
	}

	// Path-input split for the insert menu: a read-only `f/<folder>/` chip
	// on the left the user can't delete, plus an editable suffix seeded
	// with a placeholder name.
	let pathPrefix = $derived(`f/${folder}/`)
	const DEFAULT_PATH_SUFFIX = 'new_pipeline_script'
	// In-flight drafts keyed by script path. Multiple can coexist — clicking
	// + repeatedly creates additional drafts, each with its own random
	// output asset, and they all render on the graph simultaneously.
	// Saving removes a draft from the map; closing the pane keeps it so the
	// user can come back to it.
	// The draft shape + draft Map + activeDraftPath now live in the shared
	// PipelineEditorState (`pe`). `Draft` aliases the store's type so existing
	// annotations keep working.
	type Draft = PipelineDraft

	// Splitpane sizing + details-pane-open derivation live inside PipelineGraphEditor.
	// Explicit hide flag — keeps `selection` / `activeDraftPath` intact
	// so re-opening the pane re-uses them. Mirrors AppEditor's
	// hideRightPanel/showRightPanel pattern.
	let panelHidden = $state(false)
	// View mode's idle pane is the activity feed; once a node is selected
	// (or the pane hidden) the only ways back were the pane's X and the
	// floating hide toggle — neither is named. The top-bar Activity toggle
	// is the explicit affordance: shows the feed from any state, hides the
	// pane when the feed is already showing.
	let activityShowing = $derived(
		mode === 'view' && !panelHidden && pe.selection == undefined && pe.activeDraftPath == undefined
	)
	function toggleActivity() {
		if (activityShowing) {
			panelHidden = true
		} else {
			pe.selection = undefined
			pe.activeDraftPath = undefined
			panelHidden = false
			// Same reset as the pane's close button — clears the live
			// annotation overlay of whichever script was open.
			pe.liveAnnotations = EMPTY_LIVE_ANNOTATIONS
		}
	}

	// Workspace-macro explorer drawer (all `// macros` libraries + their
	// signatures/bodies). "Open" on a group selects the lib node when it's on
	// this canvas; a lib living in another folder opens its script page.
	let macroDrawer: MacroExplorerDrawer | undefined = $state()
	function openMacroLib(path: string) {
		const onCanvas = displayGraph?.runnables?.some(
			(r) => r.usage_kind === 'script' && r.path === path
		)
		if (onCanvas) {
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path }
			pe.activeDraftPath = undefined
			panelHidden = false
			focusPipelineNode(`script:${path}`)
		} else {
			window.open(`${base}/scripts/get/${path}`, '_blank')
		}
	}

	// Draft autosave (the data_pipeline DraftService bundle) lives inside
	// PipelineGraphEditor now; the route just supplies its path to the indicator.
	let pipelineDraftPath = $derived(`f/${folder}/data_pipeline`)

	// The live editor overlays (annotations / body assets / content for the open
	// script) now live in `pe`. The canonical "empty" literals stay here — they
	// also seed the no-overlay inputs to the deployed graph below.
	const EMPTY_LIVE_ASSETS = { scriptPath: undefined, assets: [] }
	const EMPTY_LIVE_ANNOTATIONS = {
		scriptPath: undefined,
		annotations: {
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
	}

	// Only-add cache of (script_path → body content) populated lazily by
	// `bodyFetchEffect` for every script in the current folder. We never
	// remove entries: stale keys (renamed-away, deleted) are simply ignored
	// at read time because the derived maps below only iterate paths that
	// appear in the current `g.runnables`. That self-cleaning property is
	// the whole reason for the refactor — no rename/delete cleanup needed.
	let bodiesByPath = $state<Map<string, string>>(new Map())
	// Sibling cache: the parsed asset usages from `inferAssets` (wasm), one
	// pass per body. Same only-add semantics as `bodiesByPath`.
	let inferredAssetsByPath = $state<Map<string, AssetWithAltAccessType[]>>(new Map())
	// Bumped on folder change so an in-flight prefetch sweep stops before
	// writing into the new folder's state.
	let bodyFetchGen = 0

	// Inferred write/read edges per script-in-graph. Derived from
	//   (a) the open pane's live overlay (`liveBodyAssets`) — current
	//       keystrokes for the script the user is editing right now, and
	//   (b) the prefetched assets cache for everyone else.
	// Iteration is gated on `graphRes.current.runnables`, so a path that
	// gets renamed / deleted disappears from the derived map as soon as
	// the refetch lands — no manual rekey, no phantom edges.
	// Single pass over the graph's scripts producing both the write- and
	// read-asset maps (they only differ by extractWrites vs extractReads over
	// the same `liveBodyAssets`-vs-cache asset source). Split into two derives
	// below so consumers can depend on one without invalidating on the other.
	let inferredAssetEdges = $derived.by(() => {
		const writes = new Map<string, Array<{ kind: AssetKind; path: string }>>()
		const reads = new Map<string, Array<{ kind: AssetKind; path: string }>>()
		const g = graphRes.current
		if (!g) return { writes, reads }
		const liveAssetsForPath = (path: string) =>
			pe.liveBodyAssets.scriptPath === path
				? pe.liveBodyAssets.assets
				: inferredAssetsByPath.get(path)
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			const assets = liveAssetsForPath(r.path)
			if (!assets) continue
			const w = extractWrites(assets)
			if (w.length > 0) writes.set(r.path, w)
			const rd = extractReads(assets)
			if (rd.length > 0) reads.set(r.path, rd)
		}
		return { writes, reads }
	})
	let inferredWritesByPath = $derived(inferredAssetEdges.writes)
	let inferredReadsByPath = $derived(inferredAssetEdges.reads)
	// Same derived shape for `// on kafka` etc. annotations. Live buffer
	// wins for the open script; everyone else is parsed from the
	// prefetched body content.
	let annotatedNativeKindsByPath = $derived.by(() => {
		const out = new Map<string, Set<NativeTriggerKind>>()
		const g = graphRes.current
		if (!g) return out
		const livePath = pe.liveAnnotations.scriptPath
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			let kinds: Set<NativeTriggerKind>
			if (r.path === livePath) {
				kinds = new Set(pe.liveAnnotations.annotations.nativeTriggers.map((n) => n.kind))
			} else {
				const body = bodiesByPath.get(r.path)
				if (!body) continue
				kinds = new Set(parsePipelineAnnotations(body).nativeTriggers.map((n) => n.kind))
			}
			if (kinds.size > 0) out.set(r.path, kinds)
		}
		return out
	})

	// Build a runnable Script from picked language / triggers / output.
	// Delegates to the shared template generator (pipelineTemplates.ts) so
	// the same logic is reachable from anywhere a draft is needed.
	function buildDraft(
		language: ScriptLang,
		scriptPath: string,
		triggers: DraftTriggerSource[],
		outputKind: PipelineOutputKind,
		output: { kind: AssetKind; path: string } | undefined,
		input: { kind: AssetKind; path: string } | undefined
	): Script {
		const content = generatePipelineDraft({
			language,
			outputKind,
			output,
			input,
			triggers
		})
		// Cast through unknown: the Script generated type has many readonly
		// deployment fields (hash, created_*) that we don't care about for a
		// local draft. The details pane only reads path/language/content/schema.
		return {
			hash: '',
			path: scriptPath,
			summary: '',
			description: '',
			content,
			schema: emptySchema(),
			is_template: false,
			extra_perms: {},
			language,
			kind: 'script',
			created_by: '',
			created_at: new Date().toISOString(),
			archived: false,
			deleted: false,
			starred: false
		} as unknown as Script
	}

	// Graph-id of the node the canvas should smoothly pan to. Set when a new
	// draft node is created; cleared on a timer once the relayout (the details
	// pane opening resizes the canvas, shifting every node) has settled so the
	// pan lands on the final position rather than a pre-resize one.
	let panToNodeId = $state<string | undefined>(undefined)
	let panToNodeTimer: ReturnType<typeof setTimeout> | undefined = undefined
	function focusPipelineNode(id: string) {
		panToNodeId = id
		if (panToNodeTimer) clearTimeout(panToNodeTimer)
		panToNodeTimer = setTimeout(() => {
			if (panToNodeId === id) panToNodeId = undefined
			panToNodeTimer = undefined
		}, 600)
	}

	function openMaterializerDraft(
		language: ScriptLang,
		scriptPath: string,
		triggers: DraftTriggerSource[],
		outputKind: PipelineOutputKind,
		input?: { kind: AssetKind; path: string },
		aiPrompt?: string
	) {
		const out = autoOutputAsset(outputKind, folder, language)
		const script = buildDraft(language, scriptPath, triggers, outputKind, out, input)
		// Write the new draft into the map (structural update so Svelte
		// re-derives graphWithDraft) and focus it in the details pane. When
		// the user picked `none`, `out` is undefined and the graph overlay
		// skips synthesizing a write edge.
		const next = new Map(pe.drafts)
		next.set(scriptPath, {
			localId: pe.newDraftLocalId(),
			script,
			outputAssets: out ? [out] : undefined
		})
		pe.drafts = next
		pe.activeDraftPath = scriptPath
		pe.selection = undefined

		// Follow the new node with a smooth pan. The id matches the runnable
		// node the canvas builds for a draft script (`script:<path>`).
		focusPipelineNode(`script:${scriptPath}`)

		// User filled the optional prompt on the path stage — fire off a
		// chat request so the AI bootstraps the body. The seeded template
		// (already in `script.content`) acts as scaffolding the AI
		// rewrites; language + chosen output + upstream input go in as
		// context so the model knows what to read from / write to.
		if (aiPrompt && aiPrompt.trim().length > 0) {
			void triggerAiBootstrap({ scriptPath, language, outputKind, input, out, prompt: aiPrompt })
		}
	}

	async function triggerAiBootstrap(args: {
		scriptPath: string
		language: ScriptLang
		outputKind: PipelineOutputKind
		input?: { kind: AssetKind; path: string }
		out?: { kind: AssetKind; path: string }
		prompt: string
	}) {
		// Wait one tick for AssetGraphDetailsPane to mount the ScriptEditor
		// and register itself with aiChatManager (scriptEditorApplyCode etc.).
		// Without this, openChat fires before the chat has a target editor.
		await tick()
		const lines: string[] = [args.prompt]
		lines.push('')
		lines.push(`Language: ${args.language}.`)
		if (args.input) {
			lines.push(`Read input from ${args.input.kind} \`${args.input.path}\`.`)
		}
		if (args.out) {
			lines.push(`Write output to ${args.out.kind} \`${args.out.path}\`.`)
		} else if (args.outputKind === 'none') {
			lines.push('No output asset is expected.')
		}
		const instructions = lines.join('\n')
		aiChatManager.openChat()
		aiChatManager.sendRequest({ instructions })
	}

	// ===================== AI chat pipeline integration =====================
	// The global AI chat (dev-gated) gains pipeline-building tools while this
	// editor is mounted, via the helpers registered below. AI mutations don't
	// deploy — they apply directly as unsaved drafts on the canvas (the same way
	// the flow/script editor applies AI edits), which the user then deploys. The
	// build/edit logic is shared verbatim with the in-session preview
	// (PipelineEditorView) via createPipelineAiHelpers.

	const pipelineAiHelpers = createPipelineAiHelpers({
		getFolder: () => folder,
		getWorkspace: () => $workspaceStore,
		getResolvedGraph: () => graphWithDraft,
		getDrafts: () => pe.drafts,
		setDrafts: (next) => (pe.drafts = next),
		newDraftLocalId: pe.newDraftLocalId,
		onForgetPath: (path) => forgetPath(path),
		onShowDrafts: () => (includeDrafts = true),
		onProposeNode: (path) => focusPipelineNode(`script:${path}`),
		ensureEditable: () => {
			// Auto-enter edit so AI changes are visible/actionable, unless the user
			// is an operator (no edit permission) — then refuse with a clear error.
			if (isOperator) {
				throw new Error('This pipeline is read-only for your role; AI edits are disabled.')
			}
			if (mode !== 'edit') setMode('edit')
		},
		onRunStarted: (jobId, path) => {
			activeRunnables.arm(`script:${path}`)
			runsPendingJobId = jobId
			runsRefreshKey++
			activeRunnable = { kind: 'script', path }
			activeRunnableJobId = jobId
		}
	})

	onMount(() => aiChatManager.setPipelineHelpers(pipelineAiHelpers))

	// Navigation guard state. `pendingNavigationUrl` holds the URL the user
	// tried to leave to so we can complete the navigation after they pick
	// "Save all" or "Discard all"; `bypassNavigationGuard` is the standard
	// SvelteKit pattern for "this navigation was already approved, don't
	// re-prompt".
	let pendingNavigationUrl = $state<URL | undefined>(undefined)
	let leaveModalOpen = $state(false)
	let bypassNavigationGuard = $state(false)
	let leaveSaving = $state(false)

	beforeNavigate((nav) => {
		if (bypassNavigationGuard) {
			bypassNavigationGuard = false
			return
		}
		if (pe.drafts.size === 0) return
		// `leave` covers tab close / hard reload / cross-origin nav. SvelteKit
		// turns a cancelled leave into a browser-native "Leave site?" prompt,
		// which we explicitly don't want — match the rest of the editors and
		// rely on debounced localStorage to keep drafts safe across reloads.
		if (nav.type === 'leave') return
		// Same-folder URL changes (search params, hash) shouldn't re-prompt;
		// the guard is for actually leaving the editor.
		if (nav.to && nav.from && nav.to.url.pathname === nav.from.url.pathname) {
			return
		}
		nav.cancel()
		pendingNavigationUrl = nav.to?.url
		leaveModalOpen = true
	})

	// Hard navigations (tab close / reload / cross-origin) don't go through
	// `beforeNavigate`, so there's no in-app modal in that case. We
	// intentionally skip the native `beforeunload` confirm here to match
	// the rest of the editors (FlowBuilder, ScriptEditor, AppEditor) — they
	// don't pop the browser-controlled "Leave site?" prompt either, and
	// drafts are debounce-persisted to localStorage on every keystroke, so
	// a hard close at most loses ~500 ms of typing. The in-app modal still
	// guards SvelteKit-handled navigation via `beforeNavigate` above.

	function leaveModalCancel() {
		leaveModalOpen = false
		pendingNavigationUrl = undefined
	}

	function leaveModalDiscard() {
		// Wipe every draft and the active selection so saved drafts and
		// stale active path don't bleed into the next page. localStorage
		// is overwritten by the persist effect on the next tick.
		pe.drafts = new Map()
		pe.activeDraftPath = undefined
		saveErrors = new Map()
		const target = pendingNavigationUrl
		leaveModalOpen = false
		pendingNavigationUrl = undefined
		if (target) {
			bypassNavigationGuard = true
			goto(target)
		}
	}

	async function leaveModalSaveAll() {
		leaveSaving = true
		await saveAllDrafts()
		leaveSaving = false
		// If anything failed, keep the modal closed and the user on the
		// page so they can deal with the failures via the bar's error
		// popover. Otherwise resume the navigation that triggered the
		// guard.
		if (pe.drafts.size === 0) {
			const target = pendingNavigationUrl
			leaveModalOpen = false
			pendingNavigationUrl = undefined
			if (target) {
				bypassNavigationGuard = true
				goto(target)
			}
		} else {
			leaveModalOpen = false
			pendingNavigationUrl = undefined
		}
	}

	// Bulk-save state. Errors are keyed by draft path so the error popover
	// can show one entry per failing draft alongside its message; successes
	// are removed from the drafts map as they land.
	let savingAll = $state(false)
	let saveErrors = $state<Map<string, string>>(new Map())

	// Post-deploy verification: capture the cascade-relevant facts (writes +
	// `// on` subscriptions) the resolved draft graph promises for `paths`.
	// MUST run before the deploy mutates `drafts` / the refetch replaces the
	// base graph — afterwards the prediction is gone.
	function predictCascadeFacts(paths: string[]): Map<string, CascadeFacts> {
		const m = new Map<string, CascadeFacts>()
		for (const p of paths) m.set(p, extractCascadeFacts(graphWithDraft, p))
		return m
	}
	// After graphRes.refetch(): the frontend preview and the backend deploy
	// derive edges independently (TS vs Rust parsers); if the deployed rows
	// lost anything the preview promised, say so NOW — the alternative is a
	// production cascade that silently doesn't fire.
	function reportDeployDrift(predicted: Map<string, CascadeFacts>) {
		const msg = formatDrift(diffDeployedGraph(predicted, graphRes.current ?? EMPTY_GRAPH))
		if (msg) sendUserToast(msg, true)
	}

	async function saveDraft(path: string, draft: Draft, ws: string): Promise<void> {
		const script = structuredClone($state.snapshot(draft.script) as Script)
		script.schema = script.schema ?? emptySchema()
		try {
			const result = await inferArgs(script.language, script.content, script.schema as Schema)
			;(script as any).auto_kind = result?.auto_kind || undefined
			script.has_preprocessor = result?.has_preprocessor ?? false
		} catch {
			// Inference failures don't block deploys (the same fallback the
			// per-pane save uses). The createScript call is the real
			// validation gate — if the body is broken it'll reject there.
		}
		// Re-infer the asset lineage from the CURRENT body. The draft's
		// `script.assets` is a snapshot that isn't refreshed on edit, so without
		// this a renamed/removed output (e.g. an old `CREATE TABLE foo`) would
		// be re-deployed as a phantom write edge and linger as an orphan asset.
		// Mirrors the per-pane save, which sends live-inferred assets.
		let assets: AssetWithAltAccessType[] = []
		try {
			const inferred = await inferAssets(script.language, script.content)
			if (inferred?.status !== 'error')
				assets = (inferred?.assets ?? []) as AssetWithAltAccessType[]
		} catch {
			// Same fallback as above — an unparsable body deploys with no
			// lineage rather than the stale snapshot.
		}
		await ScriptService.createScript({
			workspace: ws,
			requestBody: {
				...script,
				language: script.language,
				description: script.description ?? '',
				// Brand-new drafts have no parent (buildDraft seeds hash as ''
				// — treat any falsy hash as parentless, the backend rejects
				// an empty hex string); a draft promoted from unsaved edits
				// to a deployed script carries that script's hash and must
				// chain off it (createScript rejects a parentless deploy to
				// an occupied path).
				parent_hash: script.hash ? String(script.hash) : undefined,
				is_template: false,
				tag: script.tag,
				kind: script.kind as Script['kind'] | undefined,
				lock: undefined,
				// Freshly inferred above — overrides the stale snapshot carried
				// by `...script`, so the deployed lineage matches the body.
				assets: assets as any
			}
		})
	}

	async function saveAllDrafts() {
		if (!$workspaceStore || pe.drafts.size === 0 || savingAll) return
		savingAll = true
		const ws = $workspaceStore
		// The open pane's keystrokes live in `pe.liveContent` until the pane is
		// torn down — the drafts Map still holds the pre-edit snapshot. Deploy
		// what the user sees: fold the live buffer into its draft (same merge
		// the autosave bundle applies before persisting).
		const liveContentPath =
			pe.liveContent.scriptPath != undefined && pe.drafts.has(pe.liveContent.scriptPath)
				? pe.liveContent.scriptPath
				: undefined
		const entries = [...pe.drafts.entries()].map(([path, d]) => {
			if (path !== liveContentPath || d.script.content === pe.liveContent.content)
				return [path, d] as [string, Draft]
			return [path, { ...d, script: { ...d.script, content: pe.liveContent.content } }] as [
				string,
				Draft
			]
		})
		// Snapshot what the preview promises for every draft before anything
		// deploys — used to verify the persisted graph below.
		const predicted = predictCascadeFacts(entries.map(([p]) => p))
		const errors = new Map<string, string>()
		const savedPaths: string[] = []
		// Parallel — every createScript is independent. The backend handles
		// its own ordering for any cross-script lock writes; we just want
		// failures isolated per script so one bad body doesn't block the
		// other deploys.
		const results = await Promise.allSettled(
			entries.map(async ([path, d]) => {
				await saveDraft(path, d, ws)
				return path
			})
		)
		for (let i = 0; i < results.length; i++) {
			const r = results[i]
			const [path] = entries[i]
			if (r.status === 'fulfilled') {
				savedPaths.push(path)
			} else {
				const e: any = r.reason
				errors.set(path, e?.body ?? e?.message ?? String(e))
			}
		}
		// Drop the saved drafts from the map; failed ones stay so the user
		// can fix them and retry. Build the new map from the still-failing
		// entries to keep insertion order stable.
		if (savedPaths.length > 0) {
			const next = new Map<string, Draft>()
			for (const [k, v] of pe.drafts) {
				if (!savedPaths.includes(k)) next.set(k, v)
			}
			pe.drafts = next
			// If the open draft just got deployed, transfer the focus to
			// its now-persisted runnable so the pane stays on the same
			// script the user was editing — otherwise the pane closes,
			// the canvas re-fits, and the user has to re-find their
			// script after every save.
			if (pe.activeDraftPath && savedPaths.includes(pe.activeDraftPath)) {
				pe.selection = {
					kind: 'runnable',
					runnable_kind: 'script',
					path: pe.activeDraftPath
				}
				pe.activeDraftPath = undefined
			}
			await graphRes.refetch()
			// Verify only what actually deployed — failed drafts would
			// otherwise false-positive as "missing" rows.
			reportDeployDrift(new Map([...predicted].filter(([p]) => savedPaths.includes(p))))
		}
		saveErrors = errors
		savingAll = false
		if (savedPaths.length > 0 && errors.size === 0) {
			sendUserToast(`Saved ${savedPaths.length} draft${savedPaths.length === 1 ? '' : 's'}`)
		} else if (savedPaths.length > 0 && errors.size > 0) {
			sendUserToast(`Saved ${savedPaths.length}, ${errors.size} failed — see details`, true)
		} else if (errors.size > 0) {
			sendUserToast(`${errors.size} draft${errors.size === 1 ? '' : 's'} failed to save`, true)
		}
	}

	function discardDraft(path: string) {
		if (!pe.drafts.has(path)) return
		const next = new Map(pe.drafts)
		next.delete(path)
		pe.drafts = next
		forgetPath(path)
	}

	function clearSaveError(path: string) {
		if (!saveErrors.has(path)) return
		const next = new Map(saveErrors)
		next.delete(path)
		saveErrors = next
	}

	// "This path is gone" cleanup. The three big inferred-* / annotated-*
	// maps used to live here too, but they're now derived from
	// `bodiesByPath` × `g.runnables` — entries for missing paths drop out
	// implicitly when `g.runnables` no longer mentions them, so the only
	// things left to flush are the live overlays for the open pane + the
	// selection + per-path save errors. `bodiesByPath` keeps its entry
	// (only-add cache, harmless if stale).
	function forgetPath(path: string) {
		if (pe.activeDraftPath === path) pe.activeDraftPath = undefined
		if (pe.selection?.kind === 'runnable' && pe.selection.path === path) {
			pe.selection = undefined
		}
		if (pe.liveAnnotations.scriptPath === path) {
			pe.liveAnnotations = EMPTY_LIVE_ANNOTATIONS
		}
		if (pe.liveBodyAssets.scriptPath === path) {
			pe.liveBodyAssets = EMPTY_LIVE_ASSETS
		}
		if (pe.liveContent.scriptPath === path) {
			pe.liveContent = { scriptPath: undefined, content: '' }
		}
		clearSaveError(path)
	}

	// Rename a draft in place — re-key its entry in the drafts map and
	// repoint activeDraftPath. Returns false (or an error string) if the
	// new path collides with another draft so the dialog can keep itself
	// open and surface the conflict inline.
	function renameDraft(oldPath: string, newPath: string): boolean | string {
		if (oldPath === newPath) return true
		const draft = pe.drafts.get(oldPath)
		if (!draft) return 'Draft not found'
		if (pe.drafts.has(newPath)) return 'Another draft already uses this path'
		const next = new Map<string, Draft>()
		// Preserve insertion order: replace the entry at its original
		// position so the canvas / lists don't reshuffle on rename.
		for (const [k, v] of pe.drafts) {
			if (k === oldPath) {
				// Fold the open pane's live buffer in: the drafts Map only syncs
				// on pane teardown, and the rename both re-clones the pane from
				// this entry AND auto-deploys it — a stale snapshot here wipes
				// the user's unsaved keystrokes and ships pre-edit content.
				const content =
					pe.liveContent.scriptPath === oldPath ? pe.liveContent.content : v.script.content
				const updatedScript = { ...v.script, path: newPath, content }
				next.set(newPath, { ...v, script: updatedScript })
			} else {
				next.set(k, v)
			}
		}
		pe.drafts = next
		if (pe.activeDraftPath === oldPath) pe.activeDraftPath = newPath
		// Path-keyed live overlays: re-key for the renamed draft so the
		// graph stays consistent between the moment we mutate `drafts`
		// here and the next editor event that re-emits annotations /
		// asset usages with the new path. Without this re-key,
		// `resolveGraph` strips seeded triggers for the OLD path while
		// re-applying live overlays against the same OLD path — leaving
		// phantom edges that displace the + node off the top of the
		// graph and shuffle the layout.
		if (pe.liveAnnotations.scriptPath === oldPath) {
			pe.liveAnnotations = { ...pe.liveAnnotations, scriptPath: newPath }
		}
		if (pe.liveBodyAssets.scriptPath === oldPath) {
			pe.liveBodyAssets = { ...pe.liveBodyAssets, scriptPath: newPath }
		}
		if (pe.liveContent.scriptPath === oldPath) {
			pe.liveContent = { ...pe.liveContent, scriptPath: newPath }
		}
		// `inferredWritesByPath` / `inferredReadsByPath` /
		// `annotatedNativeKindsByPath` are derived from `g.runnables` ×
		// the body caches, so a rename naturally re-derives them when the
		// drafts loop in `resolveGraph` swaps to the new path — no
		// per-mutation rekey needed.
		// Errors are keyed by path — re-key the entry so a previously
		// failed draft keeps its error visible against the new path
		// after rename.
		if (saveErrors.has(oldPath)) {
			const nextErrors = new Map(saveErrors)
			const msg = nextErrors.get(oldPath)!
			nextErrors.delete(oldPath)
			nextErrors.set(newPath, msg)
			saveErrors = nextErrors
		}
		// Auto-deploy the rename. Without this, native triggers can't be
		// attached to the renamed script (the trigger row's script_path
		// needs to point at a real script) and the user has to remember
		// a separate "save" step. Backend `update_triggers_script_path`
		// already cascades the path change across every trigger table on
		// script create, so any triggers previously attached to the old
		// path follow the rename automatically.
		void deployRenamedDraft(newPath)
		return true
	}

	// Per-draft deploy queue. Each draft (keyed by stable `localId`) has at
	// most one deploy in flight; concurrent renames append to a single
	// `queuedPath` slot so we only ever fire one extra deploy at the
	// latest target. `lastDeployedPath` lets the runner archive any
	// intermediate path it deployed before the user's final rename
	// settled — otherwise a double-rename leaves a phantom script at the
	// intermediate path.
	const deployQueue = new Map<
		string,
		{ inflight: boolean; queuedPath: string | undefined; lastDeployedPath: string | undefined }
	>()

	function deployRenamedDraft(path: string) {
		const draft = pe.drafts.get(path)
		if (!draft) return
		const localId = draft.localId
		let state = deployQueue.get(localId)
		if (state?.inflight) {
			// Coalesce: only the latest target matters. The runner picks
			// it up when the current deploy completes.
			state.queuedPath = path
			return
		}
		state = state ?? { inflight: false, queuedPath: undefined, lastDeployedPath: undefined }
		state.inflight = true
		state.queuedPath = undefined
		deployQueue.set(localId, state)
		void runDeployQueue(localId, path)
	}

	async function runDeployQueue(localId: string, initialPath: string) {
		let path = initialPath
		const state = deployQueue.get(localId)!
		try {
			while (true) {
				if (!$workspaceStore) break
				const draft = pe.drafts.get(path)
				if (!draft) break
				try {
					await saveDraft(path, draft, $workspaceStore)
					// Archive the previously-deployed intermediate path (if
					// any) — a double-rename otherwise leaves it as an
					// orphan script visible on the canvas as a "deployed"
					// runnable that the user never intended to keep.
					const prev = state.lastDeployedPath
					if (prev && prev !== path) {
						try {
							await ScriptService.archiveScriptByPath({
								workspace: $workspaceStore,
								path: prev
							})
						} catch (archiveErr) {
							// Non-fatal: surface a warning so the user
							// knows to clean up manually. The fresh deploy
							// at the new path still landed.
							const msg =
								(archiveErr as any)?.body ?? (archiveErr as any)?.message ?? String(archiveErr)
							sendUserToast(
								`Renamed to "${path}" but couldn't archive the old "${prev}": ${msg}`,
								true
							)
						}
					}
					state.lastDeployedPath = path
					// Drop the now-deployed draft from the local map only
					// if no newer rename was queued during the save; if a
					// queued path is waiting, the next loop iteration will
					// pick it up and we keep the draft live.
					if (!state.queuedPath) {
						const nextDrafts = new Map(pe.drafts)
						nextDrafts.delete(path)
						pe.drafts = nextDrafts
						if (pe.activeDraftPath === path) {
							pe.selection = { kind: 'runnable', runnable_kind: 'script', path }
							pe.activeDraftPath = undefined
						}
						if (saveErrors.has(path)) {
							const nextErrors = new Map(saveErrors)
							nextErrors.delete(path)
							saveErrors = nextErrors
						}
						await graphRes.refetch()
					}
				} catch (e: any) {
					const msg = e?.body ?? e?.message ?? String(e)
					sendUserToast(`Could not deploy rename to "${path}": ${msg}`, true)
					const nextErrors = new Map(saveErrors)
					nextErrors.set(path, msg)
					saveErrors = nextErrors
				}
				// Pick up the next queued rename, if any.
				const next = state.queuedPath
				if (!next || next === path) break
				path = next
				state.queuedPath = undefined
			}
		} finally {
			state.inflight = false
			// On the rare path where the draft is also gone (deployed +
			// no queued path), drop the slot to keep the map bounded.
			if (!pe.drafts.has(path) && !state.queuedPath) {
				deployQueue.delete(localId)
			}
		}
	}

	// Currently-open draft shape (if any) — fed into the details pane.
	let activeDraft = $derived(pe.activeDraftPath ? pe.drafts.get(pe.activeDraftPath) : undefined)

	// Path of the script currently open in the details pane (draft or
	// persisted selection), used wherever run-routing / overlay logic needs
	// "the one script the user is editing right now".
	let openScriptPath = $derived(
		pe.activeDraftPath ??
			(pe.selection?.kind === 'runnable' && pe.selection.runnable_kind === 'script'
				? pe.selection.path
				: undefined)
	)

	// Entering edit mode with a selection whose path has a draft (e.g. a
	// deployed script whose unsaved edits were promoted on the switch to
	// view) re-opens the draft, not the stale deployed version — same
	// routing a node click gets in handleCanvasSelect.
	$effect(() => {
		if (mode !== 'edit') return
		if (
			pe.selection?.kind === 'runnable' &&
			pe.selection.runnable_kind === 'script' &&
			pe.drafts.has(pe.selection.path)
		) {
			pe.activeDraftPath = pe.selection.path
			pe.selection = undefined
		}
	})
	// Symmetric demotion: view mode shows the deployed truth, so an open
	// promoted draft (it has a deployed version — non-empty hash; buildDraft
	// seeds brand-new drafts with '') hands the pane to its persisted script
	// on the switch to view. Brand-new drafts have nothing deployed to show
	// and stay open read-only.
	$effect(() => {
		if (mode !== 'view') return
		const d = activeDraft
		if (d && d.script.hash) {
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: d.script.path }
			pe.activeDraftPath = undefined
		}
	})

	// The details pane's live callbacks (annotations / assets / content / persist)
	// now live on `pe` as stable arrow fields — pass `pe.handleX` straight through.

	// Canvas callbacks, named so the prop refs stay stable across re-renders
	// (same rationale as the live-callback handlers above) and so the
	// template can gate them per-mode with simple ternaries — the canvas
	// hides each affordance when its callback is undefined.
	function handleCanvasSelect(s: AssetGraphSelection | undefined) {
		// Clicking a node while the pane is explicitly hidden is a request
		// to see that node — unhide. Background clicks (s == undefined)
		// keep the hidden state.
		if (s != undefined) {
			panelHidden = false
		}
		// In edit mode, clicking a draft runnable node re-opens it in the
		// pane; clicking anything else selects it normally and detaches from
		// any active draft (drafts stay overlaid until saved or discarded).
		// View mode never routes to drafts — a deployed script with a
		// promoted draft is on the view graph too, and view shows the
		// deployed truth (the pane has no editor to show a draft with).
		if (
			mode === 'edit' &&
			s &&
			s.kind === 'runnable' &&
			s.runnable_kind === 'script' &&
			pe.drafts.has(s.path)
		) {
			pe.activeDraftPath = s.path
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = s
		}
	}
	function handleAddScriptForAsset(
		asset: { kind: AssetKind; path: string },
		language: ScriptLang,
		scriptPath: string,
		outputKind: PipelineOutputKind,
		aiPrompt?: string
	) {
		const ref = `${ASSET_PREFIX[asset.kind]}${asset.path}`
		openMaterializerDraft(
			language,
			scriptPath,
			[{ kind: 'asset', ref }],
			outputKind,
			{
				kind: asset.kind,
				path: asset.path
			},
			aiPrompt
		)
	}
	function handleAddPipelineScript(
		language: ScriptLang,
		scriptPath: string,
		source: { kind: NativeTriggerKind; path: string | undefined },
		outputKind: PipelineOutputKind,
		aiPrompt?: string
	) {
		openMaterializerDraft(language, scriptPath, [source], outputKind, undefined, aiPrompt)
	}
	function handleRunnableMenuRemove(info: {
		runnable_kind: 'script' | 'flow'
		path: string
		unsaved?: boolean
	}) {
		// Drafts: drop the local entry immediately — the existing onDiscard
		// pathway already handles this without a confirm modal. Persisted:
		// select the script (so the pane loads it), then bump the
		// remove-signal counter so the pane opens its archive/delete modal.
		if (info.unsaved) {
			discardDraft(info.path)
			return
		}
		if (info.runnable_kind !== 'script') return
		pe.activeDraftPath = undefined
		pe.selection = { kind: 'runnable', runnable_kind: 'script', path: info.path }
		requestRemoveSignal++
	}
	async function handleRunProducer(producer: {
		kind: 'script' | 'flow'
		path: string
		unsaved?: boolean
		cascade?: boolean
	}): Promise<string | undefined> {
		// Saved scripts go through runScriptByPath; drafts have no DB row
		// yet, so dispatch to runScriptPreview with the locally-cached
		// content/language. Keeping this dispatch in the page (rather than
		// the canvas or AssetNode) so the asset-graph components stay
		// stateless wrt drafts. Bump runsRefreshKey on success so the runs
		// panel picks up the new job immediately — its background poll only
		// kicks in for already-listed in-flight jobs.
		if (!$workspaceStore || producer.kind !== 'script') return undefined
		// Start the folder-scoped poll so the downstream asset-trigger
		// cascade (jobs not launched here) lights up its edges as it fans
		// out. Pass the launched id so the catch-up pulse doesn't re-flash
		// it (the page already animates it zero-latency via `activeRunnable`).
		activeRunnables.arm(`${producer.kind}:${producer.path}`)
		// Cascade default: same as the Test button — `cascade` undefined /
		// false skips the asset-trigger dispatch via
		// `_wmill_skip_asset_dispatch`; explicit `true` lets the dispatch
		// fire normally. The asset-node affordance still passes `undefined`
		// (legacy callers), which we treat as "skip" for consistency.
		const cascade = producer.cascade === true
		// Run + downstream over a chain with drafts in it: the backend
		// dispatcher can't see drafts (no DB rows), so the page orchestrates
		// the whole closure client-side. Deployed-only chains fall through
		// to the production dispatcher below — the dev run then tests the
		// real cascade machinery, not a simulation of it.
		if (cascade) {
			const hasDraftInChain =
				producer.unsaved === true ||
				computeDownstreamClosure(graphWithDraft, producer.path).nodes.some((p) => pe.drafts.has(p))
			if (hasDraftInChain) {
				return await runDraftAwareCascade(producer.path)
			}
		}
		// If the producer being run is the script currently edited in the
		// pane, route through ScriptEditor's Test path — the test panel then
		// shows logs/result and the user can cancel from there. Same UX as
		// hitting the Test button directly.
		if (openScriptPath === producer.path) {
			if (cascade) requestRunCascadeSignal++
			else requestRunSignal++
			return undefined
		}
		const skipArg = cascade ? {} : { _wmill_skip_asset_dispatch: true }
		let jobId: string | undefined
		if (producer.unsaved) {
			const draft = pe.drafts.get(producer.path)
			if (!draft?.script.content || !draft.script.language) return undefined
			jobId = await JobService.runScriptPreview({
				workspace: $workspaceStore,
				requestBody: {
					content: draft.script.content,
					language: draft.script.language,
					path: producer.path,
					args: { ...skipArg }
				}
			})
		} else {
			jobId = await JobService.runScriptByPath({
				workspace: $workspaceStore,
				path: producer.path,
				requestBody: { ...skipArg }
			})
		}
		if (jobId) {
			runsPendingJobId = jobId
			runsRefreshKey++
			activeRunnable = { kind: producer.kind, path: producer.path }
			// Track this job so the effect above releases the hint when it
			// finishes — there's no editor Test callback for an unselected
			// node's run.
			activeRunnableJobId = jobId
		}
		return jobId
	}

	// Legitimate run for view mode (data-upload entry points): a
	// plain runScriptByPath with NO _wmill_skip_asset_dispatch and no
	// client-side orchestration — the backend asset-trigger dispatcher does
	// the real downstream fan-out, and the folder poll animates the hops.
	// This is also the one dispatch operators are allowed to use (previews
	// are backend-blocked for them).
	async function runByPathLegit(
		path: string,
		args: Record<string, any>
	): Promise<string | undefined> {
		if (!$workspaceStore) return undefined
		activeRunnables.arm(`script:${path}`)
		try {
			const jobId = await JobService.runScriptByPath({
				workspace: $workspaceStore,
				path,
				requestBody: { ...args }
			})
			runsPendingJobId = jobId
			runsRefreshKey++
			activeRunnable = { kind: 'script', path }
			activeRunnableJobId = jobId
			return jobId
		} catch (e: any) {
			sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
			return undefined
		}
	}

	// Whether the script open in the pane is a data-upload pipeline entry —
	// drives the read-only pane's run form. Derived from the displayed
	// graph's triggers so the drafts overlay picks up draft annotations too.
	let openScriptHasDataUpload = $derived.by(() => {
		const path = openScriptPath
		if (!path) return false
		return displayGraph.triggers.some(
			(t) =>
				t.trigger_kind === 'data_upload' && t.runnable_kind === 'script' && t.runnable_path === path
		)
	})

	// Overlay the draft runnable + live-parsed trigger edges onto the fetched
	// graph. Live edges come from the editor buffer's `// on <spec>` lines
	// and are marked `unsaved: true` unless they already match a persisted
	// script_trigger row. This keeps the canvas in sync with the editor
	// keystroke-by-keystroke; saving replaces the live edges with real ones
	// via graphRes.refetch().
	let graphWithDraft = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: graphRes.current ?? EMPTY_GRAPH,
			drafts: pe.drafts,
			liveBodyAssets: pe.liveBodyAssets,
			liveAnnotations: pe.liveAnnotations,
			inferredWritesByPath,
			inferredReadsByPath,
			annotatedNativeKindsByPath
		})
	)

	// Deployed-only resolution for view mode: same enrichment as the editor
	// (prefetched body lineage, annotated-but-rowless trigger placeholders)
	// but with no drafts and no live editor buffer. resolveGraph is pure, so
	// feeding it empty overlays is the cheapest way to share the logic.
	const EMPTY_DRAFTS: Map<string, Draft> = new Map()
	let deployedGraph = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: graphRes.current ?? EMPTY_GRAPH,
			drafts: EMPTY_DRAFTS,
			liveBodyAssets: EMPTY_LIVE_ASSETS,
			liveAnnotations: EMPTY_LIVE_ANNOTATIONS,
			inferredWritesByPath,
			inferredReadsByPath,
			annotatedNativeKindsByPath
		})
	)
	// What the canvas renders: drafts merged in edit, deployed-only in view —
	// unless the "show drafts" chip is on, which overlays the drafts onto the
	// view (what View will show once they're deployed).
	let displayGraph = $derived(mode === 'edit' || includeDrafts ? graphWithDraft : deployedGraph)

	// Leaving edit mode: drop the live editor overlays — they substitute the
	// last keystroke buffer for one script inside inferredWritesByPath /
	// annotatedNativeKindsByPath, and a stale buffer must not pollute the
	// view graphs. Tracks `mode` only.
	$effect(() => {
		if (mode === 'edit') return
		untrack(() => {
			// Guard before reassigning so an already-empty overlay doesn't
			// needlessly invalidate the graph derives every mode toggle.
			if (
				pe.liveAnnotations.scriptPath != undefined ||
				pe.liveBodyAssets.scriptPath != undefined ||
				pe.liveContent.scriptPath != undefined
			) {
				pe.clearLiveOverlays()
			}
		})
	})
	// Drafts are only addressable in view mode while the drafts overlay is
	// on — otherwise detach from any open one (the drafts Map itself is
	// preserved, so toggling back to edit keeps them). Tracks activeDraftPath
	// too, covering the onMount localStorage restore landing after a view load.
	$effect(() => {
		if (mode === 'view' && !includeDrafts && pe.activeDraftPath != undefined) {
			pe.activeDraftPath = undefined
		}
	})

	// Bumped after every successful run dispatch so AssetRunsPanel re-fetches
	// the listing immediately — the new (preview or script) job appears in
	// the history popover without waiting on its 3 s poll tick.
	let runsRefreshKey = $state(0)
	// The most recently dispatched job id — surfaces to AssetRunsPanel so
	// the new run auto-selects without an extra click.
	let runsPendingJobId = $state<string | undefined>(undefined)
	// Runnable currently executing a previewed/run job — animates the
	// edges going into and out of its node on the canvas. Cleared when
	// AssetRunsPanel reports the run is done. The same hook will later
	// be reused for live pipeline status.
	let activeRunnable = $state<{ kind: 'script' | 'flow'; path: string } | undefined>(undefined)
	// Job id backing `activeRunnable` when the run was launched from the canvas
	// for a node that ISN'T open in the editor pane. The editor (Test) path
	// clears `activeRunnable` via its own completion callbacks, but a per-node
	// run of an unselected script has no such callback — so we track its job id
	// and release the hint once that exact job finishes (see the effect below).
	// Kept in lockstep with `activeRunnable`: set only alongside the canvas-run
	// branch, cleared at every site that clears `activeRunnable`.
	let activeRunnableJobId = $state<string | undefined>(undefined)
	// Folder-scoped poll of in-flight (and just-finished) pipeline jobs. This
	// is what lights up the *downstream cascade* on the canvas — jobs the user
	// didn't launch directly. Observed in the background (slow cadence) the whole
	// time the graph is open so the node badges stay live; `arm()` upgrades to
	// the fast cadence when a run is launched from this view. Re-scoped/torn down
	// when the folder changes or the page unmounts.
	const activeRunnables = useActiveRunnableIds(
		() => $workspaceStore,
		() => pathPrefix
	)
	$effect(() => {
		pathPrefix // re-scope poll to the current folder
		// Observe in the background for the whole time the graph is open, so the
		// node run-count badges (and the activity log) stay live without the user
		// having to expand the activity panel first. `arm()` still upgrades to the
		// fast cadence when a run is launched from here. Re-armed after a folder
		// change because `dispose()` (below) turns observing off.
		activeRunnables.setObserving(true)
		return () => activeRunnables.dispose()
	})

	// View-mode activity panel: historical runs preloaded for the last N
	// days (user-configurable, persisted) merged with the live poll's events.
	const ACTIVITY_DAYS_KEY = 'pipeline-activity-days'
	// Allowed window sizes in days (fractions for the sub-day ranges).
	const ACTIVITY_WINDOW_DAYS = [1 / 24, 1, 2, 7, 30, 90]
	let activityDays = $state(30)
	onMount(() => {
		if (typeof localStorage === 'undefined') return
		const stored = Number(localStorage.getItem(ACTIVITY_DAYS_KEY))
		if (ACTIVITY_WINDOW_DAYS.some((d) => Math.abs(d - stored) < 1e-9)) activityDays = stored
	})
	function setActivityDays(days: number) {
		activityDays = days
		try {
			localStorage.setItem(ACTIVITY_DAYS_KEY, String(days))
		} catch {}
	}
	const pipelineHistory = usePipelineHistory(
		() => $workspaceStore,
		() => pathPrefix,
		() => activityDays,
		() => mode !== 'edit'
	)
	// Live events win on id collision — the poll upserts queued → running →
	// terminal, while a history row is a frozen snapshot from preload time.
	let activityEvents = $derived.by<PipelineEvent[]>(() => {
		const byId = new Map<string, PipelineEvent>()
		for (const e of pipelineHistory.events) byId.set(e.id, e)
		for (const e of activeRunnables.events) byId.set(e.id, e)
		return Array.from(byId.values()).sort((a, b) => b.at.localeCompare(a.at))
	})
	// Keep dispatch edges live so freshly-launched runs group (see `loadEdges`).
	// The poll's id set changes the instant a new job appears (a dispatched
	// child is a new id) — exactly when fresh edges exist — so re-pull then.
	// Keyed on ids, not status, so queued→done ticks don't refetch;
	// `lastLiveEventSig` is a plain `let` so writing it can't retrigger this.
	let lastLiveEventSig = ''
	$effect(() => {
		const sig = activeRunnables.events
			.map((e) => e.id)
			.sort()
			.join(',')
		if (sig === lastLiveEventSig) return
		lastLiveEventSig = sig
		pipelineHistory.refetchEdges()
	})
	// Node run-count/status badges, derived from the SAME merged event set the
	// Activity panel shows (historic preload + live poll) so the graph badges
	// and the panel never disagree. `activityEvents` is newest-first, so the
	// first event per runnable sets the badge status and the rest add to the
	// count. Future-scheduled queued runs aren't activity — skip them.
	let mergedRunStates = $derived.by<Map<string, RunnableRunState>>(() => {
		const m = new Map<string, RunnableRunState>()
		for (const e of activityEvents) {
			if (e.status === 'queued' && !isActiveEvent(e)) continue
			const id = `${e.kind}:${e.path}`
			const status: RunStatus =
				e.status === 'running' || e.status === 'queued'
					? 'running'
					: e.status === 'failure'
						? 'failure'
						: 'success'
			const cur = m.get(id)
			// Freshness compares against completion; `at` (start) is the
			// fallback lower bound for rows without a duration.
			const successAt = e.status === 'success' ? (e.completedAt ?? e.at) : undefined
			if (cur) {
				cur.runs += 1
				// Newest-first, so the first success per id is the latest one —
				// it feeds the freshness chip between graph refetches.
				if (successAt && !cur.lastSuccessAt) cur.lastSuccessAt = successAt
			} else {
				m.set(id, { status, runs: 1, lastSuccessAt: successAt })
			}
		}
		return m
	})
	// Activity-panel → canvas node emphasis. Hover and the pinned expanded run
	// are fed to the canvas as separate path sets so they render distinctly; a
	// group-header hover passes the whole cascade.
	let activityHoverPaths = $state<string[]>([])
	let activitySelectPaths = $state<string[]>([])
	// Release the zero-latency `activeRunnable` hint for a per-node run of a
	// script that isn't open in the pane: such runs have no editor Test callback
	// to clear it, so without this the node's badge stays 'running' forever (the
	// canvas forces 'running' while `activeRunnable` points at it, overriding the
	// folder poll's completed status). We key off the exact launched job id
	// reaching a terminal status in the poll's event log; `events` and the node
	// `states` are reassigned in the same poll tick, so the handoff to the poll's
	// success/failure badge is seamless (no flicker back to running).
	$effect(() => {
		if (!activeRunnableJobId) return
		const ev = activeRunnables.events.find((e) => e.id === activeRunnableJobId)
		if (ev && (ev.status === 'success' || ev.status === 'failure')) {
			activeRunnable = undefined
			activeRunnableJobId = undefined
		}
	})
	// One dev-run cascade at a time (root path while running). Guards the
	// Run+downstream affordance against overlapping chains stomping each
	// other's storage writes.
	let cascadeRunningRoot = $state<string | undefined>(undefined)

	// Recorder: when armed, the next cascade run captures the resolved graph, the
	// per-node status timeline and each node's job stream into a downloadable
	// recording that the /pipeline_replay player can rerun offline (parity with the
	// flow/script recorders). Job capture (`watchJob`) and status capture
	// (`recordStatuses`) no-op unless the store is active, so the cascade run
	// paths call them unconditionally.
	let pipelineRecording = createPipelineRecording()
	let recordingMode = $state(false)
	let lastPipelineRecording = $state<PipelineRecording | undefined>(undefined)

	function downloadPipelineRecording() {
		if (lastPipelineRecording) {
			pipelineRecording.download(lastPipelineRecording)
		}
	}

	// Script path → its schedule's configured args, so a manual "Run pipeline"
	// launches a schedule-triggered script with the same payload a real tick
	// would (rather than empty args). Schedule is the only trigger that stores a
	// static args payload; every other trigger (webhook / http / event kinds /
	// data_upload) receives its input from the live event or request at dispatch
	// time — a message, an HTTP body, an uploaded file — which doesn't exist
	// during a manual run, so there's nothing to default and those scripts run
	// empty (data_upload being fed its staged file instead, see dataUploadArgs).
	// Resolved on demand right before a run (schedule args aren't in the graph
	// response); fail-safe — a schedule we can't fetch just contributes nothing.
	let scheduleArgsByPath: Record<string, Record<string, any>> = {}
	async function resolveScheduleArgs(scripts: string[]): Promise<void> {
		scheduleArgsByPath = {}
		const ws = $workspaceStore
		if (!ws) return
		const runSet = new Set(scripts)
		// First schedule (with args) per in-run script; dedupe schedule fetches.
		const wanted = new Map<string, string>() // script path → schedule path
		for (const t of displayGraph.triggers) {
			if (t.trigger_kind !== 'schedule' || t.runnable_kind !== 'script') continue
			if (!runSet.has(t.runnable_path) || wanted.has(t.runnable_path)) continue
			if ((t as any).path) wanted.set(t.runnable_path, (t as any).path)
		}
		await Promise.all(
			[...wanted].map(async ([scriptPath, schedulePath]) => {
				try {
					const sched = await ScheduleService.getSchedule({ workspace: ws, path: schedulePath })
					if (sched?.args && Object.keys(sched.args).length > 0) {
						scheduleArgsByPath[scriptPath] = sched.args as Record<string, any>
					}
				} catch {
					// schedule unreadable / deleted — leave its script on empty args
				}
			})
		)
	}
	// Launch one pipeline script for the dev-run cascade. Always passes
	// _wmill_skip_asset_dispatch — the orchestrator owns the whole closure,
	// so the backend dispatcher must not double-fire the deployed part of a
	// mixed chain. Drafts run as previews of their local content; deployed
	// scripts run their *saved* version (an open editor buffer with unsaved
	// edits is not picked up — same as production dispatch).
	async function launchCascadeScript(path: string): Promise<string> {
		if (!$workspaceStore) throw new Error('no workspace')
		// Only run draft content when the displayed graph actually includes
		// drafts (same condition as `displayGraph`). Otherwise — View mode with
		// drafts hidden — a bounded run must execute the *deployed* scripts the
		// user is looking at, not preview jobs from hidden local drafts.
		// Default a script's run args to its schedule's stored payload (the only
		// trigger with static args — see resolveScheduleArgs), then let a
		// data-upload entry's staged file override. runWholePipeline already
		// refused to start unless every data-upload entry is staged, so this is
		// always the intended input.
		const staged = { ...(scheduleArgsByPath[path] ?? {}), ...(dataUploadArgs[path]?.args ?? {}) }
		const draft = mode === 'edit' || includeDrafts ? pe.drafts.get(path) : undefined
		if (draft) {
			if (!draft.script.content || !draft.script.language) {
				throw new Error(`draft ${path} has no content/language`)
			}
			return await JobService.runScriptPreview({
				workspace: $workspaceStore,
				requestBody: {
					content: draft.script.content,
					language: draft.script.language,
					path,
					args: { ...staged, _wmill_skip_asset_dispatch: true }
				}
			})
		}
		return await JobService.runScriptByPath({
			workspace: $workspaceStore,
			path,
			requestBody: { ...staged, _wmill_skip_asset_dispatch: true }
		})
	}
	// Poll a launched cascade job to a terminal state. Modest fixed cadence —
	// chains are short and the folder poll is already watching the same jobs
	// for the canvas animation. Capped so a never-terminating job can't pin
	// `cascadeRunningRoot` forever and wedge every future cascade: after the
	// timeout we throw, which the orchestrator surfaces as a chain failure and
	// `runDraftAwareCascade`'s finally clears the running-root guard.
	const CASCADE_POLL_INTERVAL_MS = 1000
	const CASCADE_JOB_TIMEOUT_MS = 30 * 60 * 1000
	async function waitJobTerminal(jobId: string): Promise<'success' | 'failure'> {
		const deadline = Date.now() + CASCADE_JOB_TIMEOUT_MS
		while (Date.now() < deadline) {
			try {
				const r = await JobService.getCompletedJobResultMaybe({
					workspace: $workspaceStore!,
					id: jobId,
					getStarted: false
				})
				if (r.completed) return r.success ? 'success' : 'failure'
			} catch {
				// transient — retry on the next tick
			}
			await new Promise((res) => setTimeout(res, CASCADE_POLL_INTERVAL_MS))
		}
		throw new Error(
			`Timed out after ${Math.round(CASCADE_JOB_TIMEOUT_MS / 60000)}min waiting for job ${jobId} to finish`
		)
	}
	// "Run + downstream" over a chain that includes drafts: the backend
	// asset-trigger dispatcher only resolves deployed rows, so the page
	// orchestrates the closure itself (topological order over the resolved
	// graph the user is looking at). Deployed-only chains never come here —
	// they keep the production dispatcher (see onRunProducer).
	async function runDraftAwareCascade(rootPath: string): Promise<string | undefined> {
		if (cascadeRunningRoot) {
			sendUserToast(`A chain run from ${cascadeRunningRoot} is still in progress`, true)
			return undefined
		}
		const closure = computeDownstreamClosure(graphWithDraft, rootPath)
		if (closure.cyclic.length > 0) {
			sendUserToast(
				`Not running ${closure.cyclic.length} script(s) on a dependency cycle: ${closure.cyclic.join(', ')}`,
				true
			)
		}
		// Claim the running-guard BEFORE the first await so a rapid second click
		// (which reads `cascadeRunningRoot`) can't slip through and double-launch.
		cascadeRunningRoot = rootPath
		let rootJobId: string | undefined
		try {
			// Seed schedule-triggered members with their configured payload.
			await resolveScheduleArgs([rootPath, ...closure.nodes])
			const res = await runCascade({
				closure,
				root: rootPath,
				launch: async (path) => {
					const jobId = await launchCascadeScript(path)
					// Surface each hop exactly like a hand-launched run: poll
					// fast-armed for edge/badge animation, runs panel refreshed.
					activeRunnables.arm(`script:${path}`)
					if (path === rootPath) {
						rootJobId = jobId
						runsPendingJobId = jobId
						runsRefreshKey++
					}
					return jobId
				},
				waitTerminal: waitJobTerminal
			})
			const n = res.statuses.size
			if (res.ok) {
				sendUserToast(`Chain run complete — ${n} script${n === 1 ? '' : 's'} succeeded`)
			} else {
				const failed = [...res.statuses.entries()].filter(([, s]) => s.status === 'failure')
				const skipped = [...res.statuses.values()].filter((s) => s.status === 'skipped').length
				// Surface the failed node's error in the details pane. Clear the
				// active draft too — it has pane priority over `selection`
				// (openScriptPath), so a bare selection would stay masked while a
				// draft is open.
				if (failed.length > 0) {
					pe.activeDraftPath = undefined
					pe.selection = { kind: 'runnable', runnable_kind: 'script', path: failed[0][0] }
				}
				sendUserToast(
					`Chain run failed at ${failed.map(([p]) => p).join(', ')}` +
						(skipped > 0 ? ` — ${skipped} downstream skipped` : ''),
					true
				)
			}
		} finally {
			cascadeRunningRoot = undefined
		}
		return rootJobId
	}

	// ── Bounded-cascade selective execution ──────────────────────────────
	// "Run downstream up to…" lets the user run a *prefix* of a cascade: start
	// at a schedule/manual root, fan downstream, but stop at chosen end node(s).
	// The matched set is the path-between of start and ends over the lineage DAG.
	// Pick state is in engine node-id space (`script:path`, `${kind}:${path}`);
	// it is converted to canvas ids only at the canvas boundary.
	let boundPickStart = $state<string | undefined>(undefined)
	let boundPickEnds = $state<Set<string>>(new Set())

	// Script paths eligible to start a bounded run, for the canvas menu gate.
	// Any node with downstream — roots AND mid-DAG models — can be a "Run +
	// downstream" start (dbt `model+`); only event-triggered scripts are excluded
	// (see boundedCascade.validFromStarts). The canvas additionally requires the
	// node to have lineage downstream before offering the entry.
	let validStartPaths = $derived(new Set(scriptsOf(validFromStarts(displayGraph))))
	// Scripts with read-aware downstream — the same gate the canvas applies
	// (AssetGraphCanvas `hasLineageDownstream`). A valid start with no downstream
	// has no end to pick, so the bounded-run entry is suppressed everywhere,
	// including the details-pane (ScriptEditor) Test caret.
	let lineageDownstreamPaths = $derived(new Set(buildLineageDownstreamMap(displayGraph).keys()))

	// Rebuilt only while a pick is active (cheap to skip otherwise).
	let boundDag = $derived(boundPickStart ? buildLineageDag(displayGraph) : undefined)
	let boundResult = $derived(
		boundDag && boundPickStart
			? boundedSet(boundDag, boundPickStart, [...boundPickEnds])
			: undefined
	)
	// Event handlers (kafka/mqtt/…) can't run with empty args, so they're cut from
	// any cascade run — as is a consumer reachable only through one. But a
	// scheduled/manual root is NEVER a barrier even if it also carries an event
	// trigger: it runs on its own schedule/manual identity (schedule wins in
	// validStarts), so it — and its downstream — stay reachable when running from
	// an upstream start. The explicit start is likewise protected. Parity with the
	// CLI `barriers` set (`nonAutorunTriggerScripts` minus `starts` minus start).
	let boundReachable = $derived.by(() => {
		if (!boundDag || !boundPickStart) return new Set<string>()
		const roots = validStarts(displayGraph)
		const barriers = new Set(
			[...nonAutorunTriggerScripts(displayGraph)].filter(
				(id) => !roots.has(id) && id !== boundPickStart
			)
		)
		return reachableCutting(boundDag, [boundPickStart], barriers)
	})
	// Nodes pickable as end bounds: the barrier-cut downstream (excluding the
	// start), NOT raw descendants — else an event handler or a node only reachable
	// through one could be clicked as an end yet get silently dropped from the run.
	let boundEligible = $derived(new Set([...boundReachable].filter((id) => id !== boundPickStart)))
	// The actual run set (nodes, scripts + assets). No end picked → "Run +
	// downstream" (dbt `model+`): the start plus its full transitive downstream.
	// Picking end(s) narrows it to the path-between set. Either way, intersect with
	// the barrier-cut closure so an event descendant is never launched empty. The
	// canvas highlights exactly this set, so the ring matches what will run.
	let boundRunNodes = $derived(
		boundPickStart
			? boundPickEnds.size === 0
				? boundReachable
				: new Set([...(boundResult?.nodes ?? [])].filter((n) => boundReachable.has(n)))
			: new Set<string>()
	)
	let boundScripts = $derived(scriptsOf(boundRunNodes))

	// Engine id → canvas id: scripts keep `script:path`; assets gain the
	// canvas's `asset:` prefix (AssetGraphCanvas node ids).
	const toCanvasId = (eid: string): string => (isScriptNode(eid) ? eid : `asset:${eid}`)
	const fromCanvasId = (cid: string): string =>
		cid.startsWith('asset:') ? cid.slice('asset:'.length) : cid
	// Short label for a `script:f/folder/name` start id (last path segment).
	const shortPath = (scriptId: string): string => {
		const p = isScriptNode(scriptId) ? scriptId.slice('script:'.length) : scriptId
		return p.split('/').pop() ?? p
	}
	let boundPick = $derived(
		boundPickStart && boundDag
			? {
					start: boundPickStart,
					eligible: new Set([...boundEligible].map(toCanvasId)),
					ends: new Set([...boundPickEnds].map(toCanvasId)),
					bounded: new Set([...boundRunNodes].map(toCanvasId))
				}
			: undefined
	)

	function startBoundedRun(path: string) {
		boundPickStart = scriptNodeId(path)
		boundPickEnds = new Set()
	}
	function pickBoundEnd(canvasNodeId: string) {
		const eid = fromCanvasId(canvasNodeId)
		if (eid === boundPickStart) return
		const next = new Set(boundPickEnds)
		if (next.has(eid)) next.delete(eid)
		else next.add(eid)
		boundPickEnds = next
	}
	function cancelBoundedRun() {
		boundPickStart = undefined
		boundPickEnds = new Set()
	}
	async function confirmBoundedRun() {
		const scripts = boundScripts
		cancelBoundedRun()
		await runBoundedCascade(scripts)
	}
	// Run an arbitrary selected set of scripts in topological order. Same
	// per-hop launch + poll as the draft-aware cascade (it skips the backend
	// dispatcher so the page owns the whole closure), but multi-root: every
	// selected script with no in-set upstream is seeded at once.
	async function runBoundedCascade(scripts: string[]): Promise<void> {
		if (scripts.length === 0) {
			sendUserToast('No scripts to run in this selection', true)
			return
		}
		if (cascadeRunningRoot) {
			sendUserToast(`A chain run from ${cascadeRunningRoot} is still in progress`, true)
			return
		}
		// Read-aware adjacency so a pure-reader member runs after its producer
		// (parity with the CLI `topoOrder`); see buildLineageDownstreamMap.
		const schedule = computeInducedSchedule(
			displayGraph,
			new Set(scripts),
			buildLineageDownstreamMap(displayGraph)
		)
		if (schedule.cyclic.length > 0) {
			sendUserToast(
				`Not running ${schedule.cyclic.length} script(s) on a dependency cycle: ${schedule.cyclic.join(', ')}`,
				true
			)
		}
		if (schedule.nodes.length === 0) {
			sendUserToast('No runnable scripts in this selection', true)
			return
		}
		// Claim the running-guard BEFORE the first await so a rapid second click
		// (which reads `cascadeRunningRoot`) can't slip through and double-launch.
		cascadeRunningRoot = schedule.roots[0] ?? scripts[0]
		if (recordingMode) {
			lastPipelineRecording = undefined
			pipelineRecording.start(folder, displayGraph)
		}
		let firstJobId: string | undefined
		try {
			// Seed schedule-triggered roots with their configured payload.
			await resolveScheduleArgs(schedule.nodes)
			const res = await runSelection({
				schedule,
				launch: async (path) => {
					const jobId = await launchCascadeScript(path)
					activeRunnables.arm(`script:${path}`)
					// No-op unless a recording is active; captures the node's stream.
					if ($workspaceStore) pipelineRecording.watchJob(jobId, $workspaceStore)
					if (firstJobId === undefined) {
						firstJobId = jobId
						runsPendingJobId = jobId
						runsRefreshKey++
					}
					return jobId
				},
				waitTerminal: waitJobTerminal,
				onUpdate: (statuses) => pipelineRecording.recordStatuses(statuses)
			})
			const n = res.statuses.size
			if (res.ok) {
				sendUserToast(`Bounded run complete — ${n} script${n === 1 ? '' : 's'} succeeded`)
			} else {
				const failed = [...res.statuses.entries()].filter(([, s]) => s.status === 'failure')
				const skipped = [...res.statuses.values()].filter((s) => s.status === 'skipped').length
				// Surface the failed node's error in the details pane. Clear the
				// active draft too — it has pane priority over `selection`
				// (openScriptPath), so a bare selection would stay masked while a
				// draft is open.
				if (failed.length > 0) {
					pe.activeDraftPath = undefined
					pe.selection = { kind: 'runnable', runnable_kind: 'script', path: failed[0][0] }
				}
				sendUserToast(
					`Bounded run failed at ${failed.map(([p]) => p).join(', ')}` +
						(skipped > 0 ? ` — ${skipped} downstream skipped` : ''),
					true
				)
			}
		} finally {
			cascadeRunningRoot = undefined
			if (pipelineRecording.active) {
				lastPipelineRecording = await finalizePipelineRecording(pipelineRecording, $workspaceStore)
			}
		}
	}

	// Staged run-form input for data-upload entry scripts, keyed by path. Lifted
	// out of the (transient, per-selection) run form so the uploaded/entered data
	// persists across selection changes — it drives each entry node's green
	// "ready" state and seeds the whole-pipeline run with that input. `valid` is
	// the run form's full-schema validity (all required fields satisfied).
	let dataUploadArgs = $state<Record<string, { args: Record<string, any>; valid: boolean }>>({})

	// An S3Object-shaped value (the file picker writes `{ s3: '<path>' }`).
	function isS3Object(v: any): boolean {
		return !!v && typeof v === 'object' && !Array.isArray(v) && 's3' in v
	}
	// Whether a staged value carries actual data. Covers the two shapes a
	// data-upload entry takes: an S3Object (file picker → `{ s3: '<path>' }`) and
	// a plain required input (e.g. a JSON array pasted into the run form).
	function hasMeaningfulValue(v: any): boolean {
		if (v == null) return false
		if (typeof v === 'string') return v.length > 0
		if (Array.isArray(v)) return v.length > 0
		if (typeof v === 'object')
			return isS3Object(v) ? typeof v.s3 === 'string' && v.s3.length > 0 : Object.keys(v).length > 0
		return true // numbers / booleans count as provided
	}
	// A data-upload entry is ready once the user actually provided its data, not
	// just opened the form. Two guards: the form's own full-schema `valid` (every
	// required field — including non-file ones — is satisfied), AND that any
	// declared S3Object file field actually carries a file (the picker can leave
	// an empty `{ s3: '' }` on a non-required file field, which `valid` alone
	// wouldn't catch).
	function dataUploadReady(path: string): boolean {
		const staged = dataUploadArgs[path]
		if (!staged || !staged.valid) return false
		const values = Object.values(staged.args)
		const s3s = values.filter(isS3Object)
		if (s3s.length > 0) return s3s.every(hasMeaningfulValue)
		return values.some(hasMeaningfulValue)
	}
	// Persist the run form's args + validity, but only for data-upload entries —
	// other run forms (partitioned producers) run their own way and must not be
	// mistaken for a staged upload. Idempotent: the run form re-emits on every
	// keystroke/validation pass, so bail when the value is unchanged — otherwise
	// each emit would reassign `dataUploadArgs`, giving `readyDataUploadPaths` a
	// fresh Set identity that re-syncs the canvas and re-fires the form's emit
	// effect (effect_update_depth_exceeded).
	function stageRunFormArgs(path: string, args: Record<string, any>, valid: boolean) {
		if (!dataUploadEntryPaths.has(path)) return
		const prev = dataUploadArgs[path]
		if (prev && prev.valid === valid && JSON.stringify(prev.args) === JSON.stringify(args)) return
		dataUploadArgs = { ...dataUploadArgs, [path]: { args, valid } }
	}
	// Pipeline scripts that are data-upload entry points (a `data_upload` trigger
	// in the displayed graph). They can't auto-run — they need an uploaded file
	// before the pipeline can go (see runWholePipeline's gate + the node's green
	// state).
	let dataUploadEntryPaths = $derived(
		new Set(
			displayGraph.triggers
				.filter((t) => t.trigger_kind === 'data_upload' && t.runnable_kind === 'script')
				.map((t) => t.runnable_path)
		)
	)
	// Of those, the ones with a staged file — drives the green node treatment.
	let readyDataUploadPaths = $derived(
		new Set([...dataUploadEntryPaths].filter((p) => dataUploadReady(p)))
	)

	// Every pipeline-member script, for the always-visible header "Run pipeline"
	// control. Per-node runs are hover/select-gated on the canvas; this
	// pipeline-level affordance runs the whole graph without hunting for a root
	// node to hover. Gated on `in_pipeline` so it never launches dependency-only
	// endpoints the graph surfaces for context (macro libraries, custom
	// data-test scripts, out-of-folder producers) — only actual pipeline steps.
	let allPipelineScripts = $derived(
		displayGraph.runnables
			.filter((r) => r.usage_kind === 'script' && r.in_pipeline)
			.map((r) => r.path)
	)
	// Run the whole pipeline: hand every script to the bounded-cascade engine,
	// which topo-orders them (roots first) and fans downstream — i.e. run + all
	// downstream from every source at once. Reuses the same per-hop launch/poll
	// and one-cascade-at-a-time guard as the node-level chain runs.
	async function runWholePipeline() {
		// Data-upload entries can't auto-run with empty args (they'd run against a
		// missing S3Object). Require every one to be staged (green) first, and
		// point the user at the first unready node instead of launching a doomed
		// run.
		const unready = allPipelineScripts.filter(
			(p) => dataUploadEntryPaths.has(p) && !dataUploadReady(p)
		)
		if (unready.length > 0) {
			sendUserToast(
				`Upload data to ${unready.length} data-upload node${unready.length === 1 ? '' : 's'} first — they must be green before the pipeline can run`,
				true
			)
			openDataUploadRun(unready[0])
			return
		}
		await runBoundedCascade(allPipelineScripts)
	}

	// Counter bumped when the canvas Run button targets the currently-open
	// script — the pane intercepts and routes through ScriptEditor.runTest
	// so logs/result/cancel land in the test panel instead of going off
	// into nowhere with only edge animation as feedback. Counter rather
	// than boolean so back-to-back runs re-fire.
	let requestRunSignal = $state(0)
	// Sister counter: bumped instead of requestRunSignal when the canvas user
	// picks "Run + trigger N downstream" for the currently-open script. Lets
	// ScriptEditor.runTest run with cascade=true without permanently flipping
	// its persistent cascade choice.
	let requestRunCascadeSignal = $state(0)
	// Counter bumped from the runnable-node action menu to ask the pane to
	// open its archive/delete confirmation modal for the loaded script.
	// Counter (vs boolean) so successive triggers re-fire even if the user
	// dismissed the previous modal without acting.
	let requestRemoveSignal = $state(0)
	// Counter bumped when the user clicks a data_upload source node — asks the
	// pane to scroll the run form's S3 file input into view and pulse it, so
	// the UI-first "upload a file to run" affordance is obvious.
	let focusDataUploadSignal = $state(0)

	// Producers (write/rw edges) for the currently-selected asset, derived
	// from `graphWithDraft.edges`. Threaded into the details pane so the
	// runs panel can list jobs for the right scripts. We include drafts —
	// running a draft via runScriptPreview creates a `preview`-kind job at
	// the same path, which the panel's listing query picks up.
	let selectionProducers = $derived(assetProducers(graphWithDraft, pe.selection))

	// Empty graph reused when the trace isn't shown (no ducklake-asset selection,
	// or a draft is actively edited) so the pane blanks out like the other
	// selection overlays and `buildColumnGraph` doesn't run.
	const EMPTY_COLUMN_GRAPH: ColumnLineageGraph = {
		nodes: new Map(),
		up: new Map(),
		down: new Map()
	}
	// Pipeline-wide column-lineage graph, stitched across every producer's
	// (inferred + annotated) `column_lineage` and the asset write-edges. Drives
	// the transitive column trace in the details pane. Built from `displayGraph`
	// — the exact graph the canvas renders — so the trace matches it: draft
	// overlays in edit / show-drafts, deployed-only in plain View. Gated to a
	// ducklake-asset selection so it isn't rebuilt on every editor keystroke when
	// the trace UI isn't even shown.
	let columnGraph = $derived(
		pe.selection?.kind === 'asset' && pe.selection.asset_kind === 'ducklake'
			? buildColumnGraph(displayGraph)
			: EMPTY_COLUMN_GRAPH
	)

	// Producer-side facts for the editor's live schema-contract diagnostics:
	// which assets are muted (`on_schema_change=ignore`) and which `_current`
	// views map to an scd2 base table. Derived from the same resolved graph the
	// canvas renders so the mirror suppresses exactly what the server check does.
	let schemaContractContext = $derived(buildSchemaContractContext(graphWithDraft.runnables))

	// Whether the selected ducklake asset's captured schema can *evolve* (drives
	// the asset panel's Schema tab: version history vs. a single fixed schema).
	// Only a whole-table `replace` producer (CREATE OR REPLACE) can change
	// columns run-to-run; `append`/`merge`/partitioned writes INSERT into a
	// fixed-schema table, so their schema is pinned at first materialize.
	//
	// Fail open: show the fixed view only when we're *sure* — every producer is a
	// known insert-style write. A producer with no `materialize_strategy`
	// metadata (e.g. a draft-overlay runnable, which the graph synthesizes
	// without it) is treated as unknown → evolvable, so captured history is never
	// hidden behind a stale "fixed" verdict.
	let schemaCanEvolve = $derived.by(() => {
		const sel = pe.selection
		if (!sel || sel.kind !== 'asset' || sel.asset_kind !== 'ducklake') return true
		const producerPaths = new Set(selectionProducers.map((p) => p.path))
		const producers = graphWithDraft.runnables.filter((r) => producerPaths.has(r.path))
		const knownFixed = (r: (typeof producers)[number]) =>
			!!r.materialize_strategy && !(r.materialize_strategy === 'replace' && !r.partition_kind)
		return producers.length === 0 || !producers.every(knownFixed)
	})

	// Fork data-environment state of the selected ducklake asset (fork workspaces
	// only): read off the graph node so the details pane can show the
	// deferred-to-parent / fork-materialized banner matching the canvas chip.
	let selectionForkMaterialization = $derived.by(() => {
		const sel = pe.selection
		if (!sel || sel.kind !== 'asset' || sel.asset_kind !== 'ducklake') return undefined
		return displayGraph.assets.find((a) => a.kind === 'ducklake' && a.path === sel.path)
			?.fork_materialization
	})

	// Downstream subscriber count for the currently-edited script. Drives
	// the Test button's cascade UX: when > 0, ScriptEditor renders a split
	// button exposing "just this step" (default, with `_wmill_skip_asset_dispatch`)
	// vs "trigger N downstream" (lets the dispatch hook fan out).
	//
	// Computation: for each (asset_kind, asset_path) the edited script writes
	// (w/rw edge), count distinct script subscribers (via `// on …` declared
	// in `triggers`) other than self. Flows are excluded because V1 dispatch
	// only fans out to scripts.
	let editedScriptDownstreamCount = $derived.by(() => {
		const editedPath = openScriptPath
		if (!editedPath) return 0
		const writes = graphWithDraft.edges.filter(
			(e) =>
				e.runnable_path === editedPath &&
				e.runnable_kind === 'script' &&
				(e.access_type === 'w' || e.access_type === 'rw')
		)
		if (writes.length === 0) return 0
		const subs = new Set<string>()
		for (const w of writes) {
			for (const t of graphWithDraft.triggers) {
				if (
					t.trigger_kind === 'asset' &&
					t.runnable_kind === 'script' &&
					t.runnable_path !== editedPath &&
					t.asset_kind === w.asset_kind &&
					t.asset_path === w.asset_path
				) {
					subs.add(t.runnable_path)
				}
			}
		}
		return subs.size
	})

	// Folder-picker modal state. Opens from the folder selector button when
	// there are no other pipelines to switch to, or from the "Choose another
	// folder…" entry in the dropdown otherwise.
	let pickerModalOpen = $state(false)

	// Native trigger editor drawers live in <PipelineTriggerEditors>; the page
	// drives them imperatively. The draft guards stay here because they depend
	// on the drafts map.
	let triggerEditors: PipelineTriggerEditors | undefined = $state()

	// Webhooks have no trigger row to create — clicking the node opens a
	// drawer with the endpoint URLs + the webhook-specific token creation
	// flow. Drafts have no deployed endpoint yet, so nudge the user to save
	// first (mirrors openMissingTriggerDrawer).
	function openWebhookDrawer(scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — webhooks only trigger the deployed version.`,
				true
			)
			return
		}
		triggerEditors?.openWebhook(scriptPath)
	}

	// Data upload is a UI-first entry point — no trigger row. Clicking the
	// node opens the target script in the details pane, where its
	// auto-generated run form (the script declares an `S3Object` input, so the
	// form renders the S3 picker) lets the user upload a file and run the
	// pipeline. Drafts open in-place too so the user can fill the body first.
	function openDataUploadRun(scriptPath: string) {
		// Opening the run form is meaningless while the pane is hidden —
		// unhide first, same as a plain node click.
		panelHidden = false
		// Same mode gate as handleCanvasSelect: view mode targets the
		// deployed script (its run form runs the deployed version), even
		// when unsaved edits were promoted to a draft.
		if (mode === 'edit' && pe.drafts.has(scriptPath)) {
			pe.activeDraftPath = scriptPath
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: scriptPath }
		}
		// Bump after a tick so the pane has reacted to the new selection/draft
		// and begun mounting the run form before it hunts for the S3 input.
		// The pane's effect additionally gates on the editor being ready, so a
		// re-click (pane already open) re-pulses immediately.
		void tick().then(() => focusDataUploadSignal++)
	}

	function openMissingTriggerDrawer(kind: NativeTriggerKind, scriptPath: string) {
		// A native trigger row stores `script_path` as a hard reference —
		// pointing it at a never-saved draft would either fail at create
		// time or silently bind to nothing. Surface that as a toast and
		// keep the drawer closed; the user needs to save the script first
		// (which also creates it under the new path if they renamed it).
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — triggers can only be attached to deployed scripts.`,
				true
			)
			return
		}
		triggerEditors?.openNew(kind, scriptPath)
	}

	// Lock the script-picker to the related script so the user can't
	// reassign the trigger off this pipeline from the canvas. The trigger
	// can still be edited everywhere else (TriggersPanel, etc.) where the
	// picker stays editable.
	function deleteAttachedTrigger(kind: NativeTriggerKind, triggerPath: string) {
		triggerEditors?.requestDelete(kind, triggerPath)
	}

	function openEditTriggerDrawer(kind: NativeTriggerKind, triggerPath: string, scriptPath: string) {
		triggerEditors?.openEdit(kind, triggerPath, scriptPath)
	}

	// Reuse the empty AssetGraphResponse so we can still render the canvas
	// (layout, controls, mini-map) on a fresh pipeline.
	const EMPTY_GRAPH: AssetGraphResponse = {
		assets: [],
		runnables: [],
		edges: [],
		triggers: []
	}

	// Powers the folder switcher in the header. Same endpoint the landing
	// page uses, so switches are free after the first fetch.
	let pipelineFoldersRes = resource(
		() => $workspaceStore,
		async (ws, _prev, { signal }) => {
			if (!ws) return [] as Array<{ folder: string; script_count: number }>
			const base_url = OpenAPI.BASE ?? ''
			const res = await fetch(`${base_url}/w/${ws}/assets/pipelines`, {
				credentials: 'include',
				signal
			})
			if (!res.ok) return []
			return (await res.json()) as Array<{ folder: string; script_count: number }>
		}
	)

	// Only pipelines the user can actually switch to — the current folder
	// is excluded. If this is empty, the folder selector button opens the
	// picker modal directly instead of a single-item dropdown.
	let otherPipelineFolders = $derived(
		(pipelineFoldersRes.current ?? []).filter((p) => p.folder !== folder)
	)

	let folderSwitcherItems = $derived.by(() => {
		// Keep the current mode across folder switches (view is the
		// param-less default).
		const modeQuery = mode !== 'view' ? `?mode=${mode}` : ''
		const items = otherPipelineFolders.map((p) => ({
			displayName: `f/${p.folder}`,
			icon: Folder,
			disabled: false,
			action: () => goto(`${base}/pipeline/${encodeURIComponent(p.folder)}${modeQuery}`)
		}))
		items.push({
			displayName: 'Choose another folder…',
			icon: FolderSearch,
			disabled: false,
			action: async () => {
				pickerModalOpen = true
			}
		})
		return items
	})

	// Empty pipelines land straight in the editor — a view of nothing is a
	// dead end. One-shot per folder, replaceState so Back doesn't bounce
	// through the empty view. Deep links with an explicit ?mode are honored,
	// and operators stay on the (empty) view.
	let autoEditCheckedFolder = $state<string | undefined>(undefined)
	$effect(() => {
		const g = graphRes.current
		const f = folder
		if (!g || graphRes.loading || autoEditCheckedFolder === f) return
		autoEditCheckedFolder = f
		if (untrack(() => urlMode) != null || untrack(() => isOperator)) return
		if (g.runnables.length === 0 && g.assets.length === 0) {
			setMode('edit', { replace: true })
		}
	})

	let graphRes = resource(
		[() => $workspaceStore, () => folder],
		async ([ws, f], _prev, { signal }) => {
			if (!ws || !f) return undefined
			const base_url = OpenAPI.BASE ?? ''
			const params = new URLSearchParams({
				folder: f,
				asset_kinds: DATA_KINDS.join(',')
			})
			const res = await fetch(`${base_url}/w/${ws}/assets/graph?${params}`, {
				credentials: 'include',
				signal
			})
			if (!res.ok) throw new Error(`GET /assets/graph → ${res.status}`)
			return (await res.json()) as AssetGraphResponse
		}
	)

	// Folder whose graph is actually rendered. `graphRes.current` is stale-
	// while-revalidate on an in-place folder switch, so keying the canvas's
	// one-shot initial fit on the route param would fire the new folder's fit
	// on the old graph and leave the fresh one unfitted. `folder` is read
	// untracked: the key must move only when a graph lands.
	let viewportFitFolder = $state('')
	$effect(() => {
		if (graphRes.current) untrack(() => (viewportFitFolder = folder))
	})

	// Body / inferred-assets prefetch sweep. Watches `g.runnables`; for any
	// non-draft path we haven't fetched yet, fetches `getScriptByPath` and
	// `inferAssets`, and stores both in their respective only-add caches.
	// All three previously-sticky maps (`inferredWritesByPath`,
	// `inferredReadsByPath`, `annotatedNativeKindsByPath`) are now derived
	// from these caches × the current graph, so rename / delete cleanup
	// happens implicitly when `g.runnables` changes — no per-mutation
	// cache surgery needed. A generation counter cancels in-flight work on
	// folder change so the previous folder's results never leak into the
	// new one.
	let prefetchingAssets = $state(false)
	$effect(() => {
		const ws = $workspaceStore
		const g = graphRes.current
		if (!ws || !g) return
		const gen = ++bodyFetchGen
		const targets = untrack(() =>
			g.runnables
				.filter((r) => r.usage_kind === 'script')
				.map((r) => r.path)
				.filter((p) => !pe.drafts.has(p) && !bodiesByPath.has(p))
		)
		if (targets.length === 0) return
		let i = 0
		const POOL = 6
		const worker = async () => {
			while (i < targets.length && gen === bodyFetchGen) {
				const path = targets[i++]
				try {
					const s = await ScriptService.getScriptByPath({ workspace: ws, path })
					if (gen !== bodyFetchGen) return
					const content = s.content ?? ''
					const res = await inferAssets(s.language, content)
					if (gen !== bodyFetchGen) return
					const inferred = (res?.assets ?? []) as AssetWithAltAccessType[]
					untrack(() => {
						if (!bodiesByPath.has(path)) {
							const nextBodies = new Map(bodiesByPath)
							nextBodies.set(path, content)
							bodiesByPath = nextBodies
						}
						if (!inferredAssetsByPath.has(path)) {
							const nextAssets = new Map(inferredAssetsByPath)
							nextAssets.set(path, inferred)
							inferredAssetsByPath = nextAssets
						}
					})
				} catch {
					// Skip — that node just falls back to base-graph edges.
				}
			}
		}
		prefetchingAssets = true
		const pool = Array.from({ length: Math.min(POOL, targets.length) }, () => worker())
		void Promise.all(pool).then(() => {
			if (gen === bodyFetchGen) prefetchingAssets = false
		})
	})

	function pluralize(n: number, singular: string): string {
		return `${n} ${singular}${n === 1 ? '' : 's'}`
	}

	// Human noun per asset kind for the header summary. The raw `ducklake` kind
	// counts materialized tables/views (including `_current` history views), so
	// "N ducklakes" mis-reads as a count of lakes — surface "table" instead. Other
	// kinds keep their own noun; unmapped kinds fall back to the raw kind.
	const ASSET_KIND_NOUN: Record<string, string> = {
		ducklake: 'table',
		datatable: 'table',
		s3object: 'file',
		volume: 'volume',
		resource: 'resource'
	}

	let summary = $derived.by<string[]>(() => {
		const g = graphRes.current
		if (!g) return []
		const parts: string[] = []
		const scripts = g.runnables.filter((r) => r.usage_kind === 'script').length
		const flows = g.runnables.filter((r) => r.usage_kind === 'flow').length
		if (scripts) parts.push(pluralize(scripts, 'script'))
		if (flows) parts.push(pluralize(flows, 'flow'))
		// Collapse kinds that share a noun (ducklake + datatable → "table") into a
		// single tally so the summary reads "5 tables", not "3 tables · 2 tables".
		const byNoun = new Map<string, number>()
		for (const a of g.assets) {
			const noun = ASSET_KIND_NOUN[a.kind] ?? a.kind
			byNoun.set(noun, (byNoun.get(noun) ?? 0) + 1)
		}
		for (const [noun, n] of byNoun) parts.push(pluralize(n, noun))
		return parts
	})
</script>

<svelte:head>
	<title>Pipeline · {folder} — Windmill</title>
</svelte:head>

<div class="flex flex-col h-full">
	<div
		class="border-b flex flex-row justify-between gap-2 px-2 py-1 items-center overflow-y-visible overflow-x-auto min-h-12 shrink-0 whitespace-nowrap"
	>
		<div class="flex flex-row items-center gap-2 flex-1 min-w-0">
			<Button
				variant="subtle"
				unifiedSize="sm"
				href="{base}/pipeline"
				startIcon={{ icon: ArrowLeft }}
				iconOnly
				title="Back to pipelines"
			/>
			<NetworkIcon size={16} class="text-tertiary shrink-0" />
			<h1 class="text-sm font-semibold">{mode === 'edit' ? 'Pipeline editor' : 'Pipeline'}</h1>
			<span class="text-tertiary text-sm">·</span>
			{#if otherPipelineFolders.length === 0}
				<button
					type="button"
					onclick={() => (pickerModalOpen = true)}
					class="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-gray-300 dark:border-gray-600 bg-surface hover:bg-surface-hover transition-colors"
					title="Switch pipeline folder"
				>
					<Folder size={14} class="text-tertiary shrink-0" />
					<span class="text-sm font-mono font-medium text-emphasis">f/{folder}</span>
					<ChevronDown size={12} class="text-tertiary" />
				</button>
			{:else}
				<DropdownV2 size="sm" items={folderSwitcherItems}>
					{#snippet buttonReplacement()}
						<span
							class="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-gray-300 dark:border-gray-600 bg-surface hover:bg-surface-hover transition-colors"
							title="Switch pipeline folder"
						>
							<Folder size={14} class="text-tertiary shrink-0" />
							<span class="text-sm font-mono font-medium text-emphasis">f/{folder}</span>
							<ChevronDown size={12} class="text-tertiary" />
						</span>
					{/snippet}
				</DropdownV2>
			{/if}
			{#if summary.length > 0}
				<span class="text-xs text-tertiary">· {summary.join(' · ')}</span>
			{/if}
		</div>
		{#if !isOperator}
			<!-- Center group: the mode toggle is the page's primary control —
			     anchored between the two flex-1 side groups so it stays
			     centered, with breathing room on both sides. -->
			<div class="flex flex-row items-center gap-2 shrink-0 px-6">
				<PipelineModeToggle {mode} draftCount={pe.drafts.size} onModeChange={(m) => setMode(m)} />
				{#if mode === 'view' && pe.drafts.size > 0}
					<!-- View variant: overlay the unsaved drafts onto the deployed
					     graph — "what View will show once they're deployed". -->
					<button
						type="button"
						onclick={() => (includeDrafts = !includeDrafts)}
						class={twMerge(
							'flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs font-medium transition-colors',
							includeDrafts
								? 'bg-amber-50 dark:bg-amber-900/30 border-amber-300 dark:border-amber-700 text-amber-700 dark:text-amber-400'
								: 'bg-surface border-gray-300 dark:border-gray-600 text-secondary hover:bg-surface-hover'
						)}
						title={includeDrafts
							? 'Showing undeployed drafts overlaid on the deployed pipeline — click to hide them'
							: 'Overlay your undeployed drafts to see what the pipeline will look like once deployed'}
					>
						<Telescope size={14} />
						{includeDrafts ? 'Showing' : 'Show'}
						{pe.drafts.size} draft{pe.drafts.size === 1 ? '' : 's'}
					</button>
				{/if}
			</div>
		{/if}
		<div class="flex flex-row items-center gap-2 flex-1 justify-end">
			{#if !isOperator && allPipelineScripts.length > 0}
				<!-- Recorder: arm to capture the next pipeline run into a
				     downloadable recording the /pipeline_replay player can rerun offline
				     (parity with the flow/script recorders). -->
				{#if lastPipelineRecording && !cascadeRunningRoot}
					<Button
						variant="subtle"
						unifiedSize="sm"
						startIcon={{ icon: Download }}
						onclick={downloadPipelineRecording}
						title="Download the last recorded pipeline run"
					>
						Download recording
					</Button>
				{/if}
				<button
					type="button"
					onclick={() => (recordingMode = !recordingMode)}
					disabled={!!cascadeRunningRoot}
					class={twMerge(
						'flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs font-medium transition-colors disabled:opacity-50',
						recordingMode
							? 'bg-red-50 dark:bg-red-900/30 border-red-300 dark:border-red-700 text-red-700 dark:text-red-400'
							: 'bg-surface border-gray-300 dark:border-gray-600 text-secondary hover:bg-surface-hover'
					)}
					title={recordingMode
						? 'Recording armed — the next pipeline run will be captured. Click to disarm.'
						: 'Arm the recorder so the next pipeline run is captured for offline replay'}
				>
					<Circle
						size={12}
						class={recordingMode ? 'fill-red-600 text-red-600 animate-pulse' : ''}
					/>
					{recordingMode ? 'Recording' : 'Record'}
				</button>
				<!-- Pipeline-level run: always visible in both View and Edit so a
				     run never requires hunting for a specific node's play button
				     (dbt `build` / Dagster "Materialize all"). Runs every script in
				     dependency order (roots first, cascading downstream) over the
				     displayed graph — deployed in View, draft-overlaid in Edit. -->
				<Button
					variant="accent-secondary"
					unifiedSize="sm"
					startIcon={{
						icon: cascadeRunningRoot ? Loader2 : Play,
						classes: cascadeRunningRoot ? 'animate-spin' : undefined
					}}
					onclick={runWholePipeline}
					disabled={!!cascadeRunningRoot}
					title={cascadeRunningRoot
						? 'A pipeline run is already in progress'
						: `Run all ${allPipelineScripts.length} script${
								allPipelineScripts.length === 1 ? '' : 's'
							} in dependency order (roots first, cascading downstream)`}
				>
					{cascadeRunningRoot ? 'Running…' : 'Run pipeline'}
				</Button>
			{/if}
			{#if mode === 'edit' && saveErrors.size > 0}
				<!-- Compact errors popover anchored next to Save all so users
				     can see exactly which drafts failed and why without losing
				     the editor context. Drafts that succeed disappear from
				     the map; the ones still listed here are the unresolved
				     failures. -->
				<Popover placement="bottom-end" contentClasses="p-3 max-w-[480px]" usePointerDownOutside>
					{#snippet trigger()}
						<button
							type="button"
							class="flex items-center gap-1.5 px-2 py-1 rounded-md text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/30 hover:bg-red-100 dark:hover:bg-red-900/50 transition-colors text-xs font-medium"
							title="View save errors"
						>
							<AlertTriangle size={14} />
							<span>{saveErrors.size} failed</span>
						</button>
					{/snippet}
					{#snippet content()}
						<div class="flex flex-col gap-2">
							<span class="text-xs font-semibold text-emphasis">Save errors</span>
							<div class="flex flex-col gap-2 max-h-72 overflow-y-auto">
								{#each [...saveErrors.entries()] as [path, message]}
									<div class="flex flex-col gap-0.5 border-l-2 border-red-400 pl-2">
										<span class="text-2xs font-mono text-emphasis">{path}</span>
										<span class="text-2xs text-red-600 dark:text-red-400 break-words">
											{message}
										</span>
									</div>
								{/each}
							</div>
						</div>
					{/snippet}
				</Popover>
			{/if}
			{#if mode === 'edit' && pe.drafts.size > 0}
				<!-- Draft autosave status for the whole pipeline bundle. Distinct
				     from "Save all", which DEPLOYS the drafts — this only reflects
				     that in-flight edits are persisted to the per-user server draft. -->
				{#if $workspaceStore}
					<AutosaveIndicator
						workspace={$workspaceStore}
						itemKind="data_pipeline"
						path={pipelineDraftPath}
						draftOnly
						loadedFromDraft={pe.loadedFromDbDraft}
					/>
				{/if}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: savingAll ? Loader2 : Save }}
					onclick={saveAllDrafts}
					disabled={savingAll}
					title={savingAll ? 'Saving drafts…' : `Deploy all ${pe.drafts.size} drafts`}
				>
					{savingAll ? 'Saving…' : `Save all (${pe.drafts.size})`}
				</Button>
			{/if}
			{#if mode === 'view'}
				<Button
					variant={activityShowing ? 'accent-secondary' : 'subtle'}
					unifiedSize="sm"
					startIcon={{ icon: History }}
					onclick={toggleActivity}
					title={activityShowing
						? 'Hide the activity panel'
						: 'Show the pipeline activity feed (recent and live runs)'}
				>
					Activity
				</Button>
			{/if}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: SquareFunction }}
				onclick={() => macroDrawer?.openDrawer()}
				title="Browse the workspace's DuckDB macros (deployed // macros libraries)"
			>
				Macros
			</Button>
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: RefreshCw }}
				onclick={() => graphRes.refetch()}
				disabled={graphRes.loading}
				iconOnly
				title="Refresh"
			/>
		</div>
	</div>

	<div class="flex-1 min-h-0">
		{#if graphRes.loading && !graphRes.current}
			<div class="h-full flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={18} class="animate-spin" />
				<span>Loading pipeline…</span>
			</div>
		{:else if graphRes.error}
			<div class="h-full flex items-center justify-center text-red-500 text-sm">
				Failed to load pipeline: {graphRes.error.message}
			</div>
		{:else}
			<PipelineGraphEditor
				editor={pe}
				{folder}
				viewportFitKey={viewportFitFolder}
				persistDrafts={true}
				{displayGraph}
				{mode}
				workspace={$workspaceStore}
				{pathPrefix}
				defaultPathSuffix={DEFAULT_PATH_SUFFIX}
				{panelHidden}
				onTogglePanelHidden={() => (panelHidden = !panelHidden)}
				{prefetchingAssets}
				hoveredPaths={activityHoverPaths}
				selectedRunPaths={activitySelectPaths}
				{activeRunnable}
				activeRunnableIds={activeRunnables.ids}
				runStates={mergedRunStates}
				eventLogEvents={activeRunnables.events}
				{runsRefreshKey}
				{runsPendingJobId}
				{boundPick}
				validStartPaths={isOperator ? undefined : validStartPaths}
				onStartBoundedRun={isOperator ? undefined : startBoundedRun}
				onPickEnd={pickBoundEnd}
				{panToNodeId}
				onCreateMissingTrigger={mode === 'edit' ? openMissingTriggerDrawer : undefined}
				onEditTrigger={mode === 'edit' ? openEditTriggerDrawer : undefined}
				onDeleteTrigger={mode === 'edit' ? deleteAttachedTrigger : undefined}
				onOpenWebhook={openWebhookDrawer}
				onOpenDataUpload={openDataUploadRun}
				{readyDataUploadPaths}
				runFormInitialArgs={openScriptPath ? dataUploadArgs[openScriptPath]?.args : undefined}
				onRunFormArgsChange={stageRunFormArgs}
				onSelect={handleCanvasSelect}
				onAddScriptForAsset={mode === 'edit' ? handleAddScriptForAsset : undefined}
				onAddPipelineScript={mode === 'edit' ? handleAddPipelineScript : undefined}
				onRunnableMenuRemove={mode === 'edit' ? handleRunnableMenuRemove : undefined}
				onRunProducer={mode === 'edit' ? handleRunProducer : undefined}
				onRequestEdit={isOperator ? undefined : () => setMode('edit')}
				canRunByPath={openScriptHasDataUpload}
				onRunByPath={runByPathLegit}
				{selectionProducers}
				selectionColumnGraph={pe.activeDraft ? EMPTY_COLUMN_GRAPH : columnGraph}
				{schemaCanEvolve}
				{selectionForkMaterialization}
				{schemaContractContext}
				downstreamSubscribers={editedScriptDownstreamCount}
				onStartBoundedRunForOpen={startBoundedRun}
				canBoundedRunOpenScript={!!openScriptPath &&
					validStartPaths.has(openScriptPath) &&
					lineageDownstreamPaths.has(openScriptPath)}
				onRunCompleted={() => {
					activeRunnable = undefined
					activeRunnableJobId = undefined
				}}
				onTestStateChange={(running) => {
					const openPath = openScriptPath
					if (running && openPath) {
						activeRunnable = { kind: 'script', path: openPath }
						activeRunnables.arm(`script:${openPath}`)
						activeRunnableJobId = undefined
					} else if (!running && activeRunnable?.path === openPath) {
						activeRunnable = undefined
						activeRunnableJobId = undefined
					}
				}}
				{requestRemoveSignal}
				{requestRunSignal}
				{requestRunCascadeSignal}
				focusUploadSignal={focusDataUploadSignal}
				onDraftPathChange={renameDraft}
				onClose={() => {
					pe.selection = undefined
					pe.activeDraftPath = undefined
					pe.liveAnnotations = EMPTY_LIVE_ANNOTATIONS
				}}
				onDiscard={() => {
					if (pe.activeDraftPath) discardDraft(pe.activeDraftPath)
				}}
				onDraftSaved={async (savedPath) => {
					const predicted = predictCascadeFacts([savedPath])
					const nextDrafts = new Map(pe.drafts)
					nextDrafts.delete(savedPath)
					pe.drafts = nextDrafts
					if (pe.activeDraftPath === savedPath) {
						pe.selection = { kind: 'runnable', runnable_kind: 'script', path: savedPath }
						pe.activeDraftPath = undefined
					}
					clearSaveError(savedPath)
					await graphRes.refetch()
					reportDeployDrift(predicted)
				}}
				onPersistedSaved={async (savedPath) => {
					const predicted = predictCascadeFacts([savedPath])
					await graphRes.refetch()
					reportDeployDrift(predicted)
				}}
				onScriptRenamed={async (oldPath, newPath) => {
					if (pe.selection?.kind === 'runnable' && pe.selection.path === oldPath) {
						pe.selection = { ...pe.selection, path: newPath }
					}
					await graphRes.refetch()
				}}
				onScriptRemoved={async (removedPath) => {
					forgetPath(removedPath)
					await graphRes.refetch()
				}}
			>
				{#snippet boundBar()}
					{#if boundPick}
						<div
							class="absolute bottom-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 px-3 py-2 rounded-lg bg-surface shadow-lg border border-gray-200 dark:border-gray-700"
						>
							<Target size={15} class="text-blue-500 shrink-0" />
							<div class="flex flex-col leading-tight">
								<span class="text-xs font-semibold text-emphasis">
									{boundPickEnds.size === 0
										? `Run + all downstream · ${boundScripts.length} script${boundScripts.length === 1 ? '' : 's'} (click node(s) to bound)`
										: `${boundScripts.length} script${boundScripts.length === 1 ? '' : 's'} up to ${boundPickEnds.size} end${boundPickEnds.size === 1 ? '' : 's'}`}
								</span>
								<span class="text-2xs text-tertiary">
									from {boundPickStart ? shortPath(boundPickStart) : ''}
								</span>
							</div>
							<Button variant="subtle" unifiedSize="sm" onclick={cancelBoundedRun}>Cancel</Button>
							<Button
								variant="accent"
								unifiedSize="sm"
								startIcon={{ icon: Play }}
								disabled={boundScripts.length === 0}
								onclick={confirmBoundedRun}
							>
								{boundPickEnds.size === 0 ? 'Run + downstream' : 'Run selection'}
							</Button>
						</div>
					{/if}
				{/snippet}
				{#snippet idlePane()}
					<PipelineActivityPanel
						events={activityEvents}
						edges={pipelineHistory.edges}
						loading={pipelineHistory.loading}
						truncated={pipelineHistory.truncated}
						error={pipelineHistory.error}
						days={activityDays}
						onDaysChange={setActivityDays}
						onHoverRun={(p) => (activityHoverPaths = p ?? [])}
						onSelectRun={(p) => (activitySelectPaths = p ?? [])}
					/>
				{/snippet}
			</PipelineGraphEditor>
		{/if}
	</div>
</div>

<PipelinePickerModal bind:open={pickerModalOpen} currentFolder={folder} {mode} />

<!-- Native trigger drawer wiring: create/edit/delete drawers (edit-mode
     only) + the always-mounted webhook drawer. Driven imperatively from the
     page via `triggerEditors`. -->
<PipelineTriggerEditors
	bind:this={triggerEditors}
	mountTriggerEditors={mode === 'edit'}
	onUpdate={() => graphRes.refetch()}
/>

{#if leaveModalOpen}
	<!-- Three-button leave guard. Built inline rather than reusing
	     ConfirmationModal because that one is binary (confirm/cancel) and
	     we need a distinct "Save all" path that runs the same dispatch as
	     the bar button. Layout mirrors the archive/delete modal in
	     AssetGraphDetailsPane for consistency. -->
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
							<h3 class="text-lg font-medium text-primary">
								{pe.drafts.size === 1 ? 'Unsaved draft' : `${pe.drafts.size} unsaved drafts`}
							</h3>
							<div class="mt-2 text-sm text-secondary flex flex-col gap-2">
								<p>
									You have {pe.drafts.size === 1
										? 'a draft pipeline script'
										: `${pe.drafts.size} draft pipeline scripts`} that {pe.drafts.size === 1
										? 'has'
										: 'have'} not been deployed yet. What would you like to do?
								</p>
								<ul class="text-2xs font-mono pl-4 max-h-40 overflow-y-auto flex flex-col gap-0.5">
									{#each [...pe.drafts.keys()] as p}
										<li class="truncate text-tertiary">{p}</li>
									{/each}
								</ul>
							</div>
						</div>
					</div>
					<div class="flex items-center gap-2 flex-row-reverse mt-4">
						<Button
							disabled={leaveSaving}
							onclick={leaveModalSaveAll}
							variant="accent"
							unifiedSize="sm"
							startIcon={{ icon: leaveSaving ? Loader2 : Save }}
						>
							<span class="min-w-20"
								>{leaveSaving ? 'Saving…' : `Save all (${pe.drafts.size})`}</span
							>
						</Button>
						<Button
							disabled={leaveSaving}
							onclick={leaveModalCancel}
							variant="default"
							unifiedSize="sm"
						>
							Cancel
						</Button>
						<Button
							disabled={leaveSaving}
							onclick={leaveModalDiscard}
							variant="contained"
							color="red"
							unifiedSize="sm"
							destructive
						>
							Discard all
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}

<MacroExplorerDrawer bind:this={macroDrawer} onOpenLib={openMacroLib} />
