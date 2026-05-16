<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { page } from '$app/state'
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import { useActiveRunnableIds } from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import PipelineEventLog from '$lib/components/assets/AssetGraph/PipelineEventLog.svelte'
	import AssetGraphDetailsPane from '$lib/components/assets/AssetGraph/AssetGraphDetailsPane.svelte'
	import PipelinePickerModal from '$lib/components/assets/AssetGraph/PipelinePickerModal.svelte'
	import {
		extractWrites,
		extractReads,
		type AssetWithAltAccessType
	} from '$lib/components/assets/lib'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import {
		parsePipelineAnnotations,
		type PipelineAnnotations
	} from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import {
		generatePipelineDraft,
		autoOutputAsset,
		type PipelineOutputKind,
		type DraftTriggerSource
	} from '$lib/components/assets/AssetGraph/pipelineTemplates'
	import { decodeState, encodeState } from '$lib/utils'
	import { onMount, tick, untrack } from 'svelte'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import {
		AlertTriangle,
		ArrowLeft,
		ChevronDown,
		Folder,
		FolderSearch,
		Loader2,
		NetworkIcon,
		RefreshCw,
		Save
	} from 'lucide-svelte'
	import {
		JobService,
		OpenAPI,
		ScriptService,
		type AssetKind,
		type Script,
		type ScriptLang
	} from '$lib/gen'
	import { resource } from 'runed'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { beforeNavigate, goto } from '$app/navigation'
	import { fade } from 'svelte/transition'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import HideButton from '$lib/components/apps/editor/settingsPanel/HideButton.svelte'
	import { inferArgs, inferAssets } from '$lib/infer'

	// Variables and resources are declarative config, not pipeline assets —
	// they're hub-shaped (referenced by most runnables) and would swamp the
	// layout without adding lineage information.
	const DATA_KINDS = ['s3object', 'ducklake', 'datatable', 'volume']

	let folder = $derived(page.params.folder as string)
	let selection = $state<AssetGraphSelection | undefined>(undefined)

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
	// Default cron for top + pipeline scripts (pipeline roots). Every hour is
	// a sane middle ground between batch and real-time; users edit the
	// `// schedule "..."` line in the editor before saving if they want
	// something different. Asset-triggered scripts don't get a schedule by
	// default — they inherit their trigger from the upstream asset.
	const DEFAULT_SCHEDULE_CRON = '0 * * * *'

	// In-flight drafts keyed by script path. Multiple can coexist — clicking
	// + repeatedly creates additional drafts, each with its own random
	// output asset, and they all render on the graph simultaneously.
	// Saving removes a draft from the map; closing the pane keeps it so the
	// user can come back to it.
	type Draft = {
		script: Script
		// Undefined when the user picked `outputKind === 'none'` — the draft
		// has no auto-generated output asset, so the graph overlay skips
		// synthesizing a write edge for it.
		outputAsset?: { kind: AssetKind; path: string }
		// Inferred body writes from the last time this draft was open in
		// the details pane. Captured on transition (selection change /
		// pane close) so the canvas keeps showing the user's renamed
		// outputs after they've clicked away. Falls back to outputAsset
		// when undefined (initial state or parser miss).
		outputAssets?: Array<{ kind: AssetKind; path: string }>
	}
	let drafts = $state<Map<string, Draft>>(new Map())

	// Which draft (if any) is currently open in the details pane. When
	// undefined and `selection` is set, the pane shows the persisted
	// selection's script. Never both at once.
	let activeDraftPath = $state<string | undefined>(undefined)

	// Splitpanes sizes: bound so user-resized widths persist when the
	// details pane is hidden + re-shown, or when switching between draft
	// and persisted selections (previously the panes were sized inline,
	// which made the right pane jump width when activeDraft was set
	// since the left was hardcoded to 100%). When the right pane
	// unmounts, svelte-splitpanes does NOT auto-stretch the remaining
	// pane — the bound `leftPaneSize` stays at its last value and the
	// other 40% renders as blank space. We explicitly set leftPaneSize
	// to 100 in that state via a $derived effect below, and restore the
	// stored split when the pane comes back.
	let leftPaneSize = $state(60)
	let rightPaneSize = $state(40)
	let storedRightPaneSize = $state(40)
	// Explicit hide flag — keeps `selection` / `activeDraftPath` intact
	// so re-opening the pane re-uses them. Mirrors AppEditor's
	// hideRightPanel/showRightPanel pattern.
	let panelHidden = $state(false)
	let detailsPaneOpen = $derived(
		(selection != undefined || activeDraftPath != undefined) && !panelHidden
	)

	$effect(() => {
		if (detailsPaneOpen) {
			// Pane visible: reapply the stored split. We don't depend on
			// rightPaneSize here (only stored) so user resizes via the
			// splitter handle aren't immediately overridden.
			const restore = storedRightPaneSize
			untrack(() => {
				rightPaneSize = restore
				leftPaneSize = 100 - restore
			})
		} else {
			// About to hide. Stash the current right size so the next show
			// restores it, then expand the left pane to fill — splitpanes
			// won't do this automatically when a Pane unmounts.
			untrack(() => {
				if (rightPaneSize > 0) storedRightPaneSize = rightPaneSize
				leftPaneSize = 100
			})
		}
	})

	// Per-folder localStorage key. Matches the flow-builder pattern so
	// reloading /pipeline/<folder> restores in-flight drafts. We serialize
	// the drafts map as an entry array (Map doesn't survive JSON.stringify).
	let storageKey = $derived(`pipeline-${folder}`)

	onMount(() => {
		if (typeof localStorage === 'undefined') return
		const raw = localStorage.getItem(`pipeline-${folder}`)
		if (!raw) return
		try {
			const state = decodeState(raw)
			if (Array.isArray(state?.drafts)) {
				const loaded = new Map<string, Draft>()
				for (const entry of state.drafts) {
					if (entry && typeof entry[0] === 'string' && entry[1]?.script) {
						loaded.set(entry[0], entry[1] as Draft)
					}
				}
				if (loaded.size > 0) drafts = loaded
			}
			if (typeof state?.activeDraftPath === 'string') {
				activeDraftPath = state.activeDraftPath
			}
		} catch (e) {
			console.warn('failed to restore pipeline state', e)
		}
	})

	// Debounced persist: reruns whenever drafts / activeDraftPath change.
	// 500 ms matches FlowBuilder.saveSessionDraft; balances typing churn
	// against losing state to a crash.
	let persistTimer: number | undefined = undefined
	$effect(() => {
		// Track deps explicitly so Svelte 5 re-runs on mutation.
		// For the active draft, also snapshot the latest live body writes
		// at serialize time. Without this, edits made since the last
		// pane-transition (the cleanup that calls onDraftPersist) are
		// lost on reload — the draft restores with stale `outputAssets`
		// and the graph drops the corresponding edges until the user
		// re-opens the draft and types something new.
		const liveWritesSnapshot =
			liveBodyAssets.scriptPath != undefined && drafts.has(liveBodyAssets.scriptPath)
				? extractWrites(liveBodyAssets.assets)
				: undefined
		const liveWritesPath = liveBodyAssets.scriptPath
		const serialized = Array.from(drafts.entries()).map(([p, d]) => {
			if (liveWritesSnapshot != undefined && liveWritesPath === p) {
				return [
					p,
					{ ...d, outputAssets: liveWritesSnapshot.length > 0 ? liveWritesSnapshot : undefined }
				] as [string, Draft]
			}
			return [p, d] as [string, Draft]
		})
		const activePath = activeDraftPath
		const key = storageKey
		untrack(() => {
			if (typeof localStorage === 'undefined') return
			if (persistTimer != undefined) clearTimeout(persistTimer)
			persistTimer = window.setTimeout(() => {
				try {
					if (serialized.length === 0 && !activePath) {
						localStorage.removeItem(key)
					} else {
						localStorage.setItem(
							key,
							encodeState({ drafts: serialized, activeDraftPath: activePath })
						)
					}
				} catch (e) {
					console.warn('failed to persist pipeline state', e)
				}
			}, 500)
		})
	})

	// Live-parsed annotations from whatever script is currently open in the
	// details pane (draft or existing). Refreshed on every keystroke via
	// `onAnnotationsChange`. Used to overlay unsaved schedule / trigger-asset
	// edges onto the graph so the editor buffer and the graph stay in sync.
	let liveAnnotations = $state<{
		scriptPath: string | undefined
		annotations: PipelineAnnotations
	}>({
		scriptPath: undefined,
		annotations: {
			inPipeline: false,
			triggerAssets: [],
			schedules: [],
			nativeTriggers: []
		}
	})

	// Live-inferred body assets (read/write usages parsed by inferAssets
	// — e.g. CREATE TABLE in SQL, loadS3File / writeS3File in TS/Python).
	// Refreshed via onAssetsChange. We use the write subset as the
	// authoritative output node set for drafts whose body has been edited
	// past the seeded template; without this, renaming a CREATE TABLE
	// target leaves the stale auto-output node on the graph.
	let liveBodyAssets = $state<{
		scriptPath: string | undefined
		assets: AssetWithAltAccessType[]
	}>({ scriptPath: undefined, assets: [] })

	// Sticky cache of inferred body assets per script path — accumulates as
	// the user opens scripts in this session, so once we've seen a write
	// for `f/foo/bar`, the edge stays on the canvas even after the user
	// selects a different node. Without this, switching selection would
	// drop the previous script's edges back to whatever's in `base.edges`
	// (often nothing for scripts whose deploy didn't extract body assets,
	// e.g. when an older WASM was used at save time).
	let inferredWritesByPath = $state<Map<string, Array<{ kind: AssetKind; path: string }>>>(
		new Map()
	)
	// Same sticky cache for read usages (e.g. duckdb `read_parquet('s3://…')`,
	// loadS3File). Keeps the asset → reader lineage edge live as the body is
	// edited / across selection, instead of only after Save re-derives the
	// persisted asset rows.
	let inferredReadsByPath = $state<Map<string, Array<{ kind: AssetKind; path: string }>>>(new Map())

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
		// the user picked `none`, `outputAsset` is undefined and the graph
		// overlay skips synthesizing a write edge.
		const next = new Map(drafts)
		next.set(scriptPath, { script, outputAsset: out })
		drafts = next
		activeDraftPath = scriptPath
		selection = undefined

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
		if (drafts.size === 0) return
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
		drafts = new Map()
		activeDraftPath = undefined
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
		if (drafts.size === 0) {
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

	async function saveDraft(path: string, draft: Draft, ws: string): Promise<void> {
		const script = structuredClone($state.snapshot(draft.script) as Script)
		script.schema = script.schema ?? emptySchema()
		try {
			const result = await inferArgs(script.language, script.content, script.schema)
			;(script as any).auto_kind = result?.auto_kind || undefined
			script.has_preprocessor = result?.has_preprocessor || undefined
		} catch {
			// Inference failures don't block deploys (the same fallback the
			// per-pane save uses). The createScript call is the real
			// validation gate — if the body is broken it'll reject there.
		}
		await ScriptService.createScript({
			workspace: ws,
			requestBody: {
				...script,
				language: script.language,
				description: script.description ?? '',
				// Drafts have no parent — they're brand new at this path.
				parent_hash: undefined,
				is_template: false,
				tag: script.tag,
				kind: script.kind as Script['kind'] | undefined,
				lock: undefined
			}
		})
	}

	async function saveAllDrafts() {
		if (!$workspaceStore || drafts.size === 0 || savingAll) return
		savingAll = true
		const ws = $workspaceStore
		const entries = [...drafts.entries()]
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
			for (const [k, v] of drafts) {
				if (!savedPaths.includes(k)) next.set(k, v)
			}
			drafts = next
			if (activeDraftPath && savedPaths.includes(activeDraftPath)) {
				activeDraftPath = undefined
			}
			await graphRes.refetch()
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
		if (!drafts.has(path)) return
		const next = new Map(drafts)
		next.delete(path)
		drafts = next
		if (activeDraftPath === path) activeDraftPath = undefined
		clearSaveError(path)
	}

	function clearSaveError(path: string) {
		if (!saveErrors.has(path)) return
		const next = new Map(saveErrors)
		next.delete(path)
		saveErrors = next
	}

	// Rename a draft in place — re-key its entry in the drafts map and
	// repoint activeDraftPath. Returns false (or an error string) if the
	// new path collides with another draft so the dialog can keep itself
	// open and surface the conflict inline.
	function renameDraft(oldPath: string, newPath: string): boolean | string {
		if (oldPath === newPath) return true
		const draft = drafts.get(oldPath)
		if (!draft) return 'Draft not found'
		if (drafts.has(newPath)) return 'Another draft already uses this path'
		const next = new Map<string, Draft>()
		// Preserve insertion order: replace the entry at its original
		// position so the canvas / lists don't reshuffle on rename.
		for (const [k, v] of drafts) {
			if (k === oldPath) {
				const updatedScript = { ...v.script, path: newPath }
				next.set(newPath, { ...v, script: updatedScript })
			} else {
				next.set(k, v)
			}
		}
		drafts = next
		if (activeDraftPath === oldPath) activeDraftPath = newPath
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
		return true
	}

	// Currently-open draft shape (if any) — fed into the details pane.
	let activeDraft = $derived(activeDraftPath ? drafts.get(activeDraftPath) : undefined)

	// Named handlers for the details pane's live callbacks. Inline arrows
	// would be rebuilt on every parent re-render, and the pane's $effects
	// track those refs as deps — combined with the drafts mutation in
	// `handleDraftContentChange`, that creates a parent ↔ child feedback
	// loop ("effect_update_depth_exceeded"). Named functions keep the
	// prop reference stable so the $effects only re-fire on real
	// content changes.
	function handleAnnotationsChange(
		scriptPath: string | undefined,
		annotations: PipelineAnnotations
	) {
		liveAnnotations = { scriptPath, annotations }
	}
	function handleAssetsChange(scriptPath: string | undefined, assets: AssetWithAltAccessType[]) {
		liveBodyAssets = { scriptPath, assets }
		// Seed the sticky cache so the script's writes survive selection
		// changes. Only the active script's entry is updated (the
		// scriptPath the WASM just parsed); other entries are untouched
		// so previously-seen scripts retain their last-known writes.
		// `untrack` around the read+write of `inferredWritesByPath` is
		// crucial: this fn is called from AssetGraphDetailsPane's $effect
		// for onAssetsChange, and Svelte tracks every state read inside
		// that effect's closure. Without `untrack`, cloning the Map
		// registers it as a dep, and writing the new Map immediately
		// re-fires the effect → infinite loop.
		if (scriptPath) {
			const writes = extractWrites(assets)
			const reads = extractReads(assets)
			untrack(() => {
				const nextW = new Map(inferredWritesByPath)
				if (writes.length > 0) nextW.set(scriptPath, writes)
				else nextW.delete(scriptPath)
				inferredWritesByPath = nextW
				const nextR = new Map(inferredReadsByPath)
				if (reads.length > 0) nextR.set(scriptPath, reads)
				else nextR.delete(scriptPath)
				inferredReadsByPath = nextR
			})
		}
	}
	function handleDraftPersist(
		p: string,
		snapshot: {
			content: string
			writes: { kind: AssetKind; path: string }[]
		}
	) {
		// Persist body edits + inferred outputs back into the drafts Map so
		// they survive switching to another node and back (the details pane
		// clones draftScript locally on every prop change, and `outputAsset`
		// would otherwise stay frozen at the value seeded when the draft
		// was opened — leaving a stale write edge on the canvas after the
		// user has renamed a CREATE TABLE / writeS3File target).
		const d = drafts.get(p)
		if (!d) return
		const writesEqual =
			d.outputAssets?.length === snapshot.writes.length &&
			(d.outputAssets ?? []).every(
				(a, i) => a.kind === snapshot.writes[i]?.kind && a.path === snapshot.writes[i]?.path
			)
		if (d.script.content === snapshot.content && writesEqual) return
		const next = new Map(drafts)
		next.set(p, {
			...d,
			script: { ...d.script, content: snapshot.content },
			outputAssets: snapshot.writes.length > 0 ? snapshot.writes : undefined
		})
		drafts = next
	}

	// Overlay the draft runnable + live-parsed trigger edges onto the fetched
	// graph. Live edges come from the editor buffer's `// on <spec>` lines
	// and are marked `unsaved: true` unless they already match a persisted
	// script_trigger row. This keeps the canvas in sync with the editor
	// keystroke-by-keystroke; saving replaces the live edges with real ones
	// via graphRes.refetch().
	let graphWithDraft = $derived.by<AssetGraphResponse>(() => {
		const base = graphRes.current ?? EMPTY_GRAPH

		// Every draft contributes: a runnable, an output asset, a write edge,
		// plus its own seeded schedule trigger (template includes `// on
		// schedule "0 * * * *"` by default, picked up through live parse).
		// We iterate the whole `drafts` map so multiple concurrent drafts
		// all render as their own subgraph at once.
		const runnables = [...base.runnables]
		const assets = [...base.assets]
		const edges = [...base.edges]
		const extraTriggers: AssetGraphResponse['triggers'] = []

		for (const [path, d] of drafts) {
			const parsed = parsePipelineAnnotations(d.script.content)
			runnables.push({
				path,
				usage_kind: 'script',
				in_pipeline: true,
				partition_kind: parsed.partition?.kind,
				freshness: parsed.freshness?.duration,
				unsaved: true
			})
			// Output asset(s): three-tier resolution.
			//   1. Active draft (the body the user is editing right now):
			//      live body inference is authoritative — renaming a
			//      CREATE TABLE target or writeS3File path retires the
			//      old output node and surfaces the new one as the user
			//      types.
			//   2. Inactive draft with a captured `outputAssets` snapshot
			//      (taken on the last pane transition): use those, so a
			//      draft the user already edited keeps its renamed outputs
			//      after they've clicked elsewhere.
			//   3. Fallback to the static `outputAsset` seeded at draft
			//      creation — covers fresh drafts and parser misses (e.g.
			//      WIN-1943: wmill.writeS3File({s3, storage}) object form
			//      not yet detected by the TS parser).
			const liveForThisDraft = liveBodyAssets.scriptPath === path
			const writeOuts: Array<{ kind: AssetKind; path: string }> = []
			if (liveForThisDraft) {
				writeOuts.push(...extractWrites(liveBodyAssets.assets))
			} else if (d.outputAssets) {
				writeOuts.push(...d.outputAssets)
			}
			if (writeOuts.length === 0 && d.outputAsset) {
				writeOuts.push(d.outputAsset)
			}
			for (const out of writeOuts) {
				const hasAsset = assets.some((a) => a.kind === out.kind && a.path === out.path)
				if (!hasAsset) assets.push({ kind: out.kind, path: out.path })
				edges.push({
					runnable_path: path,
					runnable_kind: 'script',
					asset_kind: out.kind,
					asset_path: out.path,
					access_type: 'w',
					unsaved: true
				})
			}
			// Seed trigger edges (schedule + asset) from the draft's template
			// so the graph stays stable when the user clicks off this draft.
			// Live annotations (below) take over for the currently-open draft
			// so keystroke edits still update in real time.
			for (const cron of parsed.schedules) {
				extraTriggers.push({
					trigger_kind: 'schedule',
					cron,
					runnable_kind: 'script',
					runnable_path: path,
					unsaved: true
				})
			}
			for (const a of parsed.triggerAssets) {
				extraTriggers.push({
					trigger_kind: 'asset',
					asset_kind: a.kind,
					asset_path: a.path,
					runnable_kind: 'script',
					runnable_path: path,
					unsaved: true
				})
				// Also synthesize the asset node so the trigger edge has a
				// target even if the upstream asset isn't in base (e.g. the
				// producer script is in another folder we haven't fetched
				// or also a draft).
				const hasTriggerAsset = assets.some((x) => x.kind === a.kind && x.path === a.path)
				if (!hasTriggerAsset) assets.push({ kind: a.kind, path: a.path })
			}
			for (const n of parsed.nativeTriggers) {
				extraTriggers.push({
					trigger_kind: n.kind,
					path: n.path,
					runnable_kind: 'script',
					runnable_path: path,
					unsaved: true
				})
			}
		}

		// Live-parsed overlay for the currently-open script — takes precedence
		// over the seeded-template triggers for the same path by swapping
		// them out. Scoped to one path (only one pane is open at a time).
		const livePath = liveAnnotations.scriptPath
		if (livePath) {
			const persistedAssetKeys = new Set(
				base.triggers
					.filter(
						(t) =>
							t.trigger_kind === 'asset' &&
							t.runnable_kind === 'script' &&
							t.runnable_path === livePath
					)
					.map((t) => (t.trigger_kind === 'asset' ? `${t.asset_kind}:${t.asset_path}` : ''))
			)
			const persistedScheduleCrons = new Set(
				base.triggers
					.filter(
						(t) =>
							t.trigger_kind === 'schedule' &&
							t.runnable_kind === 'script' &&
							t.runnable_path === livePath
					)
					.map((t) => (t.trigger_kind === 'schedule' ? t.cron : ''))
			)
			// Strip seeded triggers we computed above for the active draft;
			// live annotations are authoritative for the open buffer.
			for (let i = extraTriggers.length - 1; i >= 0; i--) {
				if (extraTriggers[i].runnable_path === livePath) extraTriggers.splice(i, 1)
			}
			for (const a of liveAnnotations.annotations.triggerAssets) {
				const key = `${a.kind}:${a.path}`
				if (persistedAssetKeys.has(key)) continue
				extraTriggers.push({
					trigger_kind: 'asset',
					asset_kind: a.kind,
					asset_path: a.path,
					runnable_kind: 'script',
					runnable_path: livePath,
					unsaved: true
				})
				// Synthesize the asset node so the new trigger edge has a
				// target — without this, typing `// on s3:///...` adds an
				// edge to a node that doesn't exist and the canvas silently
				// drops it. Mirrors the draft branch above.
				if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
					assets.push({ kind: a.kind, path: a.path })
				}
			}
			for (const cron of liveAnnotations.annotations.schedules) {
				if (persistedScheduleCrons.has(cron)) continue
				extraTriggers.push({
					trigger_kind: 'schedule',
					cron,
					runnable_kind: 'script',
					runnable_path: livePath,
					unsaved: true
				})
			}
			// Persisted native triggers keyed by `<kind>:<path>`, used to
			// suppress duplicate overlay for already-saved `// on <kind>`
			// annotations. trigger_kind is narrower than the union so we
			// cast through string.
			const persistedNativeKeys = new Set(
				base.triggers
					.filter(
						(t) =>
							t.trigger_kind !== 'asset' &&
							t.trigger_kind !== 'schedule' &&
							t.runnable_kind === 'script' &&
							t.runnable_path === livePath
					)
					.map((t) => `${t.trigger_kind}:${(t as { path: string }).path}`)
			)
			for (const n of liveAnnotations.annotations.nativeTriggers) {
				const key = `${n.kind}:${n.path}`
				if (persistedNativeKeys.has(key)) continue
				extraTriggers.push({
					trigger_kind: n.kind,
					path: n.path,
					runnable_kind: 'script',
					runnable_path: livePath,
					unsaved: true
				})
			}
		}

		// Live body-asset lineage for any persisted script inferred at least
		// once this session (maps filled by `handleAssetsChange` + the load
		// prefetch). Drafts are handled by the loop above. For scripts whose
		// deploy didn't persist their body assets (e.g. older WASM at save
		// time, or object-form writeS3File), this keeps the lineage edge on
		// the canvas across selection changes — not just while selected.
		const overlayLineage = (
			byPath: Map<string, Array<{ kind: AssetKind; path: string }>>,
			access: 'w' | 'r'
		) => {
			for (const [scriptPath, refs] of byPath) {
				if (drafts.has(scriptPath)) continue
				const persisted = new Set(
					base.edges
						.filter(
							(e) =>
								e.runnable_path === scriptPath &&
								e.runnable_kind === 'script' &&
								(e.access_type === access || e.access_type === 'rw')
						)
						.map((e) => `${e.asset_kind}:${e.asset_path}`)
				)
				for (const a of refs) {
					const key = `${a.kind}:${a.path}`
					if (persisted.has(key)) continue
					if (!assets.some((x) => x.kind === a.kind && x.path === a.path)) {
						assets.push({ kind: a.kind, path: a.path })
					}
					edges.push({
						runnable_path: scriptPath,
						runnable_kind: 'script',
						asset_kind: a.kind,
						asset_path: a.path,
						access_type: access,
						unsaved: true
					})
				}
			}
		}
		overlayLineage(inferredWritesByPath, 'w')
		overlayLineage(inferredReadsByPath, 'r')

		return { ...base, assets, runnables, edges, triggers: [...base.triggers, ...extraTriggers] }
	})

	// Selection highlights the active draft (if any) or the user's picked
	// node. Non-active drafts render without selection highlight but are
	// still clickable to re-enter their edit pane.
	let effectiveSelection = $derived<AssetGraphSelection | undefined>(
		activeDraftPath
			? { kind: 'runnable', runnable_kind: 'script', path: activeDraftPath }
			: selection
	)

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
	// Folder-scoped poll of in-flight (and just-finished) pipeline jobs. This
	// is what lights up the *downstream cascade* on the canvas — jobs the user
	// didn't launch directly. `arm()` is called when a run is launched from
	// this view; it polls only while jobs are in flight and stops when idle
	// (zero requests at rest). Re-scoped/torn down when the folder changes or
	// the page unmounts.
	const activeRunnables = useActiveRunnableIds(
		() => $workspaceStore,
		() => pathPrefix
	)
	$effect(() => {
		pathPrefix // re-scope poll to the current folder
		return () => activeRunnables.dispose()
	})
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

	// Producers (write/rw edges) for the currently-selected asset, derived
	// from `graphWithDraft.edges`. Threaded into the details pane so the
	// runs panel can list jobs for the right scripts. We include drafts —
	// running a draft via runScriptPreview creates a `preview`-kind job at
	// the same path, which the panel's listing query picks up.
	let selectionProducers = $derived.by(() => {
		const sel = selection
		if (!sel || sel.kind !== 'asset') return []
		return graphWithDraft.edges
			.filter((e) => {
				const access = e.access_type ?? 'r'
				return (
					(access === 'w' || access === 'rw') &&
					e.asset_kind === sel.asset_kind &&
					e.asset_path === sel.path
				)
			})
			.map((e) => ({ kind: e.runnable_kind, path: e.runnable_path, unsaved: e.unsaved }))
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
		const editedPath =
			activeDraftPath ??
			(selection?.kind === 'runnable' && selection.runnable_kind === 'script'
				? selection.path
				: undefined)
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
		const items = otherPipelineFolders.map((p) => ({
			displayName: `f/${p.folder}`,
			icon: Folder,
			disabled: false,
			action: () => goto(`${base}/pipeline/${encodeURIComponent(p.folder)}`)
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

	// Eagerly infer every folder script's writes on load and seed the same
	// `inferredWritesByPath` overlay the open-script path uses — otherwise a
	// script whose persisted asset rows are missing only gets edges when
	// clicked, which re-layouts the graph. WHY untrack: deps must stay
	// (workspace, base-graph) only so a keystroke/new draft doesn't re-sweep;
	// the generation token cancels an in-flight sweep on folder change.
	let assetPrefetchGen = 0
	// True while the load-time sweep above is still parsing scripts — drives
	// a small "parsing assets…" hint so the user knows the graph is still
	// settling (edges may still appear) rather than already complete.
	let prefetchingAssets = $state(false)
	$effect(() => {
		const ws = $workspaceStore
		const g = graphRes.current
		if (!ws || !g) return
		const gen = ++assetPrefetchGen
		const targets = untrack(() =>
			g.runnables
				.filter((r) => r.usage_kind === 'script')
				.map((r) => r.path)
				.filter(
					(p) => !drafts.has(p) && !inferredWritesByPath.has(p) && !inferredReadsByPath.has(p)
				)
		)
		if (targets.length === 0) return
		let i = 0
		const POOL = 6
		const worker = async () => {
			while (i < targets.length && gen === assetPrefetchGen) {
				const path = targets[i++]
				try {
					const s = await ScriptService.getScriptByPath({ workspace: ws, path })
					if (gen !== assetPrefetchGen) return
					const res = await inferAssets(s.language, s.content ?? '')
					if (gen !== assetPrefetchGen) return
					const inferred = (res?.assets ?? []) as AssetWithAltAccessType[]
					const writes = extractWrites(inferred)
					const reads = extractReads(inferred)
					untrack(() => {
						// A live edit / prior sweep may have filled either meanwhile.
						if (writes.length > 0 && !inferredWritesByPath.has(path)) {
							const next = new Map(inferredWritesByPath)
							next.set(path, writes)
							inferredWritesByPath = next
						}
						if (reads.length > 0 && !inferredReadsByPath.has(path)) {
							const next = new Map(inferredReadsByPath)
							next.set(path, reads)
							inferredReadsByPath = next
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
			// Only clear for the sweep still current — a folder change starts
			// a new gen (and its own true) that this stale resolve mustn't undo.
			if (gen === assetPrefetchGen) prefetchingAssets = false
		})
	})

	function pluralize(n: number, singular: string): string {
		return `${n} ${singular}${n === 1 ? '' : 's'}`
	}

	let summary = $derived.by<string[]>(() => {
		const g = graphRes.current
		if (!g) return []
		const parts: string[] = []
		const scripts = g.runnables.filter((r) => r.usage_kind === 'script').length
		const flows = g.runnables.filter((r) => r.usage_kind === 'flow').length
		if (scripts) parts.push(pluralize(scripts, 'script'))
		if (flows) parts.push(pluralize(flows, 'flow'))
		const byKind = new Map<string, number>()
		for (const a of g.assets) byKind.set(a.kind, (byKind.get(a.kind) ?? 0) + 1)
		for (const [kind, n] of byKind) parts.push(pluralize(n, kind))
		return parts
	})
</script>

<svelte:head>
	<title>Pipeline · {folder} — Windmill</title>
</svelte:head>

{#if $userStore?.operator}
	<div class="p-8 text-tertiary">Page not available for operators.</div>
{:else}
	<div class="flex flex-col h-full">
		<div
			class="border-b flex flex-row justify-between gap-2 px-2 py-1 items-center overflow-y-visible overflow-x-auto min-h-10 shrink-0 whitespace-nowrap"
		>
			<div class="flex flex-row items-center gap-2">
				<Button
					variant="subtle"
					unifiedSize="sm"
					href="{base}/pipeline"
					startIcon={{ icon: ArrowLeft }}
					iconOnly
					title="Back to pipelines"
				/>
				<NetworkIcon size={16} class="text-tertiary shrink-0" />
				<h1 class="text-sm font-semibold">Pipeline editor</h1>
				<span class="text-tertiary text-sm">·</span>
				{#if otherPipelineFolders.length === 0}
					<button
						type="button"
						onclick={() => (pickerModalOpen = true)}
						class="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-gray-300 dark:border-gray-600 bg-surface hover:bg-surface-hover transition-colors"
						title="Switch pipeline folder"
					>
						<Folder size={14} class="text-emerald-600 dark:text-emerald-400 shrink-0" />
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
								<Folder size={14} class="text-emerald-600 dark:text-emerald-400 shrink-0" />
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
			<div class="flex flex-row items-center gap-2">
				{#if saveErrors.size > 0}
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
				{#if drafts.size > 0}
					<Button
						variant="accent"
						unifiedSize="sm"
						startIcon={{ icon: savingAll ? Loader2 : Save }}
						onclick={saveAllDrafts}
						disabled={savingAll}
						title={savingAll ? 'Saving drafts…' : `Deploy all ${drafts.size} drafts`}
					>
						{savingAll ? 'Saving…' : `Save all (${drafts.size})`}
					</Button>
				{/if}
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
				<Splitpanes class="!h-full">
					<Pane bind:size={leftPaneSize}>
						<div class="relative h-full">
							<AssetGraphCanvas
								graph={graphWithDraft}
								selection={effectiveSelection}
								{activeRunnable}
								activeRunnableIds={activeRunnables.ids}
								runStates={activeRunnables.states}
								{pathPrefix}
								defaultPathSuffix={DEFAULT_PATH_SUFFIX}
								defaultScheduleCron={DEFAULT_SCHEDULE_CRON}
								onselect={(s) => {
									// Clicking a draft runnable node re-opens it in the pane;
									// clicking anything else selects it normally and detaches
									// from any active draft (drafts stay overlaid until saved
									// or discarded).
									if (
										s &&
										s.kind === 'runnable' &&
										s.runnable_kind === 'script' &&
										drafts.has(s.path)
									) {
										activeDraftPath = s.path
										selection = undefined
									} else {
										activeDraftPath = undefined
										selection = s
									}
								}}
								onAddScriptForAsset={(asset, language, scriptPath, outputKind, aiPrompt) => {
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
								}}
								onAddPipelineScript={(language, scriptPath, source, outputKind, aiPrompt) =>
									openMaterializerDraft(
										language,
										scriptPath,
										[source],
										outputKind,
										undefined,
										aiPrompt
									)}
								onRunnableMenuRemove={(info) => {
									// Drafts: drop the local entry immediately — the
									// existing onDiscard pathway already handles this
									// without a confirm modal. Persisted: select the
									// script (so the pane loads it), then bump the
									// remove-signal counter so the pane opens its
									// archive/delete modal.
									if (info.unsaved) {
										discardDraft(info.path)
										return
									}
									if (info.runnable_kind !== 'script') return
									activeDraftPath = undefined
									selection = { kind: 'runnable', runnable_kind: 'script', path: info.path }
									requestRemoveSignal++
								}}
								onRunProducer={async (producer) => {
									// Saved scripts go through runScriptByPath; drafts
									// have no DB row yet, so dispatch to runScriptPreview
									// with the locally-cached content/language. Keeping
									// this dispatch in the page (rather than the canvas
									// or AssetNode) so the asset-graph components stay
									// stateless wrt drafts. Bump runsRefreshKey on
									// success so the runs panel picks up the new job
									// immediately — its background poll only kicks in
									// for already-listed in-flight jobs.
									if (!$workspaceStore || producer.kind !== 'script') return undefined
									// Start the folder-scoped poll so the downstream
									// asset-trigger cascade (jobs not launched here)
									// lights up its edges as it fans out.
									activeRunnables.arm()
									// Cascade default: same as the Test button — `cascade`
									// undefined / false skips the asset-trigger dispatch
									// via `_wmill_skip_asset_dispatch`; explicit `true`
									// lets the dispatch fire normally. The asset-node
									// affordance still passes `undefined` (legacy callers),
									// which we treat as "skip" for consistency.
									const cascade = producer.cascade === true
									// If the producer being run is the script currently
									// edited in the pane, route through ScriptEditor's
									// Test path — the test panel then shows logs/result
									// and the user can cancel from there. Same UX as
									// hitting the Test button directly.
									const openPath =
										activeDraftPath ??
										(selection?.kind === 'runnable' && selection.runnable_kind === 'script'
											? selection.path
											: undefined)
									if (openPath === producer.path) {
										if (cascade) requestRunCascadeSignal++
										else requestRunSignal++
										return undefined
									}
									const skipArg = cascade ? {} : { _wmill_skip_asset_dispatch: true }
									let jobId: string | undefined
									if (producer.unsaved) {
										const draft = drafts.get(producer.path)
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
									}
									return jobId
								}}
							/>
							<PipelineEventLog
								events={activeRunnables.events}
								onToggle={(o) => activeRunnables.setObserving(o)}
							/>
							{#if prefetchingAssets}
								<div
									class="absolute top-2 left-1/2 -translate-x-1/2 z-20 flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface/95 backdrop-blur-sm border border-gray-200 dark:border-gray-700 text-2xs text-secondary shadow-sm"
									title="Inferring assets for every script in this folder so the graph is complete"
								>
									<Loader2 size={11} class="animate-spin" />
									Parsing assets…
								</div>
							{/if}
							{#if selection != undefined || activeDraftPath != undefined}
								<!-- Floating panel toggle, mirrors the app builder's
								     hide-bar pattern. Anchored to the canvas (not the
								     toolbar) so it stays adjacent to the splitter
								     handle. z-50 + bg-surface keep it visible above
								     the SvelteFlow internals (panels, edges, controls
								     all render at z>10) and against the canvas
								     background — the bare HideButton uses
								     bg-transparent which disappears on light themes. -->
								<div
									class="absolute top-2 right-2 z-50 rounded-md bg-surface shadow-sm border border-gray-200 dark:border-gray-700"
								>
									<HideButton
										hidden={panelHidden}
										direction="right"
										on:click={() => (panelHidden = !panelHidden)}
									/>
								</div>
							{/if}
						</div></Pane
					>
					{#if detailsPaneOpen && $workspaceStore}
						<Pane bind:size={rightPaneSize} minSize={25}>
							<AssetGraphDetailsPane
								selection={activeDraft ? undefined : selection}
								selectionProducers={activeDraft ? [] : selectionProducers}
								{runsRefreshKey}
								{runsPendingJobId}
								{activeRunnable}
								downstreamSubscribers={editedScriptDownstreamCount}
								onRunCompleted={() => (activeRunnable = undefined)}
								onTestStateChange={(running) => {
									// Bridge: ScriptEditor's Test button triggers the
									// same canvas-level "is running" hint as the
									// per-node Run button. The currently-edited script
									// is whichever path is open in the pane (active
									// draft, or the persisted-script selection).
									const openPath =
										activeDraftPath ??
										(selection?.kind === 'runnable' && selection.runnable_kind === 'script'
											? selection.path
											: undefined)
									if (running && openPath) {
										activeRunnable = { kind: 'script', path: openPath }
									} else if (!running && activeRunnable?.path === openPath) {
										activeRunnable = undefined
									}
								}}
								{requestRemoveSignal}
								{requestRunSignal}
								{requestRunCascadeSignal}
								draftScript={activeDraft?.script}
								{pathPrefix}
								onDraftPathChange={renameDraft}
								workspace={$workspaceStore}
								onAnnotationsChange={handleAnnotationsChange}
								onAssetsChange={handleAssetsChange}
								onDraftPersist={handleDraftPersist}
								onclose={() => {
									// Close dismisses the pane but preserves drafts so
									// the user can come back to them. Discarding is
									// via the explicit "Discard" button in the pane.
									selection = undefined
									activeDraftPath = undefined
									liveAnnotations = {
										scriptPath: undefined,
										annotations: {
											inPipeline: false,
											triggerAssets: [],
											schedules: [],
											nativeTriggers: []
										}
									}
								}}
								onHide={() => (panelHidden = true)}
								onDiscard={() => {
									if (activeDraftPath) discardDraft(activeDraftPath)
								}}
								onDraftSaved={async (savedPath) => {
									discardDraft(savedPath)
									await graphRes.refetch()
								}}
								onPersistedSaved={async () => {
									// Refresh the asset graph so the rows the deploy
									// just inserted (from the body-asset write list we
									// pass at save time) make it into base.edges. The
									// in-memory `inferredWritesByPath` overlay
									// dedupes against base, so the edge stays put
									// instead of flickering when the ScriptEditor
									// remounts on the new hash.
									await graphRes.refetch()
								}}
								onScriptRenamed={async (oldPath, newPath) => {
									// Repoint the selection at the new path before the
									// graph refetches so the pane stays focused on the
									// same script. Order matters: update selection
									// first, then refetch — otherwise the resource
									// driving the pane would briefly resolve to nothing.
									if (selection?.kind === 'runnable' && selection.path === oldPath) {
										selection = { ...selection, path: newPath }
									}
									await graphRes.refetch()
								}}
								onScriptRemoved={async (removedPath) => {
									// Drop the selection (the script is gone) and
									// refetch so the runnable node disappears from
									// the canvas.
									if (selection?.kind === 'runnable' && selection.path === removedPath) {
										selection = undefined
									}
									await graphRes.refetch()
								}}
							/>
						</Pane>
					{/if}
				</Splitpanes>
			{/if}
		</div>
	</div>

	<PipelinePickerModal bind:open={pickerModalOpen} currentFolder={folder} />

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
									{drafts.size === 1 ? 'Unsaved draft' : `${drafts.size} unsaved drafts`}
								</h3>
								<div class="mt-2 text-sm text-secondary flex flex-col gap-2">
									<p>
										You have {drafts.size === 1
											? 'a draft pipeline script'
											: `${drafts.size} draft pipeline scripts`} that {drafts.size === 1
											? 'has'
											: 'have'} not been deployed yet. What would you like to do?
									</p>
									<ul
										class="text-2xs font-mono pl-4 max-h-40 overflow-y-auto flex flex-col gap-0.5"
									>
										{#each [...drafts.keys()] as p}
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
								<span class="min-w-20">{leaveSaving ? 'Saving…' : `Save all (${drafts.size})`}</span
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
{/if}
