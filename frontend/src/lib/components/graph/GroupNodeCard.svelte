<script lang="ts">
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

	interface Props {
		summary?: string
		selected?: boolean
		stepCount?: number
		fullWidth?: boolean
		color?: string
		actions?: Snippet
	}

	let { summary, selected = false, stepCount, fullWidth = false, color, actions }: Props = $props()

	let noteColorConfig = $derived(
		color ? NOTE_COLORS[(color as NoteColor)] ?? NOTE_COLORS[NoteColor.BLUE] : undefined
	)

	let defaultColorClasses = $derived(getNodeColorClasses(undefined, selected))
</script>

<div
	class={twMerge(
		'w-full module flex cursor-pointer max-w-full',
		fullWidth ? 'rounded-t-md' : 'rounded-md drop-shadow-base',
		noteColorConfig ? noteColorConfig.background : defaultColorClasses.bg,
		noteColorConfig ? noteColorConfig.text : ''
	)}
	style={fullWidth ? 'height: 34px;' : 'width: 275px; height: 34px;'}
>
	<div
		class={twMerge(
			'absolute z-0 outline-offset-0 inset-0',
			fullWidth ? 'rounded-t-md' : 'rounded-md',
			noteColorConfig ? noteColorConfig.outline : defaultColorClasses.outline
		)}
	></div>
	{#if fullWidth}
		<div class="flex items-center w-full gap-1.5 px-2 relative z-1">
			<BarsStaggered size={16} />
			<span class="text-2xs font-medium truncate">{summary || 'Group'}</span>
			{#if stepCount != null}
				<span class="text-2xs opacity-60">{stepCount} node{stepCount !== 1 ? 's' : ''}</span>
			{/if}
			{#if actions}
				<div class="ml-auto flex items-center gap-1">
					{@render actions()}
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col w-full">
			<FlowModuleSchemaItemViewer
				label={summary || 'Group'}
				colorClasses={noteColorConfig
					? {
							bg: noteColorConfig.background,
							text: noteColorConfig.text,
							outline: noteColorConfig.outline,
							badge: ''
						}
					: defaultColorClasses}
			>
				{#snippet icon()}
					<BarsStaggered size={16} />
				{/snippet}
			</FlowModuleSchemaItemViewer>
		</div>
	{/if}
</div>
