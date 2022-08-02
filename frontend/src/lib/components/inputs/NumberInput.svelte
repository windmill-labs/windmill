<script lang="ts">
	import { Input } from 'flowbite-svelte'

	import { createEventDispatcher } from 'svelte'

	export let value: any
	export let valid: boolean = true
	export let disabled: boolean = false
	export let placeholder: string = ''

	const dispatch = createEventDispatcher()
</script>

<Input
	bind:value
	on:input={() => dispatch('input', { value, isRaw: true })}
	{disabled}
	{placeholder}
	type="number"
	size="sm"
	color={valid ? 'base' : 'red'}
/>
