<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { page } from '$app/state'
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import AssetGraphDetailsPane from '$lib/components/assets/AssetGraph/AssetGraphDetailsPane.svelte'
	import PipelinePickerModal from '$lib/components/assets/AssetGraph/PipelinePickerModal.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import {
		parsePipelineAnnotations,
		type PipelineAnnotations
	} from '$lib/components/assets/AssetGraph/parsePipelineAnnotations'
	import { decodeState, encodeState } from '$lib/utils'
	import { onMount, untrack } from 'svelte'
	import {
		ArrowLeft,
		ChevronDown,
		Folder,
		FolderSearch,
		Loader2,
		NetworkIcon,
		RefreshCw
	} from 'lucide-svelte'
	import { OpenAPI, type AssetKind, type Script, type ScriptLang } from '$lib/gen'
	import { initialCode } from '$lib/script_helpers'
	import { resource } from 'runed'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { emptySchema } from '$lib/utils'
	import { goto } from '$app/navigation'

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
	const DEFAULT_PATH_SUFFIX = 'new_materializer'
	// Default cron for top + materializers (pipeline roots). Every hour is a
	// sane middle ground between batch and real-time; users edit the
	// `// on schedule "..."` line in the editor before saving if they want
	// something different. Per-asset materializers don't get a schedule by
	// default — they inherit their trigger from the upstream asset.
	const DEFAULT_SCHEDULE_CRON = '0 * * * *'

	// In-flight drafts keyed by script path. Multiple can coexist — clicking
	// + repeatedly creates additional drafts, each with its own random
	// output asset, and they all render on the graph simultaneously.
	// Saving removes a draft from the map; closing the pane keeps it so the
	// user can come back to it.
	type Draft = {
		script: Script
		outputAsset: { kind: AssetKind; path: string }
	}
	let drafts = $state<Map<string, Draft>>(new Map())

	// Which draft (if any) is currently open in the details pane. When
	// undefined and `selection` is set, the pane shows the persisted
	// selection's script. Never both at once.
	let activeDraftPath = $state<string | undefined>(undefined)

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
		const serialized = Array.from(drafts.entries())
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
			isMaterializer: false,
			triggerAssets: [],
			schedules: [],
			nativeTriggers: []
		}
	})

	// Crockford-ish random slug for asset paths. 7 chars of [a-z0-9] gives
	// ~36^7 ≈ 7.8e10 combinations — collision-free in practice for the
	// handful of scripts a user creates in a session.
	function randomSlug(len = 7): string {
		const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789'
		let out = ''
		for (let i = 0; i < len; i++) {
			out += alphabet[Math.floor(Math.random() * alphabet.length)]
		}
		return out
	}

	function randomOutputAssetPath(): { kind: AssetKind; path: string } {
		// `s3://` is the most universal kind; works as a literal across every
		// supported language without extra imports. Inside `pipelines/<folder>/`
		// so outputs of a single pipeline cluster under one prefix.
		return { kind: 's3object', path: `pipelines/${folder}/out_${randomSlug()}.parquet` }
	}

	// Language → comment prefix recognized by parse_pipeline_annotations.
	function commentPrefix(lang: ScriptLang): string {
		switch (lang) {
			case 'python3':
			case 'bash':
			case 'powershell':
			case 'nu':
			case 'ansible':
				return '#'
			case 'postgresql':
			case 'mysql':
			case 'bigquery':
			case 'snowflake':
			case 'mssql':
			case 'oracledb':
			case 'duckdb':
				return '--'
			default:
				return '//'
		}
	}

	// Each entry becomes a `// on <…>` line in the seeded template. Multiple
	// trigger sources are valid (a script can be fired by both a schedule
	// and a webhook, for instance), but the menu only seeds one at a time.
	type DraftTriggerSource =
		| { kind: 'schedule'; cron: string }
		| { kind: 'asset'; ref: string } // already-prefixed (e.g. s3://…)
		| {
				kind: 'webhook' | 'email' | 'kafka' | 'mqtt' | 'nats' | 'postgres' | 'sqs' | 'gcp'
				path: string | undefined
		  }

	// Short "how to author a materializer" doc block prepended to every new
	// draft. Kept terse because the editor mounts it on-screen — a wall of
	// comments up top discourages the user more than it helps.
	function materializerHeader(language: ScriptLang, sources: DraftTriggerSource[]): string {
		const p = commentPrefix(language)
		const onLines = sources.map((s) => {
			switch (s.kind) {
				case 'schedule':
					return `${p} on schedule "${s.cron}"`
				case 'asset':
					return `${p} on ${s.ref}`
				default:
					// Empty placeholder path makes it visible the user needs
					// to fill in the trigger reference.
					return `${p} on ${s.kind} ${s.path ?? '<trigger-path>'}`
			}
		})
		const lines = [
			`${p} materialize`,
			...onLines,
			`${p}`,
			`${p} This script is a pipeline materializer.`,
			`${p}   - Reads and writes detected in the code become the lineage edges`,
			`${p}     shown in the pipeline graph (no extra declaration needed).`,
			`${p}   - The \`${p} materialize\` marker opts it into the pipeline.`,
			`${p}   - \`${p} on <asset|schedule "cron"|<kind> <path>>\` declares trigger edges.`,
			`${p}     Supported trigger kinds: schedule, webhook, email, kafka, mqtt,`,
			`${p}     nats, postgres, sqs, gcp.`,
			`${p}`,
			`${p} Put your logic inside \`main\`. Whatever you return is the script's`,
			`${p} output; writes to assets (e.g. s3://, datatable://, ducklake://,`,
			`${p} volume://) go through the usual Windmill helpers.`,
			''
		]
		return lines.join('\n')
	}

	// Per-language boilerplate that declares the output asset URI at module
	// scope. Two reasons:
	//   1. The string literal is where the backend parser picks up the write
	//      on deploy — reproducing the same asset → runnable edge the
	//      frontend draft overlay shows while unsaved.
	//   2. Gives the user a named variable to reference when filling in
	//      `main`, instead of a magic string deep in the body.
	function outputAssetSnippet(language: ScriptLang, uri: string): string {
		const p = commentPrefix(language)
		const note = `${p} Output of this materializer — write to this path inside \`main\`.`
		switch (language) {
			case 'python3':
				return `${note}\nOUTPUT = "${uri}"\n\n`
			case 'postgresql':
			case 'mysql':
			case 'bigquery':
			case 'snowflake':
			case 'mssql':
			case 'oracledb':
			case 'duckdb':
				// SQL has no variable scope we can reliably reuse; drop a
				// referenceable comment the parser still finds inside strings.
				return `${note}\n${p}   ${uri}\n\n`
			case 'bash':
			case 'powershell':
			case 'nu':
			case 'ansible':
				return `${note}\nOUTPUT="${uri}"\n\n`
			default:
				return `${note}\nconst OUTPUT = "${uri}"\n\n`
		}
	}

	function buildDraft(
		language: ScriptLang,
		scriptPath: string,
		sources: DraftTriggerSource[],
		outputAssetUri?: string
	): Script {
		// Reuse the language-specific main() boilerplate Windmill already
		// ships with so every language produces a usable function signature.
		const body = initialCode(language as any, 'script', 'script')
		const header = materializerHeader(language, sources)
		const output = outputAssetUri ? outputAssetSnippet(language, outputAssetUri) : ''
		const content = header + output + body
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
		sources: DraftTriggerSource[]
	) {
		const out = randomOutputAssetPath()
		const outputUri = `${ASSET_PREFIX[out.kind]}${out.path}`
		const script = buildDraft(language, scriptPath, sources, outputUri)
		// Write the new draft into the map (structural update so Svelte
		// re-derives graphWithDraft) and focus it in the details pane.
		const next = new Map(drafts)
		next.set(scriptPath, { script, outputAsset: out })
		drafts = next
		activeDraftPath = scriptPath
		selection = undefined
	}

	function discardDraft(path: string) {
		if (!drafts.has(path)) return
		const next = new Map(drafts)
		next.delete(path)
		drafts = next
		if (activeDraftPath === path) activeDraftPath = undefined
	}

	// Currently-open draft shape (if any) — fed into the details pane.
	let activeDraft = $derived(activeDraftPath ? drafts.get(activeDraftPath) : undefined)

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
			runnables.push({ path, usage_kind: 'script', is_materializer: true })
			const out = d.outputAsset
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
			// Seed trigger edges (schedule + asset) from the draft's template
			// so the graph stays stable when the user clicks off this draft.
			// Live annotations (below) take over for the currently-open draft
			// so keystroke edits still update in real time.
			const parsed = parsePipelineAnnotations(d.script.content)
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
					<Pane size={selection ? 60 : 100}>
						<AssetGraphCanvas
							graph={graphWithDraft}
							selection={effectiveSelection}
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
							onAddScriptForAsset={(asset, language, scriptPath) => {
								const ref = `${ASSET_PREFIX[asset.kind]}${asset.path}`
								openMaterializerDraft(language, scriptPath, [{ kind: 'asset', ref }])
							}}
							onAddMaterializer={(language, scriptPath, source) =>
								openMaterializerDraft(language, scriptPath, [source])}
						/>
					</Pane>
					{#if (selection || activeDraft) && $workspaceStore}
						<Pane size={40} minSize={25}>
							<AssetGraphDetailsPane
								selection={activeDraft ? undefined : selection}
								draftScript={activeDraft?.script}
								workspace={$workspaceStore}
								onAnnotationsChange={(scriptPath, annotations) => {
									liveAnnotations = { scriptPath, annotations }
								}}
								onclose={() => {
									// Close dismisses the pane but preserves drafts so
									// the user can come back to them. Discarding is
									// via the explicit "Discard" button in the pane.
									selection = undefined
									activeDraftPath = undefined
									liveAnnotations = {
										scriptPath: undefined,
										annotations: {
											isMaterializer: false,
											triggerAssets: [],
											schedules: [],
											nativeTriggers: []
										}
									}
								}}
								onDiscard={() => {
									if (activeDraftPath) discardDraft(activeDraftPath)
								}}
								onDraftSaved={async (savedPath) => {
									discardDraft(savedPath)
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
{/if}
