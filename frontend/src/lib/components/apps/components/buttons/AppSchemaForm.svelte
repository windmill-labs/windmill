<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import type { Schema } from '$lib/common'
	import { Button } from '$lib/components/common'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		values: {}
	})

	let result: Schema | undefined = undefined
	let args: Record<string, unknown> = {}
</script>

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if result && Object.keys(result.properties).length > 0}
		<div class="m-2">
			<LightweightSchemaForm schema={result} bind:args />
			<div class="flex justify-end">
				<Button
					color="dark"
					variant="contained"
					btnClasses="mt-2"
					size="xs"
					on:click={() => {
						outputs.values.set(args, true)
					}}
				>
					Submit
				</Button>
			</div>
		</div>
	{:else}
		<p class="m-2 italic"> Empty form </p>
	{/if}
</RunnableWrapper>
