<script lang="ts">
	import type { InputType, StaticInput } from '../../inputType'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	export let value: any
	export let componentInput: StaticInput<any>
	export let subFieldType: InputType | undefined
	export let id: string | undefined

	let fakeComponentInput: StaticInput<any> = {
		...componentInput,
		value
	}

	// Bubble up changes to the real componentInput
	$: fakeComponentInput && (value = fakeComponentInput.value)
</script>

<StaticInputEditor
	{id}
	fieldType={subFieldType}
	bind:componentInput={fakeComponentInput}
	on:remove
/>
