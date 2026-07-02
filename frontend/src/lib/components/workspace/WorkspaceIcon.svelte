<script lang="ts">
	import { Building, GitFork } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { getContrastTextColor } from '$lib/utils'

	interface Props {
		workspaceColor?: string
		isForked?: boolean
		isDevWorkspace?: boolean
		parentName?: string
		size?: number
		// Tailwind padding class for the round badge — shrink it (e.g. 'p-0.5') for
		// a more compact icon.
		padding?: string
	}

	let {
		workspaceColor,
		isForked = false,
		isDevWorkspace = false,
		parentName,
		size = 14,
		padding = 'p-1.5'
	}: Props = $props()

	const iconColor = $derived(getContrastTextColor(workspaceColor))
</script>

<div style="background-color: {workspaceColor}" class="rounded-full {padding} center-center">
	{#if isForked}
		<Tooltip>
			{#snippet text()}
				{#if isForked && parentName}
					{isDevWorkspace ? 'Dev workspace of' : 'Fork of'}
					{parentName}
				{/if}
			{/snippet}
			<GitFork {size} class="flex-shrink-0" style="color: {iconColor}" />
		</Tooltip>
	{:else}
		<Building {size} style="color: {iconColor}" />
	{/if}
</div>
