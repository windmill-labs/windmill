<script lang="ts">
	import { NOTE_COLORS, NoteColor, type NoteColorConfig } from './noteColors'
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import { Maximize2, Minimize2 } from 'lucide-svelte'

	interface Props {
		stepCount?: number
		label?: string
		summary?: string
		color?: string
		collapsed?: boolean
		short?: boolean
		onExpand?: () => void
	}

	let { stepCount, label, summary, color, collapsed = true, short = false, onExpand }: Props = $props()

	let colorConfig: NoteColorConfig = $derived(
		color
			? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE])
			: NOTE_COLORS[NoteColor.BLUE]
	)
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="absolute left-0 rounded-t-md px-2 text-3xs font-medium z-[-1] flex items-start gap-0.5 {colorConfig.background} {colorConfig.text} {onExpand
		? 'cursor-pointer hover:opacity-80'
		: ''}"
	style="top: -20px; height: {short ? 20 : 34}px;"
	onclick={onExpand ? stopPropagation(preventDefault(onExpand)) : undefined}
	onpointerdown={onExpand ? stopPropagation(preventDefault(() => {})) : undefined}
>
	<div class="flex items-center gap-1 py-0.5 whitespace-nowrap">
		{#if label}{label}{:else}{stepCount} step{stepCount !== 1 ? 's' : ''}{/if}{#if summary && collapsed}<span class="opacity-60 mx-0.5">·</span><span class="truncate max-w-[120px]">{summary}</span>{/if}
		{#if onExpand}
			{#if collapsed}<Maximize2 size={10} class="-my-1" />{:else}<Minimize2
					size={10}
					class="-my-1"
				/>{/if}
		{/if}
	</div>
</div>
