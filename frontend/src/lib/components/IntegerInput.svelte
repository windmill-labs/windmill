<script lang="ts">
	import TextInput from './text_input/TextInput.svelte'
	import type { ButtonType } from './common/button/model'
	import { untrack } from 'svelte'

	interface Props {
		value?: number
		oninput?: (value: number | undefined) => void
		placeholder?: string
		id?: string
		disabled?: boolean
		error?: string
		class?: string
		size?: ButtonType.UnifiedSize
	}

	let {
		value,
		oninput,
		placeholder,
		id,
		disabled,
		error = '',
		class: className = '',
		size
	}: Props = $props()

	let displayValue: string | number = $state('')

	$effect(() => {
		const incoming = value
		const current = String(untrack(() => displayValue))
		const currentNum = current === '' ? undefined : Number(current)
		if (incoming !== currentNum) {
			displayValue = incoming != null ? String(incoming) : ''
		}
	})

	function handleKeydown(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey) return
		if (
			!/[0-9]/.test(e.key) &&
			![
				'Backspace',
				'Delete',
				'ArrowLeft',
				'ArrowRight',
				'ArrowUp',
				'ArrowDown',
				'Tab',
				'Enter'
			].includes(e.key)
		) {
			e.preventDefault()
		}
	}

	function handleInput(e: Event) {
		if (e.target instanceof HTMLInputElement) {
			const raw = e.target.value.replace(/[^0-9]/g, '')
			e.target.value = raw
			displayValue = raw
			if (raw === '') {
				oninput?.(undefined)
				return
			}
			oninput?.(Number(raw))
		}
	}
</script>

<TextInput
	{size}
	class={className}
	{error}
	value={displayValue}
	inputProps={{
		type: 'number',
		inputmode: 'numeric',
		placeholder,
		id,
		disabled,
		onkeydown: handleKeydown,
		oninput: handleInput
	}}
/>
