<script lang="ts">
	import { Building, GitFork } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { getContrastTextColor } from '$lib/utils'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { devLabelWord } from '$lib/utils/devWorkspaceLabel'

	interface Props {
		workspaceColor?: string
		isForked?: boolean
		isDevWorkspace?: boolean
		devWorkspaceLabel?: string | null
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
		devWorkspaceLabel,
		parentName,
		size = 14,
		padding = 'p-1.5'
	}: Props = $props()

	// A colored FORK tints the fork icon with the derived accent (the fork
	// picker convention) instead of a raw-color disc; roots keep the filled
	// disc with a contrast-picked icon color.
	const accentStyle = $derived(isForked ? forkAccentStyle(workspaceColor) : undefined)
	const iconColor = $derived(isForked ? undefined : getContrastTextColor(workspaceColor))
</script>

<div
	style={accentStyle ??
		(!isForked && workspaceColor ? `background-color: ${workspaceColor}` : undefined)}
	class="rounded-full {padding} center-center"
>
	{#if isForked}
		<Tooltip>
			{#snippet text()}
				{#if isForked && parentName}
					{isDevWorkspace ? `${devLabelWord(devWorkspaceLabel)} workspace of` : 'Fork of'}
					{parentName}
				{/if}
			{/snippet}
			<!-- Explicit neutral fallback: rows are often <a> elements, and an
			     icon left on currentColor would inherit the global link blue. -->
			<GitFork
				{size}
				class="flex-shrink-0 {accentStyle
					? 'text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]'
					: 'text-tertiary'}"
			/>
		</Tooltip>
	{:else}
		<Building
			{size}
			class={iconColor ? '' : 'text-tertiary'}
			style={iconColor ? `color: ${iconColor}` : undefined}
		/>
	{/if}
</div>
