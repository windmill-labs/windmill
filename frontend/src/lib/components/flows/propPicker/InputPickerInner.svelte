<script lang="ts">
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

	onMount(() => {
		testSteps?.updateStepArgs(id, $flowStateStore, flowStore?.val, previewArgs?.val)
	})
</script>

<div class="p-4 pr-6 h-full overflow-y-auto">
	<ObjectViewer json={testSteps?.getStepArgs(id)} {inputTransform} />
</div>
