<script lang="ts">
	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from './InlineScriptRunnableByPath.svelte'
	import {
		isRunnableByName,
		isRunnableByPath,
		type Runnable,
		type StaticAppInput
	} from '../../inputType'
	import { createEventDispatcher, getContext } from 'svelte'

	interface Props {
		runnable: HiddenRunnable
		id: string
		transformer: boolean
		oncreateScriptFromInlineScript?: (...args: any[]) => any
		ondelete?: (...args: any[]) => any
	}

	let {
		runnable = $bindable(),
		id,
		transformer,
		oncreateScriptFromInlineScript = undefined,
		ondelete = undefined
	}: Props = $props()

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
			ondelete={() => {
				if (runnableComponents) {
					delete $runnableComponents[id]
					runnable.transformer = undefined
					runnable = runnable
				}
			}}
		/>
	{:else}
		<div class="px-2 pt-4 text-primary">
			Selected editor component is a transformer but component has no transformer
		</div>
	{/if}
{:else if isRunnableByName(runnable) && runnable.inlineScript}
	<InlineScriptEditor
		oncreateScriptFromInlineScript={() => (
			dispatch('createScriptFromInlineScript', runnable),
			oncreateScriptFromInlineScript?.(runnable)
		)}
		{id}
		bind:inlineScript={runnable.inlineScript}
		bind:name={runnable.name}
		bind:fields={runnable.fields}
		syncFields
		ondelete={ondelete}
	/>
{:else if isRunnableByPath(runnable)}
	<InlineScriptRunnableByPath
		bind:runnable
		bind:fields={runnable.fields}
		onfork={(e) => fork(e)}
		ondelete={ondelete}
		{id}
	/>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={$app?.unusedInlineScripts}
		onpick={(e) => onPick(e)}
		ondelete={ondelete}
		showScriptPicker
		onnew={(e) => {
			runnable = {
				type: 'inline',
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
