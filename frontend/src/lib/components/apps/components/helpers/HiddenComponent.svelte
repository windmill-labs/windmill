<script lang="ts">
	import { getContext } from 'svelte'
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
	export let autoRefresh: boolean = false

	let result: any = undefined

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})
</script>

<RunnableComponent
	render={false}
	{id}
	{fields}
	{autoRefresh}
	bind:result
	runnable={{
		name,
		inlineScript,
		type: 'runnableByName'
	}}
	wrapperClass="hidden"
	recomputable
>
	<slot />
</RunnableComponent>
