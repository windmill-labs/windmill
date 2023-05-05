<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput, selectId } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import type { Schema } from '$lib/common'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore, connectingInput, app, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		values: {}
	})

	let result: Schema | undefined = undefined
	let args: Record<string, unknown> = {}

	function handleArgsChange() {
		const newArgs: Record<string, unknown> = {}

		for (const key in args) {
			if (result?.properties[key]) {
				newArgs[key] = args[key]
			}
		}

		outputs.values.set(newArgs, true)
	}

	$: args && handleArgsChange()
</script>

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if result && Object.keys(result.properties).length > 0}
		<div
			class="m-2"
			on:pointerdown|stopPropagation={(e) =>
				!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
		>
			<LightweightSchemaForm schema={result} bind:args />
		</div>
	{:else}
		<p class="m-2 italic"> Empty form </p>
	{/if}
</RunnableWrapper>
