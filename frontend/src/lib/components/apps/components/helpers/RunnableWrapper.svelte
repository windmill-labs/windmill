<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	export let componentInput: AppInput | undefined
	export let id: string
	export let result: any = undefined

	// Optional props
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
</script>

{#if componentInput === undefined}
	<slot />
{:else if componentInput.type === 'runnable' && componentInput.runnable}
	<RunnableComponent
		bind:this={runnableComponent}
		bind:inputs={componentInput.fields}
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
