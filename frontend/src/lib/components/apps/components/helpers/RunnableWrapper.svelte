<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'
	import { goto } from '$app/navigation'

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
	export let gotoUrl: string | undefined = undefined
	export let gotoNewTab: boolean | undefined = undefined
	export let setTab: Array<{ id: string; index: number }> | undefined = undefined
	export let render: boolean
	export let recomputeIds: string[] = []
	export let outputs: { result: Output<any>; loading: Output<boolean> }
	export let extraKey: string | undefined = undefined

	const { staticExporter, noBackend, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	$: if (initializing && result) {
		initializing = false
	}

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}
	onMount(() => {
		$staticExporter[id] = () => result
	})

	function isRunnableDefined(componentInput) {
		return isScriptByNameDefined(componentInput) || isScriptByPathDefined(componentInput)
	}

	export function onSuccess() {
		if (Array.isArray(setTab)) {
			setTab.forEach((tab) => {
				const { id, index } = tab
				$componentControl[id].setTab?.(index)
			})
		}

		if (gotoUrl && gotoUrl != '' && result?.error == undefined) {
			if (gotoNewTab) {
				window.open(gotoUrl, '_blank')
			} else {
				goto(gotoUrl)
			}
		}

		if (recomputeIds) {
			recomputeIds.map((id) => $runnableComponents?.[id]?.())
		}
	}
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined(componentInput)}
	<RunnableComponent
		{extraKey}
		bind:this={runnableComponent}
		fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		transformer={componentInput.transformer}
		{autoRefresh}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
		{initializing}
		wrapperClass={runnableClass}
		wrapperStyle={runnableStyle}
		{render}
		on:success={onSuccess}
		{outputs}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent {render} bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
