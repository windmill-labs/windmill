<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	// import FlowRetries from './FlowRetries.svelte'
	import { Button, Drawer, Tab, TabContent, Tabs, Alert } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { getStepPropPicker } from '../previousResults'
	import { enterpriseLicense } from '$lib/stores'

	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleMock from './FlowModuleMock.svelte'
	import { Play } from 'lucide-svelte'
	import type { Job } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import IteratorGen from '$lib/components/copilot/IteratorGen.svelte'

	const { previewArgs, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	export let mod: FlowModule
	export let parentModule: FlowModule | undefined
	export let previousModule: FlowModule | undefined
	export let noEditor: boolean
	export let enableAi = false

	let editor: SimpleEditor | undefined = undefined
	let selected: string = 'early-stop'

	$: stepPropPicker = getStepPropPicker(
		$flowStateStore,
		parentModule,
		previousModule,
		mod.id,
		$flowStore,
		$previewArgs,
		false
	)

	let previewOpen = false
	let jobId: string | undefined = undefined
	let job: Job | undefined = undefined

	let iteratorFieldFocused = false
	let iteratorGen: IteratorGen | undefined = undefined

	$: previewIterationArgs = $flowStateStore[mod.id]?.previewArgs ?? {}
</script>

<Drawer bind:open={previewOpen} alwaysOpen size="75%">
	<FlowLoopIterationPreview
		modules={mod.value.type == 'forloopflow' ? mod.value.modules : []}
		open={previewOpen}
		previewArgs={previewIterationArgs}
		bind:job
		bind:jobId
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="For loop">
		<div slot="header" class="grow">
			<input bind:value={mod.summary} placeholder={'Summary'} />
		</div>

		<Splitpanes horizontal class="!max-h-[calc(100%-48px)]">
			<Pane size={60} minSize={20} class="p-4">
				{#if !noEditor}
					<Alert
						type="info"
						title="For loops"
						tooltip="For loops"
						documentationLink="https://www.windmill.dev/docs/flows/flow_loops"
						class="mb-4"
						size="xs"
					>
						Add steps inside the loop and specify an iterator expression that defines the sequence
						over which your subsequent steps will iterate.
					</Alert>
				{/if}

				{#if mod.value.type === 'forloopflow'}
					<div class="flex flex-row gap-8 mt-2 mb-6">
						<div>
							<div class="mb-2 text-sm font-bold"
								>Skip failures <Tooltip
									documentationLink="https://www.windmill.dev/docs/flows/flow_loops"
									>If disabled, the flow will fail as soon as one of the iteration fail. Otherwise,
									the error will be collected as the result of the iteration. Regardless of this
									setting, if an error handler is defined, it will process the error.</Tooltip
								></div
							>
							<Toggle
								bind:checked={mod.value.skip_failures}
								options={{
									right: 'Skip failures'
								}}
							/>
						</div>
						<div>
							<div class="mb-2 text-sm font-bold">Run in parallel</div>
							<Toggle
								bind:checked={mod.value.parallel}
								options={{
									right: 'All iterations run in parallel'
								}}
							/>
						</div>
						<div>
							<div class="mb-2 text-sm font-bold"
								>Parallelism <Tooltip
									>Assign a maximum number of branches run in parallel to control huge for-loops.</Tooltip
								>
							</div>
							<input
								type="number"
								disabled={!mod.value.parallel}
								bind:value={mod.value.parallelism}
							/>
						</div>
					</div>

					<div class="my-2 flex flex-row gap-2 items-center">
						<div class="text-sm font-bold whitespace-nowrap">
							Iterator expression
							<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_loops">
								List to iterate over.
							</Tooltip>
						</div>
						{#if enableAi}
							<IteratorGen
								bind:this={iteratorGen}
								focused={iteratorFieldFocused}
								arg={mod.value.iterator}
								on:showExpr={(e) => {
									editor?.setSuggestion(e.detail)
								}}
								on:setExpr={(e) => {
									if (mod.value.type === 'forloopflow') {
										mod.value.iterator = {
											type: 'javascript',
											expr: e.detail
										}
									}
									editor?.setCode('')
									editor?.insertAtCursor(e.detail)
								}}
								pickableProperties={stepPropPicker.pickableProperties}
							/>
						{/if}
						<div class="flex w-full justify-end">
							<Button
								on:click={() => (previewOpen = true)}
								startIcon={{ icon: Play }}
								color="dark"
								size="sm">Test an iteration</Button
							>
						</div>
					</div>

					{#if mod.value.iterator.type == 'javascript'}
						<!-- svelte-ignore a11y-no-static-element-interactions -->
						<div
							class="border w-full"
							id="flow-editor-iterator-expression"
							on:keyup={iteratorGen?.onKeyUp}
						>
							<PropPickerWrapper
								notSelectable
								pickableProperties={stepPropPicker.pickableProperties}
								on:select={({ detail }) => {
									editor?.insertAtCursor(detail)
									editor?.focus()
								}}
								noPadding
							>
								<SimpleEditor
									bind:this={editor}
									on:focus={() => {
										iteratorFieldFocused = true
									}}
									on:blur={() => {
										iteratorFieldFocused = false
									}}
									autofocus
									lang="javascript"
									bind:code={mod.value.iterator.expr}
									class="small-editor"
									shouldBindKey={false}
									extraLib={stepPropPicker.extraLib}
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
				{/if}
			</Pane>
			<Pane size={40} minSize={20} class="flex flex-col flex-1">
				<Tabs bind:selected>
					<!-- <Tab value="retries">Retries</Tab> -->
					<Tab value="early-stop">Early Stop/Break</Tab>
					<Tab value="suspend">Suspend/Approval/Prompt</Tab>
					<Tab value="sleep">Sleep</Tab>
					<Tab value="mock">Mock</Tab>
					<Tab value="lifetime">Lifetime</Tab>

					<svelte:fragment slot="content">
						<div class="overflow-hidden bg-surface" style="height:calc(100% - 32px);">
							<!-- <TabContent value="retries" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowRetries bind:flowModule={mod} />
								</div>
							</TabContent> -->

							<TabContent value="early-stop" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleEarlyStop bind:flowModule={mod} />
								</div>
							</TabContent>

							<TabContent value="suspend" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule={mod} />
								</div>
							</TabContent>
							<TabContent value="sleep" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule={mod} />
								</div>
							</TabContent>
							<TabContent value="mock" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleMock bind:flowModule={mod} />
								</div>
							</TabContent>
							<TabContent value="lifetime" class="flex flex-col flex-1 h-full">
								<div class="p-4 overflow-y-auto">
									<FlowModuleDeleteAfterUse bind:flowModule={mod} disabled={!$enterpriseLicense} />
								</div>
							</TabContent>
						</div>
					</svelte:fragment>
				</Tabs>
			</Pane>
		</Splitpanes>
	</FlowCard>
</div>
