<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import InputValue from './InputValue.svelte'

	export let result: any = undefined
	export let componentInput: AppInput
	export let id: string

	// Sync the result to the output
	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: outputs = $worldStore?.outputsById[id] as {
		loading: Output<boolean>
		result: Output<any>
	}

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	function setOutput() {
		if (outputs) {
			outputs.result?.set(result)
		}
	}

	$: result !== undefined && setOutput()
</script>

{#if componentInput.type !== 'runnable'}
	<InputValue {id} input={componentInput} bind:value={result} />
{/if}

<slot />
