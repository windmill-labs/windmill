<script lang="ts">
	import type { AppInput, Runnable } from '../../inputType'
	import { clearResultAppInput } from '../../utils'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'

	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import InlineScriptRunnableByPath from './InlineScriptRunnableByPath.svelte'

	export let componentInput: AppInput | undefined
	export let defaultUserInput = false
	export let componentType: string
	export let id: string
	export let transformer: boolean

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	function clear() {
		if (componentInput && componentInput.type == 'runnable') {
			componentInput = clearResultAppInput(componentInput)
			$app = $app
		}
	}

	async function fork(runnable: Runnable) {
		if (componentInput?.type == 'runnable') {
			componentInput.runnable = runnable
		}
	}

	const dispatch = createEventDispatcher()
</script>

{#if transformer}
	{#if componentInput?.type == 'runnable' && componentInput.transformer}
		<InlineScriptEditor
			transformer
			defaultUserInput={false}
			{id}
			{componentType}
			bind:inlineScript={componentInput.transformer}
			name="Transformer"
			on:delete={() => {
				if (componentInput?.type == 'runnable') {
					componentInput.transformer = undefined
					componentInput = componentInput
				}
			}}
		/>
	{:else}
		<span class="px-2 text-tertiary">
			Selected editor component is a transformer but component has no transformer
		</span>
	{/if}
{:else if componentInput?.type == 'runnable'}
	{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.name !== undefined}
		{#if componentInput.runnable.inlineScript}
			<InlineScriptEditor
				on:createScriptFromInlineScript={() => {
					if (
						componentInput?.type == 'runnable' &&
						componentInput?.runnable?.type === 'runnableByName'
					) {
						dispatch('createScriptFromInlineScript', componentInput?.runnable)
					}
				}}
				{defaultUserInput}
				{id}
				{componentType}
				bind:inlineScript={componentInput.runnable.inlineScript}
				bind:name={componentInput.runnable.name}
				bind:fields={componentInput.fields}
				syncFields
				on:delete={clear}
			/>
		{:else}
			<EmptyInlineScript
				{componentType}
				name={componentInput.runnable.name}
				on:delete={clear}
				on:new={(e) => {
					if (
						componentInput &&
						componentInput.type == 'runnable' &&
						componentInput?.runnable?.type === 'runnableByName'
					) {
						componentInput.runnable.inlineScript = e.detail
						componentInput.autoRefresh = true
						componentInput.recomputeOnInputChanged = true
						$app = $app
					}
				}}
			/>
		{/if}
	{:else if componentInput?.runnable?.type === 'runnableByPath' && componentInput?.runnable?.path}
		<InlineScriptRunnableByPath
			on:fork={(e) => fork(e.detail)}
			bind:runnable={componentInput.runnable}
			bind:fields={componentInput.fields}
			on:delete={clear}
			{id}
		/>
	{/if}
{/if}
