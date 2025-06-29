<script lang="ts">
	import { readFieldsRecursively } from '$lib/utils'
	import type { InputType, StaticInput } from '../../inputType'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	interface Props {
		value: any
		componentInput: StaticInput<any>
		subFieldType: InputType | undefined
		id: string | undefined
	}

	let { value = $bindable(), componentInput = $bindable(), subFieldType, id }: Props = $props()

	let fakeComponentInput: StaticInput<any> = $state({
		...componentInput,
		value
	})

	$effect(() => {
		readFieldsRecursively(fakeComponentInput)
		// console.log('fakeComponentInput', fakeComponentInput.value)
		value = fakeComponentInput.value
	})
	// Bubble up changes to the real componentInput
</script>

<StaticInputEditor
	{id}
	fieldType={subFieldType}
	bind:componentInput={fakeComponentInput}
	on:remove
/>
