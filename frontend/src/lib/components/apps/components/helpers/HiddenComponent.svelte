<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'

	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import RunnableComponent from './RunnableComponent.svelte'
	import InitializeComponent from './InitializeComponent.svelte'

	export let id: string
	export let runnable: HiddenRunnable

	const { worldStore, staticExporter, noBackend, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let result: any = noBackend ? runnable.noBackendValue : undefined

	export function onSuccess() {
		if (runnable.recomputeIds) {
			runnable.recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb()))
		}
	}

	onMount(() => {
		$staticExporter[id] = () => {
			return result
		}
	})

	let outputs = initOutput($worldStore, id, {
		result: result,
		loading: false
	})
</script>

{#if runnable && (runnable.type == 'runnableByPath' || (runnable.type == 'runnableByName' && runnable.inlineScript != undefined))}
	<RunnableComponent
		render={false}
		{id}
		fields={runnable.fields}
		autoRefresh={true}
		bind:result
		transformer={runnable?.transformer}
		recomputeOnInputChanged={runnable.recomputeOnInputChanged ?? true}
		{runnable}
		wrapperClass="hidden"
		recomputableByRefreshButton={runnable.autoRefresh ?? true}
		on:success={onSuccess}
		{outputs}
	>
		<slot />
	</RunnableComponent>
{:else}
	<InitializeComponent {id} />
{/if}
