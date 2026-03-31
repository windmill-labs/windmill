<script lang="ts">
	import { createBubbler } from 'svelte/legacy'
	import Button from './common/button/Button.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { Eye, EyeClosed } from 'lucide-svelte'

	const bubble = createBubbler()
	interface Props {
		password: string | undefined
		placeholder?: string
		disabled?: boolean
		required?: boolean
		small?: boolean
		minRows?: number
		id?: string
		onKeyDown?: (event: KeyboardEvent) => void
		onBlur?: (event: FocusEvent) => void
	}

	let {
		password = $bindable(),
		placeholder = '******',
		disabled = false,
		required = false,
		small = false,
		minRows,
		id,
		onKeyDown,
		onBlur
	}: Props = $props()

	let red = $derived(required && (password == '' || password == undefined))

	let hideValue = $state(true)
	let forceMultiline = $state(false)
	let isMultiline = $derived(
		forceMultiline || (minRows != null && minRows > 1) || (password?.includes('\n') ?? false)
	)

	function onPasteIntoInput(e: ClipboardEvent) {
		const text = e.clipboardData?.getData('text')
		if (text?.includes('\n')) {
			e.preventDefault()
			const input = e.currentTarget as HTMLInputElement
			const start = input.selectionStart ?? 0
			const end = input.selectionEnd ?? 0
			password = (password ?? '').substring(0, start) + text + (password ?? '').substring(end)
			forceMultiline = true
		}
	}
</script>

<div class="relative w-full {small ? 'max-w-lg' : ''}">
	<div class="absolute {isMultiline ? 'top-1' : 'inset-y-0'} right-1 flex items-center z-10">
		<Button
			unifiedSize="sm"
			onClick={() => (hideValue = !hideValue)}
			iconOnly
			startIcon={{ icon: hideValue ? Eye : EyeClosed }}
			variant="subtle"
			wrapperClasses="bg-surface-input"
		/>
	</div>
	{#if isMultiline}
		<TextInput
			size="md"
			error={red}
			bind:value={password}
			underlyingInputEl="textarea"
			inputProps={{
				id,
				disabled,
				placeholder,
				rows: minRows ?? 3,
				autocomplete: 'new-password',
				onblur: (e) => onBlur?.(e),
				onkeydown: (e) => {
					onKeyDown?.(e)
					bubble('keydown')(e)
				},
				style: hideValue ? '-webkit-text-security: disc' : ''
			}}
			class="pr-8"
			unifiedHeight={false}
		/>
	{:else}
		<TextInput
			size="md"
			error={red}
			bind:value={password}
			inputProps={{
				id,
				disabled,
				placeholder,
				autocomplete: 'new-password',
				onblur: (e) => onBlur?.(e),
				onkeydown: (e) => {
					if (e.key === 'Enter') {
						e.preventDefault()
						const input = e.currentTarget as HTMLInputElement
						const start = input.selectionStart ?? 0
						const end = input.selectionEnd ?? 0
						password = (password ?? '').substring(0, start) + '\n' + (password ?? '').substring(end)
						forceMultiline = true
						return
					}
					onKeyDown?.(e)
					bubble('keydown')(e)
				},
				onpaste: onPasteIntoInput,
				type: hideValue ? 'password' : 'text'
			}}
			class="pr-8"
		/>
	{/if}
</div>
{#if red}
	<div class="text-red-600 text-2xs grow">This field is required</div>
{/if}
