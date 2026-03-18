<script lang="ts">
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import { ChevronRight } from 'lucide-svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		summary?: string
		color?: string
		collapsed: boolean
		editMode: boolean
		onToggleCollapse: () => void
		onSummaryUpdate?: (text: string) => void
	}

	let { summary, color, collapsed, editMode, onToggleCollapse, onSummaryUpdate }: Props = $props()

	let colorConfig = $derived(
		NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE]
	)

	// Inline summary editing
	let editingSummary = $state(false)
	let summaryInput = $state('')
	let textInputComponent: TextInput | undefined = $state(undefined)

	function startEditingSummary() {
		if (!editMode) return
		editingSummary = true
		summaryInput = summary ?? ''
		requestAnimationFrame(() => {
			textInputComponent?.focus()
			textInputComponent?.select()
		})
	}

	function saveSummary() {
		editingSummary = false
		const trimmed = summaryInput.trim()
		if (trimmed !== (summary ?? '')) {
			onSummaryUpdate?.(trimmed)
		}
	}

	function handleSummaryKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			saveSummary()
		} else if (event.key === 'Escape') {
			editingSummary = false
		}
	}

	// Animate chevron rotation on mount
	let chevronRotated = $state(collapsed)
	$effect(() => {
		requestAnimationFrame(() => {
			chevronRotated = !collapsed
		})
	})
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class="flex items-center h-[22px] w-full px-2 relative cursor-pointer {colorConfig.background} {colorConfig.text} {collapsed
		? 'rounded-t-md'
		: 'rounded-md'}"
	onclick={stopPropagation(preventDefault(onToggleCollapse))}
	onpointerdown={stopPropagation(preventDefault(() => {}))}
	title={collapsed ? 'Expand group' : 'Collapse group'}
>
	<div
		class="flex items-center justify-center shrink-0 opacity-60 transition-transform duration-100"
		class:rotate-90={chevronRotated}
	>
		<ChevronRight size={12} />
	</div>
	<div class="absolute inset-x-0 flex items-center justify-center h-full pointer-events-none px-7">
		{#if editingSummary}
			<TextInput
				bind:this={textInputComponent}
				bind:value={summaryInput}
				size="xs"
				class="!bg-transparent !border-transparent !shadow-none !text-2xs !font-medium !p-0 !m-0 !min-w-0 !w-fit text-center !min-h-0 !h-auto nodrag nowheel pointer-events-auto"
				inputProps={{
					placeholder: 'Group',
					onblur: saveSummary,
					onkeydown: handleSummaryKeydown,
					spellcheck: false,
					style: 'padding: 2px !important; field-sizing: content; min-width: 5ch !important'
				}}
			/>
		{:else}
			<span
				class="text-2xs font-medium truncate text-center pointer-events-auto {editMode
					? 'cursor-text rounded px-0.5 -mx-0.5 hover:bg-black/10 dark:hover:bg-white/10'
					: ''}"
				onclick={editMode ? stopPropagation(preventDefault(startEditingSummary)) : undefined}
				onpointerdown={editMode ? stopPropagation(preventDefault(() => {})) : undefined}
				>{summary || 'Group'}</span
			>
		{/if}
	</div>
</div>
