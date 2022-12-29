<script lang="ts">
	import type { StaticAppInput } from '../../inputType'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	export let value: any
	export let componentInput: StaticAppInput

	let fakeComponentInput: StaticAppInput = {
		...componentInput,
		value,
		// We don't support array of arrays
		// @ts-ignore
		fieldType: componentInput.subFieldType
	}

	// Bubble up changes to the real componentInput
	$: fakeComponentInput && (value = fakeComponentInput.value)
</script>

<StaticInputEditor bind:componentInput={fakeComponentInput} />
