<script lang="ts">
	import { getContext, onMount, untrack } from 'svelte'
	import { initOutput } from '../../editor/appUtils'

	import type { AppViewerContext, HiddenRunnable } from '../../types'
	import RunnableComponent from './RunnableComponent.svelte'
	import InitializeComponent from './InitializeComponent.svelte'
	import { isRunnableByName, isRunnableByPath } from '../../inputType'

	interface Props {
		id: string
		runnable: HiddenRunnable
		children?: import('svelte').Snippet
	}

	let { id, runnable, children }: Props = $props()

	const { worldStore, staticExporter, noBackend, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let result: any = $state(noBackend ? runnable.noBackendValue : undefined)
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
		result: untrack(() => result),
		loading: false,
		jobId: undefined
	})
</script>

{#if runnable && (isRunnableByPath(runnable) || (isRunnableByName(runnable) && runnable.inlineScript != undefined))}
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
		{@render children?.()}
	</RunnableComponent>
{:else}
	<InitializeComponent {id} />
{/if}
