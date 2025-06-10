<script lang="ts">
	import type { FlowState } from '../flowState'
	import type { FlowModule, OpenFlow } from '$lib/gen'
	import { dfs, getPreviousModule, getStepPropPicker } from '../previousResults'
	import type { FlowEditorContext } from '../types'
	import { getContext, onMount } from 'svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'

	interface Props {
		inputTransform: Record<string, any> | undefined
		id: string
	}

	let { inputTransform, id }: Props = $props()

	const { flowStore, flowStateStore, testSteps, previewArgs } =
		getContext<FlowEditorContext | undefined>('FlowEditorContext') || {}

	function updateStepArgs(flowState: FlowState | undefined, flow: OpenFlow | undefined) {
		if (!flowState || !flow) {
			return
		}
		const modules = dfs(id, flow, true)
		const previousModule = getPreviousModule(id, flow)
		if (modules.length < 1) {
			return
		}
		let parentModule: FlowModule | undefined = undefined
		if (modules.length > 2) {
			parentModule = modules[-1]
		}
		const stepPropPicker = getStepPropPicker(
			flowState,
			parentModule,
			previousModule,
			id,
			flow,
			$previewArgs,
			false
		)
		const pickableProperties = stepPropPicker.pickableProperties
		testSteps?.initializeFromSchema(modules[0], flowState[id]?.schema ?? {}, pickableProperties)
	}

	onMount(() => {
		updateStepArgs($flowStateStore, $flowStore)
	})
</script>

<div class="p-4 h-full overflow-y-auto">
	<ObjectViewer json={testSteps?.getStepArgs(id)} {inputTransform} />
</div>
