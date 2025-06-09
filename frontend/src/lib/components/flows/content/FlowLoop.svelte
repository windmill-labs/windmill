<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	// import FlowRetries from './FlowRetries.svelte'
	import { Button, Drawer, Tab, TabContent } from '$lib/components/common'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { getStepPropPicker } from '../previousResults'
	import { enterpriseLicense } from '$lib/stores'

	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleMock from './FlowModuleMock.svelte'
	import { Play } from 'lucide-svelte'
	import type { FlowModule, Job } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import IteratorGen from '$lib/components/copilot/IteratorGen.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import FlowPlugConnect from '$lib/components/FlowPlugConnect.svelte'

	import PropPickerWrapper, { CONNECT } from '../propPicker/PropPickerWrapper.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import TabsV2 from '$lib/components/common/tabs/TabsV2.svelte'

	const { previewArgs, flowStateStore, flowStore, currentEditor } =
		getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		mod: FlowModule
		parentModule: FlowModule | undefined
		previousModule: FlowModule | undefined
		noEditor: boolean
		enableAi?: boolean
	}

	let {
		mod = $bindable(),
		parentModule,
		previousModule,
		noEditor,
		enableAi = false
	}: Props = $props()

	let editor: SimpleEditor | undefined = $state(undefined)
	let selected: string = $state('early-stop')

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')
	flowPropPickerConfig.set(undefined)

	let stepPropPicker = $derived(
		getStepPropPicker(
			$flowStateStore,
			parentModule,
			previousModule,
			mod.id,
			flowStore.val,
			$previewArgs,
			false
		)
	)

	let previewOpen = $state(false)
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)

	let iteratorFieldFocused = $state(false)
	let iteratorGen: IteratorGen | undefined = $state(undefined)

	let previewIterationArgs = $derived($flowStateStore[mod.id]?.previewArgs ?? {})

	function setExpr(code: string) {
		if (mod.value.type === 'forloopflow') {
			mod.value.iterator = {
				type: 'javascript',
				expr: code
			}
		}
		editor?.setCode('')
		editor?.insertAtCursor(code)
	}

	$effect(() => {
		editor && currentEditor.set({ type: 'iterator', editor, stepId: mod.id })
	})
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

<FlowCard {noEditor} title="For loop">
	{#snippet header()}
		<div class="grow">
			<div class="my-2 flex flex-row gap-2 items-center">
				<div>
					<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_loops">
						Add steps inside the loop and specify an iterator expression that defines the sequence
						over which your subsequent steps will iterate.
					</Tooltip>
				</div>
				<div class="grow">
					<input bind:value={mod.summary} placeholder={'Summary'} />
				</div>
				<div class="justify-end">
					<Button
						on:click={() => (previewOpen = true)}
						startIcon={{ icon: Play }}
						color="dark"
						size="sm">Test an iteration</Button
					>
				</div>
			</div>
		</div>
	{/snippet}

	<Splitpanes horizontal class="h-full">
		<Pane size={50} minSize={20} class="p-4">
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
							The JavaScript expression that will be evaluated to get the list of items to iterate
							over. Example : ["banana", "apple", flow_input.my_fruit].
						</Tooltip>
					</div>
					<FlowPlugConnect
						connecting={$flowPropPickerConfig?.insertionMode == CONNECT}
						on:click={() => {
							const config = {
								insertionMode: CONNECT,
								onSelect: (code) => {
									setExpr(code)
									return true
								},
								clearFocus: () => {
									flowPropPickerConfig.set(undefined)
								}
							}
							flowPropPickerConfig.set({
								...config,
								clearFocus: () => {
									flowPropPickerConfig.set(undefined)
								}
							})
						}}
					/>
					{#if enableAi}
						<IteratorGen
							bind:this={iteratorGen}
							focused={iteratorFieldFocused}
							arg={mod.value.iterator}
							on:showExpr={(e) => {
								editor?.setSuggestion(e.detail)
							}}
							on:setExpr={(e) => {
								setExpr(e.detail)
							}}
							pickableProperties={stepPropPicker.pickableProperties}
						/>
					{/if}
				</div>

				{#if mod.value.iterator.type == 'javascript'}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="border w-full"
						id="flow-editor-iterator-expression"
						onkeyup={iteratorGen?.onKeyUp}
					>
						<PropPickerWrapper
							alwaysOn
							notSelectable
							pickableProperties={stepPropPicker.pickableProperties}
							on:select={({ detail }) => {
								if ($flowPropPickerConfig?.insertionMode == CONNECT) {
									setExpr(detail)
									flowPropPickerConfig.set(undefined)
									return
								}
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
			<TabsV2 bind:selected>
				<!-- <Tab value="retries">Retries</Tab> -->
				<Tab value="early-stop">Early Stop/Break</Tab>
				<Tab value="skip">Skip</Tab>
				<Tab value="suspend">Suspend/Approval/Prompt</Tab>
				<Tab value="sleep">Sleep</Tab>
				<Tab value="mock">Mock</Tab>
				<Tab value="lifetime">Lifetime</Tab>

				{#snippet content()}
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
						<TabContent value="skip" class="flex flex-col flex-1 h-full">
							<div class="p-4 overflow-y-auto">
								<FlowModuleSkip bind:flowModule={mod} {parentModule} {previousModule} />
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
				{/snippet}
			</TabsV2>
		</Pane>
	</Splitpanes>
</FlowCard>
