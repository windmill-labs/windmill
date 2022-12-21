<script lang="ts">
	import type { AppInput } from '../../inputType'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'

	export let componentInput: AppInput | undefined
	export let id: string
	export let result: any = undefined

	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
	export let forceSchemaDisplay: boolean = false

	function isRunnableDefined() {
		return isScriptByNameDefined(componentInput) || isScriptByPathDefined(componentInput)
	}
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined()}
	<RunnableComponent
		bind:this={runnableComponent}
		bind:inputs={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		{autoRefresh}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
