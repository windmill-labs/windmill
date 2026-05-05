<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		value: string
		placeholder?: string
		onSave?: (newValue: string) => void
		editable?: boolean
		size?: 'xs' | 'sm' | 'md' | 'lg'
		class?: string
		inputClass?: string
		/** Text styling (font-size, weight, color, line-height...) applied to both the
		 * idle button and the editing input so the two render identically. */
		textClass?: string
	}

	let {
		value,
		placeholder = '',
		onSave,
		editable = true,
		size = 'sm',
		class: className = '',
		inputClass = '',
		textClass = ''
	}: Props = $props()

	let editing = $state(false)
	let draft = $state('')
	let textInputComponent: TextInput | undefined = $state(undefined)

	function startEditing() {
		if (!editable) return
		editing = true
		draft = value ?? ''
		requestAnimationFrame(() => {
			textInputComponent?.focus()
			textInputComponent?.select()
		})
	}

	function save() {
		editing = false
		const trimmed = draft.trim()
		if (trimmed !== (value ?? '')) {
			onSave?.(trimmed)
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') save()
		else if (e.key === 'Escape') editing = false
	}
</script>

{#if editing}
	<span
		class="input-sizer inline-grid items-center {textClass} {className}"
		data-value={draft || placeholder}
	>
		<TextInput
			bind:this={textInputComponent}
			bind:value={draft}
			{size}
			class="!bg-transparent !border-0 !shadow-none !p-0 !m-0 !min-w-0 !min-h-0 !h-auto {textClass} {inputClass}"
			inputProps={{
				placeholder,
				onblur: save,
				onkeydown: handleKeydown,
				spellcheck: false,
				size: 1,
				style: 'padding: 2px !important; grid-area: 1 / 1'
			}}
		/>
	</span>
{:else}
	<button
		type="button"
		onclick={startEditing}
		disabled={!editable}
		class="text-left truncate rounded p-0.5 {editable
			? 'cursor-text hover:bg-surface-hover'
			: 'cursor-default'} {textClass} {className}"
	>
		{value || placeholder}
	</button>
{/if}

<style>
	.input-sizer::after {
		content: attr(data-value) ' ';
		visibility: hidden;
		white-space: pre;
		grid-area: 1 / 1;
		font: inherit;
		padding: 2px;
	}
</style>
