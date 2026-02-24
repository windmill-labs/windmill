<script lang="ts">
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import { Group } from 'lucide-svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		summary?: string
		selected?: boolean
		stepCount?: number
		fullWidth?: boolean
		color?: string
	}

	let { summary, selected = false, stepCount, fullWidth = false, color }: Props = $props()

	let noteColorConfig = $derived(
		color ? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE]) : undefined
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
			<Group size={14} />
			<span class="text-2xs font-medium truncate">{summary || 'Group'}</span>
			{#if stepCount != null}
				<span class="text-2xs opacity-60">{stepCount} node{stepCount !== 1 ? 's' : ''}</span>
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
					<Group size={14} />
				{/snippet}
			</FlowModuleSchemaItemViewer>
		</div>
	{/if}
</div>
