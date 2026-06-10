<script lang="ts">
	import { Eye, Pen } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import type { PipelineMode } from './types'

	interface Props {
		mode: PipelineMode
		/** Unsaved draft count — shown as a badge on the Edit button. */
		draftCount: number
		onModeChange: (mode: PipelineMode) => void
	}

	let { mode, draftCount, onModeChange }: Props = $props()
</script>

<ToggleButtonGroup
	selected={mode}
	noWFull
	onSelected={(v) => {
		if (v !== mode) onModeChange(v as PipelineMode)
	}}
>
	{#snippet children({ item })}
		<ToggleButton
			label="View"
			value="view"
			icon={Eye}
			tooltip="Deployed pipeline and its executions"
			iconProps={{ size: 16 }}
			class="gap-2"
			{item}
		/>
		<ToggleButton
			label={draftCount > 0 ? `Edit (${draftCount})` : 'Edit'}
			value="edit"
			icon={Pen}
			tooltip={draftCount > 0
				? `Edit pipeline — ${draftCount} unsaved draft${draftCount === 1 ? '' : 's'}`
				: 'Edit pipeline'}
			iconProps={{ size: 16 }}
			class="gap-2"
			{item}
		/>
	{/snippet}
</ToggleButtonGroup>
