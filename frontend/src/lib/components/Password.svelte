<script lang="ts">
	import { createBubbler } from 'svelte/legacy'
	import { tick } from 'svelte'
	import Button from './common/button/Button.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { Eye, EyeClosed } from 'lucide-svelte'
	import type { HTMLInputAttributes } from 'svelte/elements'

	const bubble = createBubbler()
	interface Props {
		password: string | undefined
		placeholder?: string
		disabled?: boolean
		required?: boolean
		small?: boolean
		minRows?: number
		id?: string
		autocomplete?: HTMLInputAttributes['autocomplete']
		/** Off for login-style fields: keeps Enter free to submit. Overrides `minRows`. */
		allowMultiline?: boolean
		/** Renders the field in its error state; the message itself is the caller's to display. */
		error?: boolean
		/** id of the element holding that message, wired up as aria-describedby. */
		describedBy?: string
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
		autocomplete = 'new-password',
		allowMultiline = true,
		error = false,
		describedBy = undefined,
		onKeyDown,
		onBlur
	}: Props = $props()

	let red = $derived(required && (password == '' || password == undefined))
	let hasError = $derived(red || error)
	let hideValue = $state(true)
	let forceMultiline = $state(false)
	let isMultiline = $derived(
		allowMultiline &&
			(forceMultiline || (minRows != null && minRows > 1) || (password?.includes('\n') ?? false))
	)

	let textareaRef: TextInput<'textarea'> | undefined = $state()
	let inputRef: TextInput<'input'> | undefined = $state()

	export function focus() {
		;(isMultiline ? textareaRef : inputRef)?.focus()
	}

	// Revealing swaps the input to type="text". Auth forms conceal again before submitting,
	// so the browser sees a password field when it decides whether to save the credential.
	export function conceal() {
		hideValue = true
	}

	function insertAndSwitchToMultiline(input: HTMLInputElement, text: string) {
		const start = input.selectionStart
		const end = input.selectionEnd
		if (start != null && end != null) {
			password = (password ?? '').substring(0, start) + text + (password ?? '').substring(end)
		} else {
			// selectionStart/End are null for type="password" inputs
			password = (password ?? '') + text
		}
		forceMultiline = true
		tick().then(() => textareaRef?.focus())
	}
</script>

<div class="relative w-full {small ? 'max-w-lg' : ''}">
	{#if isMultiline}
		<TextInput
			bind:this={textareaRef}
			size="md"
			error={hasError}
			bind:value={password}
			underlyingInputEl="textarea"
			inputProps={{
				id,
				disabled,
				placeholder,
				rows: minRows ?? 3,
				autocomplete,
				'aria-invalid': hasError ? 'true' : undefined,
				'aria-describedby': describedBy,
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
			bind:this={inputRef}
			size="md"
			error={hasError}
			bind:value={password}
			inputProps={{
				id,
				disabled,
				placeholder,
				autocomplete,
				'aria-invalid': hasError ? 'true' : undefined,
				'aria-describedby': describedBy,
				onblur: (e) => onBlur?.(e),
				onkeydown: (e) => {
					if (allowMultiline && e.key === 'Enter') {
						e.preventDefault()
						insertAndSwitchToMultiline(e.currentTarget as HTMLInputElement, '\n')
						return
					}
					onKeyDown?.(e)
					bubble('keydown')(e)
				},
				onpaste: (e) => {
					const text = e.clipboardData?.getData('text')
					if (allowMultiline && text?.includes('\n')) {
						e.preventDefault()
						insertAndSwitchToMultiline(e.currentTarget as HTMLInputElement, text)
					}
				},
				type: hideValue ? 'password' : 'text'
			}}
			class="pr-8"
		/>
	{/if}
	<!-- After the input in DOM order so Tab reaches the field before the toggle -->
	<div class="absolute {isMultiline ? 'top-1' : 'inset-y-0'} right-1 flex items-center z-10">
		<Button
			unifiedSize="sm"
			onClick={() => (hideValue = !hideValue)}
			iconOnly
			startIcon={{ icon: hideValue ? Eye : EyeClosed }}
			variant="subtle"
			title={hideValue ? 'Show password' : 'Hide password'}
			wrapperClasses="bg-surface-input"
		/>
	</div>
</div>
{#if red}
	<div class="text-red-600 text-2xs grow">This field is required</div>
{/if}
