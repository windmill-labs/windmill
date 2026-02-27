<script lang="ts">
	import Button from './common/button/Button.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { Eye, EyeClosed } from 'lucide-svelte'
	interface Props {
		password: string | undefined
		placeholder?: string
		disabled?: boolean
		required?: boolean
		small?: boolean
		id?: string
		onKeyDown?: (event: KeyboardEvent) => void
		onBlur?: (event: FocusEvent) => void
		onkeydown?: (...args: any[]) => any
	}

	let {
		password = $bindable(),
		placeholder = '******',
		disabled = false,
		required = false,
		small = false,
		id,
		onKeyDown,
		onBlur,
		onkeydown = undefined
	}: Props = $props()

	let red = $derived(required && (password == '' || password == undefined))

	let hideValue = $state(true)
</script>

<div class="relative w-full {small ? 'max-w-lg' : ''}">
	<div class="absolute inset-y-0 right-1 flex items-center">
		<Button
			unifiedSize="sm"
			onClick={() => (hideValue = !hideValue)}
			iconOnly
			startIcon={{ icon: hideValue ? Eye : EyeClosed }}
			variant="subtle"
			wrapperClasses="bg-surface-input"
		/>
	</div>
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
				onKeyDown?.(e)
				onkeydown?.(e)
			},
			type: hideValue ? 'password' : 'text'
		}}
		class="pr-8"
	/>
</div>
{#if red}
	<div class="text-red-600 text-2xs grow">This field is required</div>
{/if}
