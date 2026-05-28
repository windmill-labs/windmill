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
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import {
		extractWrites,
		extractReads,
		type AssetWithAltAccessType
	} from '$lib/components/assets/lib'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind
	} from '$lib/components/assets/AssetGraph/types'
	import {
		parsePipelineAnnotations,
		type PipelineAnnotations
	} from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
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
		EmailTriggerService,
		GcpTriggerService,
		JobService,
		KafkaTriggerService,
		MqttTriggerService,
		NatsTriggerService,
		OpenAPI,
		PostgresTriggerService,
		ScheduleService,
		ScriptService,
		SqsTriggerService,
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
	import KafkaTriggerEditor from '$lib/components/triggers/kafka/KafkaTriggerEditor.svelte'
	import MqttTriggerEditor from '$lib/components/triggers/mqtt/MqttTriggerEditor.svelte'
	import NatsTriggerEditor from '$lib/components/triggers/nats/NatsTriggerEditor.svelte'
	import PostgresTriggerEditor from '$lib/components/triggers/postgres/PostgresTriggerEditor.svelte'
	import SqsTriggerEditor from '$lib/components/triggers/sqs/SqsTriggerEditor.svelte'
	import GcpTriggerEditor from '$lib/components/triggers/gcp/GcpTriggerEditor.svelte'
	import EmailTriggerEditor from '$lib/components/triggers/email/EmailTriggerEditor.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import WebhookEditor from '$lib/components/triggers/webhook/WebhookEditor.svelte'

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
	// In-flight drafts keyed by script path. Multiple can coexist — clicking
	// + repeatedly creates additional drafts, each with its own random
	// output asset, and they all render on the graph simultaneously.
	// Saving removes a draft from the map; closing the pane keeps it so the
	// user can come back to it.
	// Counter-based id source — sufficient for "stable across renames in
	// this session"; doesn't need to survive a reload. (We have crypto.
	// randomUUID() too but a short numeric id keeps localStorage tidy.)
	let nextDraftLocalIdCounter = 0
	function newDraftLocalId(): string {
		nextDraftLocalIdCounter += 1
		return `d${nextDraftLocalIdCounter}-${Date.now()}`
	}

	type Draft = {
		// Stable per-draft identifier, generated on first create and
		// preserved across renames. Used to track concurrent deploys (a
		// fast double-rename otherwise fires two saves that each leave a
		// persisted script behind — the latest deploy archives the prior
		// one keyed on this id).
		localId: string
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
						const d = entry[1] as Draft
						// Backfill localId for state persisted by older builds.
						if (typeof d.localId !== 'string' || d.localId === '') {
							d.localId = newDraftLocalId()
						}
						loaded.set(entry[0], d)
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
	let inferredWritesByPath = $derived.by(() => {
		const out = new Map<string, Array<{ kind: AssetKind; path: string }>>()
		const g = graphRes.current
		if (!g) return out
		const liveAssetsForPath = (path: string) =>
			liveBodyAssets.scriptPath === path ? liveBodyAssets.assets : inferredAssetsByPath.get(path)
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			const assets = liveAssetsForPath(r.path)
			if (!assets) continue
			const w = extractWrites(assets)
			if (w.length > 0) out.set(r.path, w)
		}
		return out
	})
	let inferredReadsByPath = $derived.by(() => {
		const out = new Map<string, Array<{ kind: AssetKind; path: string }>>()
		const g = graphRes.current
		if (!g) return out
		const liveAssetsForPath = (path: string) =>
			liveBodyAssets.scriptPath === path ? liveBodyAssets.assets : inferredAssetsByPath.get(path)
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			const assets = liveAssetsForPath(r.path)
			if (!assets) continue
			const reads = extractReads(assets)
			if (reads.length > 0) out.set(r.path, reads)
		}
		return out
	})
	// Same derived shape for `// on kafka` etc. annotations. Live buffer
	// wins for the open script; everyone else is parsed from the
	// prefetched body content.
	let annotatedNativeKindsByPath = $derived.by(() => {
		const out = new Map<string, Set<NativeTriggerKind>>()
		const g = graphRes.current
		if (!g) return out
		const livePath = liveAnnotations.scriptPath
		for (const r of g.runnables) {
			if (r.usage_kind !== 'script') continue
			let kinds: Set<NativeTriggerKind>
			if (r.path === livePath) {
				kinds = new Set(liveAnnotations.annotations.nativeTriggers.map((n) => n.kind))
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
		next.set(scriptPath, { localId: newDraftLocalId(), script, outputAsset: out })
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
			// If the open draft just got deployed, transfer the focus to
			// its now-persisted runnable so the pane stays on the same
			// script the user was editing — otherwise the pane closes,
			// the canvas re-fits, and the user has to re-find their
			// script after every save.
			if (activeDraftPath && savedPaths.includes(activeDraftPath)) {
				selection = {
					kind: 'runnable',
					runnable_kind: 'script',
					path: activeDraftPath
				}
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
		if (activeDraftPath === path) activeDraftPath = undefined
		if (selection?.kind === 'runnable' && selection.path === path) {
			selection = undefined
		}
		if (liveAnnotations.scriptPath === path) {
			liveAnnotations = {
				scriptPath: undefined,
				annotations: { inPipeline: false, triggerAssets: [], nativeTriggers: [] }
			}
		}
		if (liveBodyAssets.scriptPath === path) {
			liveBodyAssets = { scriptPath: undefined, assets: [] }
		}
		clearSaveError(path)
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
		// Path-keyed live overlays: re-key for the renamed draft so the
		// graph stays consistent between the moment we mutate `drafts`
		// here and the next editor event that re-emits annotations /
		// asset usages with the new path. Without this re-key,
		// `resolveGraph` strips seeded triggers for the OLD path while
		// re-applying live overlays against the same OLD path — leaving
		// phantom edges that displace the + node off the top of the
		// graph and shuffle the layout.
		if (liveAnnotations.scriptPath === oldPath) {
			liveAnnotations = { ...liveAnnotations, scriptPath: newPath }
		}
		if (liveBodyAssets.scriptPath === oldPath) {
			liveBodyAssets = { ...liveBodyAssets, scriptPath: newPath }
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
		const draft = drafts.get(path)
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
				const draft = drafts.get(path)
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
						const nextDrafts = new Map(drafts)
						nextDrafts.delete(path)
						drafts = nextDrafts
						if (activeDraftPath === path) {
							selection = { kind: 'runnable', runnable_kind: 'script', path }
							activeDraftPath = undefined
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
			if (!drafts.has(path) && !state.queuedPath) {
				deployQueue.delete(localId)
			}
		}
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
		// Single update site for the live overlay. `inferredWritesByPath`
		// / `inferredReadsByPath` are now derived from `liveBodyAssets`
		// (for the open script) + `inferredAssetsByPath` (prefetched
		// snapshot for every other script), so we don't have to write
		// into those caches here — the derive picks up our update on the
		// next reactive tick.
		liveBodyAssets = { scriptPath, assets }
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
	let graphWithDraft = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: graphRes.current ?? EMPTY_GRAPH,
			drafts,
			liveBodyAssets,
			liveAnnotations,
			inferredWritesByPath,
			inferredReadsByPath,
			annotatedNativeKindsByPath
		})
	)

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

	// Native trigger editors mounted inline so clicking a "missing"
	// placeholder opens the matching drawer with `script_path` pre-filled
	// — keeps pipeline drafts intact instead of navigating away. Each
	// editor's wrapper lazily mounts its Inner only when `open=true`, so
	// holding refs to all seven is cheap.
	let kafkaEditor: KafkaTriggerEditor | undefined = $state()
	let mqttEditor: MqttTriggerEditor | undefined = $state()
	let natsEditor: NatsTriggerEditor | undefined = $state()
	let postgresEditor: PostgresTriggerEditor | undefined = $state()
	let sqsEditor: SqsTriggerEditor | undefined = $state()
	let gcpEditor: GcpTriggerEditor | undefined = $state()
	let emailEditor: EmailTriggerEditor | undefined = $state()
	let scheduleEditor: ScheduleEditor | undefined = $state()
	let webhookEditor: WebhookEditor | undefined = $state()

	// Webhooks have no trigger row to create — clicking the node opens a
	// drawer with the endpoint URLs + the webhook-specific token creation
	// flow. Drafts have no deployed endpoint yet, so nudge the user to save
	// first (mirrors openMissingTriggerDrawer).
	function openWebhookDrawer(scriptPath: string) {
		if (drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — webhooks only trigger the deployed version.`,
				true
			)
			return
		}
		webhookEditor?.openDrawer(scriptPath, false)
	}

	function openMissingTriggerDrawer(kind: NativeTriggerKind, scriptPath: string) {
		// A native trigger row stores `script_path` as a hard reference —
		// pointing it at a never-saved draft would either fail at create
		// time or silently bind to nothing. Surface that as a toast and
		// keep the drawer closed; the user needs to save the script first
		// (which also creates it under the new path if they renamed it).
		if (drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — triggers can only be attached to deployed scripts.`,
				true
			)
			return
		}
		switch (kind) {
			case 'schedule':
				return scheduleEditor?.openNew(false, scriptPath, undefined, scriptPath)
			case 'kafka':
				return kafkaEditor?.openNew(false, scriptPath)
			case 'mqtt':
				return mqttEditor?.openNew(false, scriptPath)
			case 'nats':
				return natsEditor?.openNew(false, scriptPath)
			case 'postgres':
				return postgresEditor?.openNew(false, scriptPath)
			case 'sqs':
				return sqsEditor?.openNew(false, scriptPath)
			case 'gcp':
				return gcpEditor?.openNew(false, scriptPath)
			case 'email':
				return emailEditor?.openNew(false, scriptPath)
			// webhook has no dedicated editor.
			default:
				return
		}
	}

	// Lock the script-picker to the related script so the user can't
	// reassign the trigger off this pipeline from the canvas. The trigger
	// can still be edited everywhere else (TriggersPanel, etc.) where the
	// picker stays editable.
	// Delete-trigger confirmation state. Kebab → Delete opens the standard
	// ConfirmationModal; the actual delete is dispatched from onConfirmed
	// so the dialog stays consistent with the rest of the app.
	let triggerDeleteTarget = $state<{ kind: NativeTriggerKind; path: string } | undefined>(undefined)
	let triggerDeleteLoading = $state(false)
	let triggerDeleteOpen = $derived(triggerDeleteTarget != undefined)

	function deleteAttachedTrigger(kind: NativeTriggerKind, triggerPath: string) {
		triggerDeleteTarget = { kind, path: triggerPath }
	}

	async function confirmDeleteAttachedTrigger() {
		if (!triggerDeleteTarget || !$workspaceStore) return
		const { kind, path: triggerPath } = triggerDeleteTarget
		const workspace = $workspaceStore
		triggerDeleteLoading = true
		try {
			switch (kind) {
				case 'schedule':
					await ScheduleService.deleteSchedule({ workspace, path: triggerPath })
					break
				case 'kafka':
					await KafkaTriggerService.deleteKafkaTrigger({ workspace, path: triggerPath })
					break
				case 'mqtt':
					await MqttTriggerService.deleteMqttTrigger({ workspace, path: triggerPath })
					break
				case 'nats':
					await NatsTriggerService.deleteNatsTrigger({ workspace, path: triggerPath })
					break
				case 'postgres':
					await PostgresTriggerService.deletePostgresTrigger({ workspace, path: triggerPath })
					break
				case 'sqs':
					await SqsTriggerService.deleteSqsTrigger({ workspace, path: triggerPath })
					break
				case 'gcp':
					await GcpTriggerService.deleteGcpTrigger({ workspace, path: triggerPath })
					break
				case 'email':
					await EmailTriggerService.deleteEmailTrigger({ workspace, path: triggerPath })
					break
				default:
					return
			}
			sendUserToast(`Deleted ${kind} trigger "${triggerPath}"`)
			triggerDeleteTarget = undefined
			await graphRes.refetch()
		} catch (e: any) {
			sendUserToast(
				`Could not delete ${kind} trigger "${triggerPath}": ${e?.body ?? e?.message ?? String(e)}`,
				true
			)
		} finally {
			triggerDeleteLoading = false
		}
	}

	function openEditTriggerDrawer(kind: NativeTriggerKind, triggerPath: string, scriptPath: string) {
		switch (kind) {
			case 'schedule':
				return scheduleEditor?.openEdit(triggerPath, false, scriptPath)
			case 'kafka':
				return kafkaEditor?.openEdit(triggerPath, false, scriptPath)
			case 'mqtt':
				return mqttEditor?.openEdit(triggerPath, false, scriptPath)
			case 'nats':
				return natsEditor?.openEdit(triggerPath, false, scriptPath)
			case 'postgres':
				return postgresEditor?.openEdit(triggerPath, false, scriptPath)
			case 'sqs':
				return sqsEditor?.openEdit(triggerPath, false, scriptPath)
			case 'gcp':
				return gcpEditor?.openEdit(triggerPath, false, scriptPath)
			case 'email':
				return emailEditor?.openEdit(triggerPath, false, scriptPath)
			default:
				return
		}
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
				.filter((p) => !drafts.has(p) && !bodiesByPath.has(p))
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
								onCreateMissingTrigger={openMissingTriggerDrawer}
								onEditTrigger={openEditTriggerDrawer}
								onDeleteTrigger={deleteAttachedTrigger}
								onOpenWebhook={openWebhookDrawer}
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
									// lights up its edges as it fans out. Pass the
									// launched id so the catch-up pulse doesn't
									// re-flash it (the page already animates it
									// zero-latency via `activeRunnable`).
									activeRunnables.arm(`${producer.kind}:${producer.path}`)
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
											nativeTriggers: []
										}
									}
								}}
								onHide={() => (panelHidden = true)}
								onDiscard={() => {
									if (activeDraftPath) discardDraft(activeDraftPath)
								}}
								onDraftSaved={async (savedPath) => {
									// Drop the now-deployed draft and hand focus to its
									// persisted runnable so the pane stays open on the
									// same script. `discardDraft` would clear
									// activeDraftPath without setting selection — the
									// canvas would deselect and the view reset on the
									// next refetch.
									const nextDrafts = new Map(drafts)
									nextDrafts.delete(savedPath)
									drafts = nextDrafts
									if (activeDraftPath === savedPath) {
										selection = {
											kind: 'runnable',
											runnable_kind: 'script',
											path: savedPath
										}
										activeDraftPath = undefined
									}
									clearSaveError(savedPath)
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
									// Drop every path-keyed overlay / cache entry
									// pointing at the now-archived runnable so
									// resolveGraph doesn't keep emitting lineage
									// edges or missing-trigger placeholders against
									// a script that no longer exists. Without this
									// the inferred writes / annotation maps would
									// keep dragging phantom nodes onto the canvas
									// until the next folder change.
									forgetPath(removedPath)
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

	<ConfirmationModal
		open={triggerDeleteOpen}
		loading={triggerDeleteLoading}
		title={triggerDeleteTarget ? `Delete ${triggerDeleteTarget.kind} trigger` : 'Delete trigger'}
		confirmationText="Delete"
		onConfirmed={confirmDeleteAttachedTrigger}
		onCanceled={() => {
			if (!triggerDeleteLoading) triggerDeleteTarget = undefined
		}}
	>
		{#if triggerDeleteTarget}
			<p>
				Delete <code class="font-mono">{triggerDeleteTarget.path}</code>? The
				<code class="font-mono">// on {triggerDeleteTarget.kind}</code> annotation on the script stays
				— the trigger will read as missing on the canvas until you recreate it or remove the annotation.
			</p>
		{/if}
	</ConfirmationModal>

	<!-- Native trigger editors mounted off-screen. Each only renders its
	     inner drawer when `open=true` (set by openNew/openEdit), so this
	     adds ~zero render cost while idle. `onUpdate` refreshes the graph
	     so the new trigger row replaces the red missing placeholder. -->
	<KafkaTriggerEditor bind:this={kafkaEditor} onUpdate={() => graphRes.refetch()} />
	<MqttTriggerEditor bind:this={mqttEditor} onUpdate={() => graphRes.refetch()} />
	<NatsTriggerEditor bind:this={natsEditor} onUpdate={() => graphRes.refetch()} />
	<PostgresTriggerEditor bind:this={postgresEditor} onUpdate={() => graphRes.refetch()} />
	<SqsTriggerEditor bind:this={sqsEditor} onUpdate={() => graphRes.refetch()} />
	<GcpTriggerEditor bind:this={gcpEditor} onUpdate={() => graphRes.refetch()} />
	<EmailTriggerEditor bind:this={emailEditor} onUpdate={() => graphRes.refetch()} />
	<ScheduleEditor bind:this={scheduleEditor} onUpdate={() => graphRes.refetch()} />
	<WebhookEditor bind:this={webhookEditor} />

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
