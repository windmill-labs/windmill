<script lang="ts">
	import { tick } from 'svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		value: string
		placeholder: string
		disabled?: boolean
		class?: string
	}

	let {
		value = $bindable(),
		placeholder,
		disabled = false,
		class: className = ''
	}: Props = $props()

	let open = $state(false)
	let input: TextInput | undefined = $state()

	async function reveal() {
		open = true
		await tick()
		input?.focus()
	}
</script>

{#if open}
	<TextInput
		bind:this={input}
		bind:value
		class={className}
		inputProps={{ placeholder, maxlength: 1000, disabled }}
	/>
{:else}
	<Button variant="subtle" unifiedSize="sm" {disabled} onClick={reveal}>Add a reason</Button>
{/if}
