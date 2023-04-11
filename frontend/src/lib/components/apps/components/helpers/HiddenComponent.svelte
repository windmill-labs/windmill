<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import type { AppViewerContext, InlineScript } from '../../types'
	import RunnableComponent from './RunnableComponent.svelte'

	export let id: string
	export let name: string
	export let inlineScript: InlineScript | undefined
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let recomputeOnInputChanged: boolean
	export let recomputableByRefreshButton: boolean
	export let noBackendValue: any = undefined

	const { worldStore, staticExporter, noBackend } = getContext<AppViewerContext>('AppViewerContext')

	let result: any = noBackend ? noBackendValue : undefined

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

<RunnableComponent
	render={false}
	{id}
	{fields}
	autoRefresh={true}
	{recomputeOnInputChanged}
	bind:result
	transformer={undefined}
	runnable={{
		name,
		inlineScript,
		type: 'runnableByName'
	}}
	wrapperClass="hidden"
	{outputs}
	{recomputableByRefreshButton}
>
	<slot />
</RunnableComponent>
