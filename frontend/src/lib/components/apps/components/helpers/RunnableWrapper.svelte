<script lang="ts">
	import type { AppInput } from '../../inputType'
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
		if (!componentInput) return false
		if (componentInput.type !== 'runnable') return false

		if (
			(componentInput.runnable?.type === 'runnableByName' &&
				componentInput.runnable.inlineScript) ||
			(componentInput.runnable?.type === 'runnableByPath' && componentInput.runnable.path)
		) {
			return true
		}

		return false
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
