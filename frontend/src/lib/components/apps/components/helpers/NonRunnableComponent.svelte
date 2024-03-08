<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import InputValue from './InputValue.svelte'
	import InitializeComponent from './InitializeComponent.svelte'

	export let componentInput: AppInput
	export let id: string
	export let result: any
	export let render: boolean
	export let hasChildrens: boolean
	export let noInitialize

	// Sync the result to the output
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	$: outputs = $worldStore?.outputsById?.[id] as {
		result: Output<any>
	}

	function setOutput(v: any) {
		// console.log('setnr', id)
		outputs?.result?.set(v, true)
	}

	$: result != undefined && outputs && setOutput(result)
</script>

{#if !noInitialize}
	<InitializeComponent {id} />
{/if}

{#if componentInput.type !== 'runnable'}
	<InputValue key="nonrunnable" {id} input={componentInput} bind:value={result} />
{/if}

{#if render || hasChildrens}
	<div class={render ? 'h-full w-full' : 'invisible h-0 overflow-hidden'}>
		<slot />
	</div>
{:else}
	<div class="w-full h-full" />
{/if}
