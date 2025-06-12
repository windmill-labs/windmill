<script lang="ts">
	import { Eye, Pen } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		loading?: boolean
	}

	let { loading = false }: Props = $props()

	const { mode, jobs, jobsById } = getContext<AppViewerContext>('AppViewerContext')
</script>

<ToggleButtonGroup
	class="h-[30px]"
	bind:selected={$mode}
	on:selected={(e) => {
		jobsById.set({})
		jobs.set([])
	}}
>
	{#snippet children({ item })}
		<ToggleButton
			label="Editor"
			value="dnd"
			icon={Pen}
			disabled={loading}
			iconProps={{ size: 16 }}
			class="gap-2"
			{item}
		/>
		<ToggleButton
			label="Preview"
			value="preview"
			icon={Eye}
			tooltip="Preview mode"
			disabled={loading}
			id="app-editor-preview-toggle"
			iconProps={{ size: 16 }}
			class="gap-2"
			{item}
		/>
	{/snippet}
</ToggleButtonGroup>
