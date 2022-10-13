<script lang="ts">
	import type { Branches, FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import { VSplitPane } from 'svelte-split-pane'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import { getStepPropPicker } from '../flowStateUtils'
	import { flowStateStore } from '../flowState'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let editor: SimpleEditor | undefined = undefined
	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	$: [branchKeyword, parentIndex, branchIndex] = $selectedId.split('-')

	$: flowModule = $flowStore.value.modules[parentIndex]
	$: flowValue = flowModule.value as Branches
	$: branch = flowValue.branches[Number(branchIndex) - 1] as {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}

	$: pickableProperties = getStepPropPicker(
		[Number(parentIndex)],
		$flowStore.schema,
		$flowStateStore,
		$previewArgs
	).pickableProperties
</script>

<div class="h-full flex flex-col">
	<FlowCard title="Branch">
		<div slot="header" class="grow">
			<input bind:value={branch.summary} placeholder={'Summary'} />
		</div>
		<div class="overflow-hidden flex-grow">
			<VSplitPane topPanelSize="60%" downPanelSize="40%" minTopPaneSize="20%" minDownPaneSize="20%">
				<top slot="top" class="h-full">
					<div class="p-6 flex flex-col h-full overflow-clip">
						<span class="mb-2 text-sm font-bold">Branch predicate</span>
						<div class="border w-full">
							<PropPickerWrapper
								{pickableProperties}
								on:select={({ detail }) => {
									editor?.insertAtCursor(detail)
								}}
							>
								<SimpleEditor
									bind:this={editor}
									lang="javascript"
									bind:code={branch.expr}
									class="small-editor"
									shouldBindKey={false}
								/>
							</PropPickerWrapper>
						</div>
					</div>
				</top>
				<down slot="down" class="flex flex-col flex-1 h-full" />
			</VSplitPane>
		</div>
	</FlowCard>
</div>
