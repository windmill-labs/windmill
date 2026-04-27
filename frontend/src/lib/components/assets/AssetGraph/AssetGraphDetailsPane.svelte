<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'
	import { resource } from 'runed'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatAssetKind } from '$lib/components/assets/lib'
	import { Code2, ExternalLink, GitBranch, Loader2, Save, Trash2, X } from 'lucide-svelte'
	import { inferArgs } from '$lib/infer'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type { AssetGraphSelection } from './types'
	import { parsePipelineAnnotations, type PipelineAnnotations } from './parsePipelineAnnotations'

	interface Props {
		// Regular selection — loads the script by path for inline editing.
		selection?: AssetGraphSelection | undefined
		// Pre-built draft script not yet persisted. Takes precedence over
		// `selection` when present. Used by the pipeline + menu so a new
		// materializer opens inline instead of navigating to /scripts/add.
		draftScript?: Script | undefined
		workspace: string
		onclose: () => void
		// Called after a draft is saved for the first time so the page can
		// refetch the graph and clear its local draft.
		onDraftSaved?: (savedPath: string) => void
		// Called when the user hits "Discard" on the draft header. Separate
		// from `onclose` so the page can drop the draft from its map vs
		// just dismissing the pane.
		onDiscard?: () => void
		// Emits live-parsed pipeline annotations from the open script so the
		// canvas can overlay unsaved schedule / trigger nodes in real time.
		// Fires whenever the script content changes.
		onAnnotationsChange?: (scriptPath: string | undefined, annotations: PipelineAnnotations) => void
	}
	let {
		selection,
		draftScript,
		workspace,
		onclose,
		onDraftSaved,
		onDiscard,
		onAnnotationsChange
	}: Props = $props()

	// When `draftScript` is provided we bypass the fetch entirely and edit
	// it locally; saving calls ScriptService.createScript to deploy it.
	let scriptRes = resource(
		[() => workspace, () => selection, () => draftScript],
		async ([ws, sel, draft], _prev, { signal }) => {
			if (draft) return undefined
			if (!sel || sel.kind !== 'runnable' || sel.runnable_kind !== 'script') return undefined
			return await ScriptService.getScriptByPath({ workspace: ws, path: sel.path }, signal as any)
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
		script = fresh ? structuredClone($state.snapshot(fresh) as Script) : undefined
	})

	let args = $state<Record<string, any>>({})
	let saving = $state(false)
	let isDraft = $derived(draftScript != undefined)

	// Live-parse the editor buffer so the page can render unsaved schedule /
	// trigger nodes on the canvas. Parsed TS output mirrors the Rust
	// `parse_pipeline_annotations` used at deploy time.
	let liveAnnotations = $derived<PipelineAnnotations>(
		script
			? parsePipelineAnnotations(script.content ?? '')
			: {
					isMaterializer: false,
					triggerAssets: [],
					schedules: [],
					nativeTriggers: []
				}
	)
	$effect(() => {
		onAnnotationsChange?.(script?.path, liveAnnotations)
	})

	async function save() {
		if (!script) return
		saving = true
		try {
			script.schema = script.schema ?? emptySchema()
			try {
				const result = await inferArgs(script.language, script.content, script.schema)
				;(script as any).auto_kind = result?.auto_kind || undefined
				script.has_preprocessor = result?.has_preprocessor || undefined
			} catch {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}
			await ScriptService.createScript({
				workspace,
				requestBody: {
					...script,
					language: script.language,
					description: script.description ?? '',
					// Drafts have no prior hash; workspace scripts chain off their last hash.
					parent_hash: isDraft || script.hash == undefined ? undefined : String(script.hash),
					is_template: false,
					tag: script.tag,
					kind: script.kind as Script['kind'] | undefined,
					lock: undefined
				}
			})
			sendUserToast(`Saved ${script.path}`)
			if (isDraft) {
				onDraftSaved?.(script.path)
			} else {
				await scriptRes.refetch()
			}
		} catch (e: any) {
			sendUserToast(`Save failed: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			saving = false
		}
	}

	let isScriptView = $derived(
		isDraft || (selection?.kind === 'runnable' && selection.runnable_kind === 'script')
	)
</script>

<div class="flex flex-col h-full bg-surface">
	<div
		class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10 whitespace-nowrap"
	>
		<div class="flex items-center gap-2 min-w-0">
			{#if isDraft && script}
				<Code2 size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-emerald-600 dark:text-emerald-400">
						Draft materializer
					</span>
					<span class="text-xs font-mono truncate" title={script.path}>{script.path}</span>
				</div>
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
				<Code2 size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Script</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow'}
				<GitBranch size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Flow</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			{#if isDraft && onDiscard}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: Trash2 }}
					onclick={onDiscard}
					iconOnly
					title="Discard draft"
				/>
			{/if}
			{#if isScriptView && script}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					onclick={save}
					disabled={saving}
				>
					{saving ? 'Saving…' : isDraft ? 'Create' : 'Save'}
				</Button>
			{/if}
			{#if !isDraft && selection?.kind === 'runnable'}
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
			<div class="p-3 text-xs text-secondary">
				Asset details. Use the producer/consumer arrows in the graph to navigate.
			</div>
		{:else if selection?.kind === 'runnable' && selection.runnable_kind === 'flow' && !isDraft}
			<div class="p-3 text-xs text-secondary">
				Flows are not editable inline. Use the open-in-editor button above.
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
		{:else if script}
			{#key (script.hash ?? 'draft') + script.language}
				<ScriptEditor
					showCaptures={false}
					noSyncFromGithub
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
							disableJsonView: true
						}
					}}
					bind:code={script.content}
					bind:schema={script.schema}
					{args}
				>
					{#snippet editorBarRight()}
						<div>
							<WorkerTagSelect bind:tag={() => script?.tag, (v) => script && (script.tag = v)} />
						</div>
					{/snippet}
				</ScriptEditor>
			{/key}
		{/if}
	</div>
</div>
