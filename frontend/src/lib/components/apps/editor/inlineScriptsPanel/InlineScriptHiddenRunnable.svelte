<script lang="ts">
	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from './InlineScriptRunnableByPath.svelte'
	import type { Runnable, StaticAppInput } from '../../inputType'
	import { createEventDispatcher, getContext } from 'svelte'

	interface Props {
		runnable: HiddenRunnable
		id: string
		transformer: boolean
	}

	let { runnable = $bindable(), id, transformer }: Props = $props()

	const { runnableComponents, app } = getContext<AppViewerContext>('AppViewerContext')
	async function fork(nrunnable: Runnable) {
		runnable = { ...runnable, ...nrunnable, autoRefresh: true, recomputeOnInputChanged: true }
	}

	function onPick(o: { runnable: Runnable; fields: Record<string, StaticAppInput> }) {
		runnable = {
			...runnable,
			...o.runnable,
			fields: o.fields,
			autoRefresh: true,
			recomputeOnInputChanged: true
		}
	}
	const dispatch = createEventDispatcher()
</script>

{#if transformer}
	{#if runnable.transformer}
		<InlineScriptEditor
			transformer
			defaultUserInput={false}
			{id}
			componentType={undefined}
			bind:inlineScript={runnable.transformer}
			name="Transformer"
			on:delete={() => {
				if (runnableComponents) {
					delete $runnableComponents[id]
					runnable.transformer = undefined
					runnable = runnable
				}
			}}
		/>
	{:else}
		<div class="px-2 pt-4 text-tertiary">
			Selected editor component is a transformer but component has no transformer
		</div>
	{/if}
{:else if runnable?.type === 'runnableByName' && runnable.inlineScript}
	<InlineScriptEditor
		on:createScriptFromInlineScript={() => dispatch('createScriptFromInlineScript', runnable)}
		{id}
		bind:inlineScript={runnable.inlineScript}
		bind:name={runnable.name}
		bind:fields={runnable.fields}
		syncFields
		on:delete
	/>
{:else if runnable?.type == 'runnableByPath'}
	<InlineScriptRunnableByPath
		bind:runnable
		bind:fields={runnable.fields}
		on:fork={(e) => fork(e.detail)}
		on:delete
		{id}
	/>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={$app?.unusedInlineScripts}
		on:pick={(e) => onPick(e.detail)}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'runnableByName',
				inlineScript: e.detail,
				name: runnable.name,
				fields: {},
				autoRefresh: true,
				recomputeOnInputChanged: true,
				recomputeIds: []
			}
		}}
	/>
{/if}
