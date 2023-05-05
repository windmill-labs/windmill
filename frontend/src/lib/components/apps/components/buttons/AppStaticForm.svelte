<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import type { Schema } from '$lib/common'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: Schema | undefined = undefined
</script>

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if result}
		<div class="m-2">
			<LightweightSchemaForm schema={result} args={{}} />
		</div>
	{/if}
</RunnableWrapper>
