<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import InputValue from './InputValue.svelte'

	export let componentInput: AppInput
	export let id: string
	export let result: any
	export let render: boolean

	// Sync the result to the output
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	$: outputs = $worldStore?.outputsById[id] as {
		loading: Output<boolean>
		result: Output<any>
	}

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	function setOutput(v: any) {
		outputs.result?.set(v, true)
	}

	$: result && outputs && setOutput(result)
</script>

{#if componentInput.type !== 'runnable'}
	<InputValue {id} input={componentInput} bind:value={result} />
{/if}

{#if render}
	<slot />
{:else}
	<div class="w-full h-full">
		<div
			out:fade|local={{ duration: 50 }}
			class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
		>
			<Loader2 class="animate-spin" size={16} />
			<span class="text-xs mt-1">Loading</span>
		</div>
	</div>
{/if}
