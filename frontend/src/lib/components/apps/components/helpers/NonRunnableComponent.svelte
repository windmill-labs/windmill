<script lang="ts">
	import { getContext, createEventDispatcher } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import InputValue from './InputValue.svelte'
	import InitializeComponent from './InitializeComponent.svelte'

	export let componentInput: AppInput
	export let id: string
	export let result: any
	export let render: boolean

	const dispatch = createEventDispatcher()

	// Sync the result to the output
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	$: outputs = $worldStore?.outputsById?.[id] as {
		result: Output<any>
	}

	function setOutput(newResult: any) {
		dispatch('done')
		outputs?.result?.set(newResult, true)
	}

	$: result && outputs && setOutput(result)
</script>

<InitializeComponent {id} />

{#if componentInput.type !== 'runnable'}
	<InputValue {id} input={componentInput} bind:value={result} />
{/if}

{#if render}
	<slot />
{:else}
	<div class="w-full h-full" />
{/if}
