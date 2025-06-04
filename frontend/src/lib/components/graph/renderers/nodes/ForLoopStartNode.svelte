<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule, FlowStatusModule } from '$lib/gen'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import { getContext } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import type { ForLoopStartN } from '../../graphBuilder.svelte'

	interface Props {
		data: ForLoopStartN['data']
	}

	let { data }: Props = $props()

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const pickablePropertiesFiltered = propPickerContext?.pickablePropertiesFiltered

	function filterIterFromInput(inputJson: Record<string, any> | undefined): Record<string, any> {
		if (!inputJson || typeof inputJson !== 'object' || (!inputJson.iter && !inputJson.iter_parent))
			return {}
		const selectedIdIsDescendant = isSelectedDescendant(data.module, data.selectedId)
		if (selectedIdIsDescendant === 'child') {
			return { iter: inputJson.iter }
		}
		if (selectedIdIsDescendant === 'grandchild') {
			return { iter_parent: inputJson.iter_parent }
		}
		return {}
	}

	function computeStatus(
		state: GraphModuleState | undefined
	): FlowStatusModule['type'] | undefined {
		if (state?.type == 'InProgress' || state?.type == 'Success' || state?.type == 'Failure') {
			let r = state?.flow_jobs_success?.[state?.selectedForloopIndex ?? 0]
			if (r == undefined) return 'InProgress'
			return r ? 'Success' : 'InProgress'
		}
	}

	function isSelectedDescendant(
		module: FlowModule,
		selectedId: string
	): 'child' | 'grandchild' | 'none' {
		if (!selectedId) return 'none'
		// Check direct children
		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules.some((m) => m.id === selectedId)) {
				return 'child'
			}
			// Check grandchildren
			return module.value.modules.some(
				(m) =>
					(m.value.type === 'forloopflow' || m.value.type === 'whileloopflow') &&
					m.value.modules.some((gm) => gm.id === selectedId)
			)
				? 'grandchild'
				: 'none'
		}
		return 'none'
	}
	let filteredInput = $derived(filterIterFromInput($pickablePropertiesFiltered?.flow_input))
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.simplifiedTriggerView ? 'For each new event' : 'Do one iteration'}
			selectable={false}
			selected={false}
			id={data.id}
			hideId
			bgColor={getStateColor(undefined, darkMode)}
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			borderColor={getStateColor(computeStatus(data.flowModuleStates?.[data.id]), darkMode)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
			inputJson={filteredInput}
			prefix="flow_input"
			editMode={data.editMode}
		/>
	{/snippet}
</NodeWrapper>
