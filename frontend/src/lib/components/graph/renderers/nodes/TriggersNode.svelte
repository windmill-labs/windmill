<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'

	export let data: {
		path: string
		openSchedules: () => void
		triggerDetail: (e: { detail: 'webhook' | 'mail' | 'schedule' }) => void
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		index: number
		disableAi: boolean
		flowIsSimplifiable: boolean
	}
</script>

<NodeWrapper wrapperClass="shadow-none">
	<TriggersWrapper
		{data}
		path={data.path}
		on:new={(e) => {
			console.log('new', e)
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
				script: e.detail
			})
			data?.eventHandlers.insert({
				modules: data.modules,
				index: 1,
				kind: 'forloop',
				light: true
			})
		}}
		on:openSchedules={() => data.openSchedules()}
		on:triggerDetail={(e) => {
			data.triggerDetail(e.detail)
		}}
		isEditor={data.isEditor}
		newFlow={data.newFlow}
	/>
</NodeWrapper>
