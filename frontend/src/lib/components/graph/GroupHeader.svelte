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

	const PLACEHOLDER = 'Group'

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
		class:rotate-90={!collapsed}
	>
		<ChevronRight size={12} />
	</div>
	<div class="absolute inset-x-0 flex items-center justify-center h-full pointer-events-none px-7">
		{#if editingSummary}
			<div
				class="input-sizer inline-grid items-center pointer-events-auto text-2xs font-medium max-w-full"
				data-value={summaryInput || PLACEHOLDER}
			>
				<TextInput
					bind:this={textInputComponent}
					bind:value={summaryInput}
					size="xs"
					class="!bg-transparent !border-transparent !shadow-none !text-2xs !font-medium !p-0 !m-0 !min-w-0 text-center !min-h-0 !h-auto nodrag nowheel"
					inputProps={{
						placeholder: PLACEHOLDER,
						onblur: saveSummary,
						onkeydown: handleSummaryKeydown,
						spellcheck: false,
						size: 1,
						style: 'padding: 2px !important; grid-area: 1 / 1'
					}}
				/>
			</div>
		{:else}
			<span
				class="text-2xs font-medium truncate text-center pointer-events-auto {editMode
					? 'cursor-text rounded px-0.5 -mx-0.5 hover:bg-black/10 dark:hover:bg-white/10'
					: ''}"
				onclick={editMode ? stopPropagation(preventDefault(startEditingSummary)) : undefined}
				onpointerdown={editMode ? stopPropagation(preventDefault(() => {})) : undefined}
				>{summary || PLACEHOLDER}</span
			>
		{/if}
	</div>
</div>

<style>
	.input-sizer::after {
		content: attr(data-value) ' ';
		visibility: hidden;
		white-space: pre;
		grid-area: 1 / 1;
		font: inherit;
		padding: 2px;
		text-align: center;
	}
</style>
