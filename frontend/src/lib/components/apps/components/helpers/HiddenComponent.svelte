<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import type { InlineScript } from '../../types'
	import RunnableComponent from './RunnableComponent.svelte'

	export let id: string
	export let name: string
	export let inlineScript: InlineScript | undefined
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>

	let result: any = undefined

	export const staticOutputs: string[] = ['result', 'loading']
</script>

<RunnableComponent
	{id}
	{fields}
	bind:result
	runnable={{
		name,
		inlineScript,
		type: 'runnableByName'
	}}
	wrapperClass="hidden"
>
	<slot />
</RunnableComponent>
