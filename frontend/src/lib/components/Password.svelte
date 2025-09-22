<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	interface Props {
		password: string | undefined
		placeholder?: string
		disabled?: boolean
		required?: boolean
		small?: boolean
		onKeyDown?: (event: KeyboardEvent) => void
	}

	let {
		password = $bindable(),
		placeholder = '******',
		disabled = false,
		required = false,
		small = false,
		onKeyDown
	}: Props = $props()

	let red = $derived(required && (password == '' || password == undefined))

	let hideValue = $state(true)

	let randomId = (Math.random() * 10e15).toString(16)
</script>

<div class="relative w-full {small ? 'max-w-lg' : ''}">
	<div class="absolute inset-y-0 right-0 flex items-center px-2">
		<input bind:checked={hideValue} class="!hidden" id={randomId} type="checkbox" />
		<label
			class="bg-surface-secondary hover:bg-gray-400 rounded px-2 py-1 text-sm text-tertiary font-mono cursor-pointer"
			for={randomId}>{hideValue ? 'show' : 'hide'}</label
		>
	</div>
	{#if hideValue}
		<input
			class="block {small ? '!text-2xs' : 'w-full'} px-2 py-1 {red
				? '!border-red-500'
				: ''} text-sm h-9"
			type="password"
			bind:value={password}
			onkeydown={onKeyDown}
			autocomplete="new-password"
			{placeholder}
			{disabled}
		/>
	{:else}
		<input
			class="block {small ? '!text-2xs' : 'w-full'} px-2 py-1 {red
				? '!border-red-500'
				: ''} text-sm h-9"
			type="text"
			bind:value={password}
			onkeydown={bubble('keydown')}
			autocomplete="new-password"
			{placeholder}
			{disabled}
		/>
	{/if}
</div>
{#if red}
	<div class="text-red-600 text-2xs grow">This field is required</div>
{/if}
