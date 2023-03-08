<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
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
	export let flexWrap = false
	export let runnableClass = ''
	export let runnableStyle = ''
	export let goto: string | undefined = undefined
	export let gotoNewTab: boolean | undefined = undefined
	export let render: boolean

	const { staticExporter, noBackend } = getContext<AppEditorContext>('AppEditorContext')

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
		gotoUrl={goto}
		{gotoNewTab}
		{flexWrap}
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
	>
		<slot />
	</RunnableComponent>
{:else if render}
	<NonRunnableComponent bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{:else if !render}
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
