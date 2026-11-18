<script lang="ts">
	import { Eye, Pen } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		loading?: boolean
		/** Drops the two labels, for a bar with no room for them: the pen and the eye carry it. */
		iconOnly?: boolean
	}

	let { loading = false, iconOnly = false }: Props = $props()

	const { mode, jobs, jobsById } = getContext<AppViewerContext>('AppViewerContext')
</script>

<ToggleButtonGroup
	bind:selected={$mode}
	on:selected={(e) => {
		jobsById.set({})
		jobs.set([])
	}}
>
	{#snippet children({ item })}
		<ToggleButton
			label={iconOnly ? undefined : 'Editor'}
			tooltip={iconOnly ? 'Editor mode' : undefined}
			value="dnd"
			icon={Pen}
			disabled={loading}
			iconProps={{ size: 16 }}
			class="gap-2"
			{item}
		/>
		<ToggleButton
			label={iconOnly ? undefined : 'Preview'}
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
