<script lang="ts">
	import type { Snippet } from 'svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import PipelineLockedRunnableInfo from '$lib/components/triggers/PipelineLockedRunnableInfo.svelte'

	// Shared runnable section for trigger editors. When `fixedScriptPath` is
	// non-empty the drawer was opened from the pipeline editor for an
	// already-bound script, so the ScriptPicker is swapped for a read-only
	// viewer (see PipelineLockedRunnableInfo) to keep the trigger from being
	// silently reassigned off the pipeline.
	interface Props {
		fixedScriptPath: string
		itemKind: 'script' | 'flow'
		scriptPath: string
		initialScriptPath: string
		canWrite: boolean
		isOperator: boolean
		promptText?: string
		promptClass?: string
		workspace?: string
		// Per-trigger "Create from template" button (hub URL, variant and any
		// extra guard/tooltip differ per trigger kind, so the caller owns it).
		createButton?: Snippet
	}

	let {
		fixedScriptPath,
		itemKind = $bindable(),
		scriptPath = $bindable(),
		initialScriptPath,
		canWrite,
		isOperator,
		promptText = 'Pick a script or flow to be triggered',
		promptClass = 'text-xs mb-1 text-primary',
		workspace = undefined,
		createButton
	}: Props = $props()
</script>

{#if fixedScriptPath != ''}
	<PipelineLockedRunnableInfo path={fixedScriptPath} />
{:else}
	<p class={promptClass}>
		{promptText}<Required required={true} />
	</p>
	<div class="flex flex-row mb-2">
		<ScriptPicker
			disabled={!canWrite}
			initialPath={initialScriptPath}
			kinds={['script']}
			allowFlow={true}
			bind:itemKind
			bind:scriptPath
			allowRefresh={canWrite}
			allowEdit={!isOperator}
			{workspace}
			clearable
		/>
		{@render createButton?.()}
	</div>
{/if}
