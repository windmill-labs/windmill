<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import { type GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'

	export let data: {
		path: string
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		index: number
		disableAi: boolean
		flowIsSimplifiable: boolean
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper wrapperClass="shadow-none" let:darkMode>
	<TriggersWrapper
		{darkMode}
		{data}
		path={data.path}
		on:new={(e) => {
			data?.eventHandlers.insert({
				modules: data.modules,
				index: 0,
				kind: 'trigger',
				inlineScript: e.detail.inlineScript
			})
			data?.eventHandlers.insert({
				modules: data.modules,
				index: 1,
				kind: 'forloop',
				light: true
			})
		}}
		on:pickScript={(e) => {
			data?.eventHandlers.insert({
				modules: data.modules,
				index: 0,
				kind: 'trigger',
				script: e.detail
			})
			data?.eventHandlers.insert({
				modules: data.modules,
				index: 1,
				kind: 'forloop',
				light: true
			})
		}}
		on:select={() => {
			data?.eventHandlers?.select('triggers')
		}}
		isFlow={true}
		selected={$selectedId == 'triggers'}
		newItem={data.newFlow}
	/>
</NodeWrapper>
