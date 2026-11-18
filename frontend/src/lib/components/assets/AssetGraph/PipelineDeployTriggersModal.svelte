<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { PipelineTriggerDraft } from './types'

	// Triggers start running once deployed, so deploying a pipeline's trigger
	// drafts asks first, listing what will start.
	let {
		open,
		triggerDrafts,
		onConfirmed,
		onCanceled
	}: {
		open: boolean
		triggerDrafts: ReadonlyMap<string, PipelineTriggerDraft>
		onConfirmed: () => void
		onCanceled: () => void
	} = $props()
</script>

<ConfirmationModal
	{open}
	title="Deploy triggers?"
	confirmationText="Deploy"
	type="info"
	{onConfirmed}
	{onCanceled}
>
	<div class="flex flex-col gap-2 text-xs text-secondary">
		<p>
			{triggerDrafts.size === 1
				? 'Saving the pipeline also creates this trigger. It starts running its script once deployed.'
				: `Saving the pipeline also creates these ${triggerDrafts.size} triggers. They start running their scripts once deployed.`}
		</p>
		<ul class="flex flex-col gap-1">
			{#each [...triggerDrafts] as [key, d] (key)}
				<li class="flex flex-col">
					<span class="font-mono text-2xs text-emphasis truncate">{d.config.path}</span>
					<span class="text-2xs text-hint truncate">
						{d.kind}{#if d.kind === 'schedule'}
							· <span class="font-mono">{d.config.schedule}</span> ({d.config.timezone}){/if}
						→ {d.config.script_path}
					</span>
				</li>
			{/each}
		</ul>
	</div>
</ConfirmationModal>
