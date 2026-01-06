<script lang="ts">
	import { Building, GitFork } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { getContrastTextColor } from '$lib/utils'

	interface Props {
		workspaceColor?: string
		isForked?: boolean
		parentName?: string
		size?: number
	}

	let { workspaceColor, isForked = false, parentName, size = 14 }: Props = $props()

	const iconColor = $derived(getContrastTextColor(workspaceColor))
</script>

<div style="background-color: {workspaceColor}" class="rounded-full p-1.5 center-center">
	{#if isForked}
		<Tooltip>
			{#snippet text()}
				{#if isForked && parentName}
					Fork of {parentName}
				{/if}
			{/snippet}
			<GitFork {size} class="flex-shrink-0" style="color: {iconColor}" />
		</Tooltip>
	{:else}
		<Building {size} style="color: {iconColor}" />
	{/if}
</div>