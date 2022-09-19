<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { flowStateStore } from '../flowState'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'

	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	const [prefix, index] = $selectedId.split('-')
</script>

<FlowCard title="For loop">
	<div>
		<div class="p-6 flex flex-col">
			{#if $flowStateStore.modules[index].flowModule.value.type === 'forloopflow'}
				<span class="mb-2 text-sm font-bold">Iterator expression</span>
				<SimpleEditor
					lang="javascript"
					bind:code={$flowStateStore.modules[index].flowModule.value.iterator.expr}
					class="few-lines-editor"
				/>
				<span class="mb-2 text-sm font-bold">Skip failures</span>

				<Toggle
					bind:checked={$flowStateStore.modules[index].flowModule.value.skip_failures}
					options={{
						right: 'Skip failures'
					}}
				/>
			{/if}
		</div>
	</div>
</FlowCard>
