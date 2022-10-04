<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { flowStore } from '../flowStore'
	import { getStepPropPicker } from '../flowStateUtils'
	import { flowStateStore } from '../flowState'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import { selectedIdToIndexes, selectedIdToModule } from '../utils'
	import { Button } from '$lib/components/common'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	$: mod = selectedIdToModule($selectedId, $flowStore)

	let editor: SimpleEditor | undefined = undefined

	$: index = selectedIdToIndexes($selectedId)[1]

	$: pickableProperties = getStepPropPicker(
		[Number(index)],
		$flowStore.schema,
		$flowStateStore,
		$previewArgs
	).pickableProperties
</script>

<FlowCard title="For loop">
	<div slot="header" class="grow">
		<input bind:value={mod.summary} placeholder={'Summary'} />
	</div>
	<div>
		<div class="p-6 flex flex-col">
			{#if mod.value.type === 'forloopflow'}
				<span class="mb-2 text-sm font-bold"
					>Iterator expression
					<Tooltip>
						List to iterate over. For more information see the
						<a href="https://docs.windmill.dev/docs/getting_started/flows#for-loops">docs.</a>
					</Tooltip>
				</span>

				{#if mod.value.iterator.type == 'javascript'}
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
								bind:code={mod.value.iterator.expr}
								class="small-editor"
								shouldBindKey={false}
							/>
						</PropPickerWrapper>
					</div>
				{:else}
					<Button
						on:click={() => {
							if (mod.value.type === 'forloopflow') mod.value.iterator.type = 'javascript'
						}}
					/>
				{/if}

				<span class="my-2 text-sm font-bold">Skip failures</span>

				<Toggle
					bind:checked={mod.value.skip_failures}
					options={{
						right: 'Skip failures'
					}}
				/>
			{/if}
		</div>
	</div>
</FlowCard>
