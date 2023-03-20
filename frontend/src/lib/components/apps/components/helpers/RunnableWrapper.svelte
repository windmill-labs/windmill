<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'

	export let componentInput: AppInput | undefined
	export let id: string
	export let result: any = undefined
	export let initializing: boolean = true

	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
	export let forceSchemaDisplay: boolean = false
	export let runnableClass = ''
	export let runnableStyle = ''
	export let goto: string | undefined = undefined
	export let gotoNewTab: boolean | undefined = undefined
	export let render: boolean
	export let recomputeIds: string[] = []

	const { staticExporter, noBackend } = getContext<AppViewerContext>('AppViewerContext')

	$: if (initializing && result) {
		initializing = false
	}

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}
	onMount(() => {
		$staticExporter[id] = () => result
	})

	function isRunnableDefined() {
		return isScriptByNameDefined(componentInput) || isScriptByPathDefined(componentInput)
	}
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined()}
	<RunnableComponent
		{recomputeIds}
		gotoUrl={goto}
		{gotoNewTab}
		bind:this={runnableComponent}
		fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		{autoRefresh}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
		{initializing}
		wrapperClass={runnableClass}
		wrapperStyle={runnableStyle}
		{render}
		on:success
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent {render} bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
