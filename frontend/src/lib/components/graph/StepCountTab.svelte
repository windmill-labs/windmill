<script lang="ts">
	import { NOTE_COLORS, NoteColor, type NoteColorConfig } from './noteColors'
	import { stopPropagation, preventDefault } from 'svelte/legacy'

	interface Props {
		stepCount: number
		color?: string
		onExpand?: () => void
	}

	let { stepCount, color, onExpand }: Props = $props()

	let colorConfig: NoteColorConfig = $derived(
		color
			? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE])
			: NOTE_COLORS[NoteColor.BLUE]
	)
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="absolute left-0 rounded-t-md px-2 py-0.5 text-3xs font-medium z-[-1] {colorConfig.background} {colorConfig.text} {onExpand ? 'cursor-pointer hover:opacity-80' : ''}"
	style="top: -16px; height: 24px; line-height: 13px;"
	onclick={onExpand ? stopPropagation(preventDefault(onExpand)) : undefined}
	onpointerdown={onExpand ? stopPropagation(preventDefault(() => {})) : undefined}
>
	{stepCount} step{stepCount !== 1 ? 's' : ''}
</div>
