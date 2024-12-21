<script lang="ts">
	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from './InlineScriptRunnableByPath.svelte'
	import type { Runnable, StaticAppInput } from '../../inputType'
	import { createEventDispatcher, getContext } from 'svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'

	export let runnable: HiddenRunnable
	export let id: string
	export let transformer: boolean
	export let rawApps: boolean = false

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
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
				delete $runnableComponents[id]
				runnable.transformer = undefined
				runnable = runnable
			}}
		/>
	{:else}
		<div class="px-2 pt-4 text-tertiary">
			Selected editor component is a transformer but component has no transformer
		</div>
	{/if}
{:else if runnable?.type === 'runnableByName' && runnable.inlineScript}
	{#if rawApps}
		{#if runnable.inlineScript.language == 'frontend'}
			<div class="text-sm text-tertiary">Frontend scripts not supported for raw apps</div>
		{:else}
			<ScriptEditor
				noHistory
				noSyncFromGithub
				lang={runnable.inlineScript.language}
				path={runnable.inlineScript.path ? runnable.inlineScript.path + '_fullscreen' : undefined}
				fixedOverflowWidgets={false}
				bind:code={runnable.inlineScript.content}
				bind:schema={runnable.inlineScript.schema}
				on:createScriptFromInlineScript
				tag={undefined}
				saveToWorkspace
			/>
		{/if}
	{:else}
		<InlineScriptEditor
			on:createScriptFromInlineScript={() => dispatch('createScriptFromInlineScript', runnable)}
			{id}
			bind:inlineScript={runnable.inlineScript}
			bind:name={runnable.name}
			bind:fields={runnable.fields}
			syncFields
			on:delete
		/>
	{/if}
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
		{rawApps}
		on:pick={(e) => onPick(e.detail)}
		name={runnable.name}
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
