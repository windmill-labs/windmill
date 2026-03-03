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
	import { Play, FunctionSquare } from 'lucide-svelte'
	import type { FlowModule, ForloopFlow, Job } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import IteratorGen from '$lib/components/copilot/IteratorGen.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import FlowPlugConnect from '$lib/components/FlowPlugConnect.svelte'

	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import TabsV2 from '$lib/components/common/tabs/TabsV2.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import { emptySchema } from '$lib/utils'
	import { slide } from 'svelte/transition'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

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
	let parallelismEditor: SimpleEditor | undefined = $state(undefined)
	let selected: string = $state('early-stop')
	let parallelismType: 'static' | 'javascript' | undefined = $state(
		mod.value.type === 'forloopflow'
			? mod.value.parallelism?.type === 'javascript'
				? 'javascript'
				: 'static'
			: undefined
	)

	let parallelismSchema = $state(emptySchema())
	parallelismSchema.properties['parallelism'] = {
		type: 'number'
	}

	if (mod.value.type === 'forloopflow') {
		const forloopValue = mod.value as ForloopFlow
		if (typeof forloopValue.parallelism === 'number') {
			forloopValue.parallelism = {
				type: 'static',
				value: forloopValue.parallelism
			}
		}
	}

	// UI Intent handling for AI tool control
	useUiIntent(`forloopflow-${mod.id}`, {
		openTab: (tab) => {
			selected = tab
		}
	})

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')
	flowPropPickerConfig.set(undefined)

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			parentModule,
			previousModule,
			mod.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	let previewOpen = $state(false)
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)

	let iteratorFieldFocused = $state(false)
	let iteratorGen: IteratorGen | undefined = $state(undefined)

	let previewIterationArgs = $derived(flowStateStore.val[mod.id]?.previewArgs ?? {})

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
		editor && (currentEditor as any).set({ type: 'iterator', editor, stepId: mod.id })
	})

	let suggestion: string | undefined = $state(undefined)
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
						variant="accent"
						size="sm">Test an iteration</Button
					>
				</div>
			</div>
		</div>
	{/snippet}

	<Splitpanes horizontal class="h-full">
		<Pane size={50} minSize={20} class="p-4">
			{#if mod.value.type === 'forloopflow'}
				<div class="flex flex-row gap-6 mt-2 mb-6">
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold"
							>Skip failures <Tooltip
								documentationLink="https://www.windmill.dev/docs/flows/flow_loops"
								>If disabled, the flow will fail as soon as one of the iteration fail. Otherwise,
								the error will be collected as the result of the iteration. Regardless of this
								setting, if a flow level error handler is defined, it will process the error.
								(Workspace error handlers will NOT be used to process errors if enabled.)</Tooltip
							></div
						>
						<Toggle
							bind:checked={mod.value.skip_failures}
							options={{
								right: 'Skip failures'
							}}
							class="whitespace-nowrap"
						/>
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold"
							>Squash

							<Badge
								>Beta <Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_loops">
									<span class="font-semibold"
										>This can result in unexpected behavior, use at your own risk for now.</span
									><br />
									Squashing a for loop runs all iterations on the same worker, using a single runner
									per step for the entire loop. This eliminates cold starts between iterations for supported
									languages (Bun, Deno, and Python).
								</Tooltip>
							</Badge>
						</div>
						<Toggle
							bind:checked={mod.value.squash}
							on:change={({ detail }) => {
								;(mod.value as ForloopFlow).squash = detail
							}}
							options={{
								right: 'Squash'
							}}
							class="whitespace-nowrap"
							disabled={mod.value.parallel}
						/>
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold">Run in parallel</div>
						<Toggle
							bind:checked={mod.value.parallel}
							on:change={({ detail }) => {
								if (detail === false) {
									;(mod.value as ForloopFlow).parallelism = undefined
								}
							}}
							options={{
								right: 'All iterations run in parallel'
							}}
							class="whitespace-nowrap"
							disabled={mod.value.squash}
						/>
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold"
							>Parallelism <Tooltip
								>Assign a maximum number of branches run in parallel to control huge for-loops.</Tooltip
							>
						</div>
						<div class="flex gap-2 items-center">
							<input
								type="number"
								min="1"
								class="w-20 px-2 py-1 text-sm border border-gray-200 dark:border-gray-700 rounded bg-surface"
								disabled={!mod.value.parallel || parallelismType === 'javascript'}
								placeholder={parallelismType === 'javascript' ? 'Expression' : ''}
								bind:value={
									() => {
										const parallelismExpr = (mod.value as ForloopFlow).parallelism

										return parallelismExpr && parallelismExpr.type === 'static'
											? parallelismExpr.value
											: ''
									},
									(value) => {
										;(mod.value as ForloopFlow).parallelism = {
											type: 'static',
											value
										}
									}
								}
							/>
							<ToggleButtonGroup
								disabled={!mod.value.parallel}
								bind:selected={parallelismType}
								on:selected={(e) => {
									const forLoopFlow = mod.value as ForloopFlow
									if (e.detail == parallelismType) return
									if (e.detail === 'javascript') {
										if (!forLoopFlow.parallelism || forLoopFlow.parallelism.type !== 'javascript') {
											;(mod.value as ForloopFlow).parallelism = {
												type: 'javascript',
												expr: ''
											}
										}
									} else {
										if (!forLoopFlow.parallelism || forLoopFlow.parallelism.type !== 'static') {
											;(mod.value as ForloopFlow).parallelism = {
												type: 'static',
												value: 0
											}
										}
									}
								}}
								class="h-6"
							>
								{#snippet children({ item })}
									<ToggleButton small label="static" value="static" {item} />

									<ToggleButton
										small
										tooltip="JavaScript expression ('flow_input' or 'results')."
										value="javascript"
										icon={FunctionSquare}
										{item}
									/>
								{/snippet}
							</ToggleButtonGroup>
						</div>
					</div>
				</div>

				{#if mod.value.type === 'forloopflow' && mod.value.parallel && mod.value.parallelism?.type == 'javascript'}
					<div class="my-2 flex flex-row gap-2 items-center">
						<div class="text-sm font-bold whitespace-nowrap">
							Parallelism expression
							<Tooltip>
								JavaScript expression that defines the maximum number of parallel executions.
								Example: flow_input.max_parallel || 3
							</Tooltip>
						</div>
					</div>
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="border rounded-md overflow-auto w-full mb-2 h-full max-h-[250px]"
						id="flow-editor-parallel-expression"
						transition:slide={{ duration: 300 }}
					>
						<PropPickerWrapper
							notSelectable
							noPadding
							flow_input={stepPropPicker.pickableProperties.flow_input}
							pickableProperties={stepPropPicker.pickableProperties}
							on:select={({ detail }) => {
								parallelismEditor?.insertAtCursor(detail)
								parallelismEditor?.focus()
							}}
						>
							<SimpleEditor
								bind:this={parallelismEditor}
								autofocus
								lang="javascript"
								bind:code={
									() => {
										const parallelismExpr = (mod.value as ForloopFlow).parallelism

										return parallelismExpr && parallelismExpr.type === 'javascript'
											? parallelismExpr.expr
											: ''
									},
									(expr) => {
										;(mod.value as ForloopFlow).parallelism = {
											type: 'javascript',
											expr
										}
									}
								}
								class="h-full"
								shouldBindKey={false}
								extraLib={stepPropPicker.extraLib}
							/>
						</PropPickerWrapper>
					</div>
				{/if}
				<div class="my-2 flex flex-row gap-2 items-center">
					<div class="text-sm font-bold whitespace-nowrap">
						Iterator expression
						<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_loops">
							The JavaScript expression that will be evaluated to get the list of items to iterate
							over. Example : ["banana", "apple", flow_input.my_fruit].
						</Tooltip>
					</div>
					<FlowPlugConnect
						connecting={$flowPropPickerConfig != undefined}
						on:click={() => {
							const config = {
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
							on:showExpr={(e) => (suggestion = e.detail || undefined)}
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
						class="border rounded-md overflow-auto w-full"
						id="flow-editor-iterator-expression"
						onkeyup={iteratorGen?.onKeyUp}
					>
						<PropPickerWrapper
							notSelectable
							pickableProperties={stepPropPicker.pickableProperties}
							on:select={({ detail }) => {
								if ($flowPropPickerConfig) {
									setExpr(detail)
									flowPropPickerConfig.set(undefined)
									return
								}
								editor?.insertAtCursor(detail)
								editor?.focus()
							}}
							noPadding
						>
							<div class="relative w-full h-full overflow-clip">
								<SimpleEditor
									small
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
									class="h-full"
									shouldBindKey={false}
									extraLib={stepPropPicker.extraLib}
									{suggestion}
								/>
							</div>
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
				<Tab value="early-stop" label="Early Stop/Break" />
				<Tab value="skip" label="Skip" />
				<Tab value="suspend" label="Suspend/Approval/Prompt" />
				<Tab value="sleep" label="Sleep" />
				<Tab value="mock" label="Mock" />
				<Tab value="lifetime" label="Lifetime" />

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
