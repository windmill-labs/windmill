<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'
	import { resource } from 'runed'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatAssetKind } from '$lib/components/assets/lib'
	import { Code2, ExternalLink, GitBranch, Loader2, Save, X } from 'lucide-svelte'
	import { inferArgs } from '$lib/infer'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import type { AssetGraphSelection } from './types'

	interface Props {
		selection: AssetGraphSelection
		workspace: string
		onclose: () => void
	}
	let { selection, workspace, onclose }: Props = $props()

	// Mirror the workspace-script editing flow from ScriptEditorDrawer: load
	// the script by path, hand a mutable copy to <ScriptEditor>, and on save
	// re-infer args then POST a new version with parent_hash chained.
	let scriptRes = resource(
		[() => workspace, () => selection],
		async ([ws, sel], _prev, { signal }) => {
			if (sel.kind !== 'runnable' || sel.runnable_kind !== 'script') return undefined
			return await ScriptService.getScriptByPath({ workspace: ws, path: sel.path }, signal as any)
		}
	)

	// Local mutable copy that ScriptEditor binds to. Reset whenever the
	// underlying resource yields a new script (selection change or refetch).
	let script = $state<Script | undefined>(undefined)
	$effect.pre(() => {
		const fresh = scriptRes.current
		script = fresh ? structuredClone($state.snapshot(fresh) as Script) : undefined
	})

	let args = $state<Record<string, any>>({})
	let saving = $state(false)

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
					parent_hash: script.hash != undefined ? String(script.hash) : undefined,
					is_template: false,
					tag: script.tag,
					kind: script.kind as Script['kind'] | undefined,
					lock: undefined
				}
			})
			sendUserToast(`Saved ${script.path}`)
			await scriptRes.refetch()
		} catch (e: any) {
			sendUserToast(`Save failed: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="flex flex-col h-full bg-surface">
	<div
		class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10 whitespace-nowrap"
	>
		<div class="flex items-center gap-2 min-w-0">
			{#if selection.kind === 'asset'}
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
			{:else if selection.runnable_kind === 'script'}
				<Code2 size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Script</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{:else}
				<GitBranch size={16} class="shrink-0 text-emerald-700 dark:text-emerald-400" />
				<div class="flex flex-col min-w-0">
					<span class="text-3xs uppercase tracking-wide text-tertiary">Flow</span>
					<span class="text-xs font-mono truncate" title={selection.path}>{selection.path}</span>
				</div>
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			{#if selection.kind === 'runnable' && selection.runnable_kind === 'script' && script}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					onclick={save}
					disabled={saving}
				>
					{saving ? 'Saving…' : 'Save'}
				</Button>
			{/if}
			{#if selection.kind === 'runnable'}
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
		{#if selection.kind === 'asset'}
			<div class="p-3 text-xs text-secondary">
				Asset details. Use the producer/consumer arrows in the graph to navigate.
			</div>
		{:else if selection.runnable_kind === 'flow'}
			<div class="p-3 text-xs text-secondary">
				Flows are not editable inline. Use the open-in-editor button above.
			</div>
		{:else if scriptRes.loading && !script}
			<div class="absolute inset-0 flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={16} class="animate-spin" />
				<span class="text-xs">Loading script…</span>
			</div>
		{:else if scriptRes.error}
			<div class="p-3 text-xs text-red-500">
				Failed to load: {scriptRes.error.message}
			</div>
		{:else if script}
			{#key script.hash}
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
