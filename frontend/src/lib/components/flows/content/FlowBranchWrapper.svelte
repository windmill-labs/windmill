<script lang="ts">
	import type { BranchOne, FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import { flowStateStore } from '../flowState'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { flowModuleMap } from '../flowModuleMap'
	import { getStepPropPicker } from '../previousResults'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
	let editor: SimpleEditor | undefined = undefined

	$: moduleMapped = $flowModuleMap[$selectedId]

	const { flowModule, parentModuleId, previousModuleId } = moduleMapped

	$: flowValue = flowModule.value as BranchOne

	$: branch = flowValue.branches[Number(21) - 1] as {
		summary?: string
		expr: string
		modules: Array<FlowModule>
	}

	$: pickableProperties = getStepPropPicker(
		flowModule.id,
		$flowStore.schema,
		$flowStateStore,
		$flowModuleMap,
		$previewArgs
	).pickableProperties
</script>

<div class="h-full flex flex-col">
	<FlowCard title="Branch">
		<div slot="header" class="grow">
			<input bind:value={branch.summary} placeholder={'Summary'} />
		</div>
		<div class="overflow-hidden flex-grow">
			<Splitpanes>
				<Pane>
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
				</Pane>
			</Splitpanes>
		</div>
	</FlowCard>
</div>
