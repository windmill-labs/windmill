<script lang="ts">
	import ObjectResourceInput from '../ObjectResourceInput.svelte'

	import BooleanInput from './BooleanInput.svelte'
	import NumberInput from './NumberInput.svelte'
	import ObjectInput from './ObjectInput.svelte'

	export let type: string
	export let value: any
	export let valid: boolean = true
	export let disabled: boolean = false
	export let placeholder: string = ''
	export let format: string = ''

	const inputs = [
		{ type: 'number', component: NumberInput, props: { valid, disabled, placeholder } },
		{ type: 'boolean', component: BooleanInput, props: {} },
		{ type: 'resource-object', component: ObjectResourceInput, props: { format } },
		{ type: 'object', component: ObjectInput }
	]
	const selected = inputs.find((input) => input.type === type)
</script>

{#if selected}
	<svelte:component this={selected.component} {...selected.props} bind:value />
{/if}
