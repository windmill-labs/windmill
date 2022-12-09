<script lang="ts">
	import type { AppInput } from '../../inputType'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let value: any
	export let canHide: boolean = false
	export let componentInput: Extract<AppInput, { type: 'static' }>

	let fakeComponentInput: Extract<AppInput, { type: 'static' }> = {
		...componentInput,
		value,
		visible: componentInput.visible,
		// We don't support array of arrays
		// @ts-ignore
		fieldType: componentInput.subFieldType
	}

	// Bubble up changes to the real componentInput
	$: fakeComponentInput && (value = fakeComponentInput.value)
</script>

<StaticInputEditor bind:componentInput={fakeComponentInput} {canHide} />
