<script lang="ts">
	import type { ComponentInput } from '../../inputType'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'

	export let componentInput: ComponentInput | undefined
	export let id: string
	export let result: any = undefined

	// Optional props
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable'}
	<RunnableComponent
		bind:this={runnableComponent}
		bind:fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		{autoRefresh}
		{id}
		{extraQueryParams}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
