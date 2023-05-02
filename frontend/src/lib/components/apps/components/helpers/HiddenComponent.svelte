<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'

	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import RunnableComponent from './RunnableComponent.svelte'
	import InitializeComponent from './InitializeComponent.svelte'

	export let id: string
	export let runnable: HiddenRunnable

	const { worldStore, staticExporter, noBackend } = getContext<AppViewerContext>('AppViewerContext')

	let result: any = noBackend ? runnable.noBackendValue : undefined

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
		autoRefresh={runnable.autoRefresh ?? false}
		bind:result
		transformer={undefined}
		recomputeOnInputChanged={runnable.recomputeOnInputChanged ?? true}
		{runnable}
		wrapperClass="hidden"
		{outputs}
	>
		<slot />
	</RunnableComponent>
{:else}
	<InitializeComponent {id} />
{/if}
