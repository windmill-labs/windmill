<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'

	export let componentInput: AppInput | undefined
	export let id: string
	export let result: any = undefined

	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
	export let forceSchemaDisplay: boolean = false

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function isRunnableDefined() {
		if (!componentInput) return false
		if (componentInput.type !== 'runnable') return false

		if (
			componentInput.runnable?.type === 'runnableByName' &&
			$app.inlineScripts[componentInput.runnable.inlineScriptName] === undefined
		) {
			return false
		}

		return true
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
