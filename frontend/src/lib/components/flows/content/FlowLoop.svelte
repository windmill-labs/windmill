<script lang="ts">
	import { getContext, tick } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import { Button, Drawer, Tab, Tabs } from '$lib/components/common'
	import { getStepPropPicker } from '../previousResults'

	import { Play } from 'lucide-svelte'
	import type { FlowModule, ForloopFlow, Job } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import IteratorGen from '$lib/components/copilot/IteratorGen.svelte'
	import FlowRunSettings from './FlowRunSettings.svelte'
	import StepSettingsBadges from './StepSettingsBadges.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'

	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import { emptySchema, emptyString } from '$lib/utils'
	import { slideDynamic } from '$lib/transitions'

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
	let runSettings: FlowRunSettings | undefined = $state(undefined)

	let parallelismSchema = $state(emptySchema())
	parallelismSchema.properties['parallelism'] = {
		type: 'integer'
	}

	// `array` keeps the field on the expression editor: an iterator is never a literal
	// the static input could hold, which is also why the type switch is hidden.
	let iteratorSchema = $state(emptySchema())
	iteratorSchema.properties['iterator'] = {
		type: 'array'
	}
	iteratorSchema.required = ['iterator']

	if (mod.value.type === 'forloopflow') {
		const forloopValue = mod.value as ForloopFlow
		if (typeof forloopValue.parallelism === 'number') {
			forloopValue.parallelism = {
				type: 'static',
				value: forloopValue.parallelism
			}
		}
	}

	let selectedTab = $state('loop')

	useUiIntent(`forloopflow-${mod.id}`, {
		openTab: async (tab) => {
			// Every setting the intent can name lives in the other tab, which only mounts
			// `runSettings` once selected.
			selectedTab = 'settings'
			await tick()
			runSettings?.openSetting(tab)
		}
	})

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const { flowPropPickerConfig } = propPickerContext
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

	// A loop with nothing to iterate over fails at runtime, and the step's own inputs
	// aren't schema-checked like a script's — so the field says it here.
	const iterator = $derived(
		mod.value.type === 'forloopflow' ? (mod.value as ForloopFlow).iterator : undefined
	)
	const iteratorMissing = $derived.by(() => {
		if (iterator == undefined) return true
		if (iterator.type === 'javascript') return emptyString(iterator.expr)
		if (iterator.type === 'static') return iterator.value == undefined
		return false
	})

	const ITERATOR_LABEL = 'Iterator expression'
	const ITERATOR_MISSING = 'An iterator expression is required for the loop to run.'
	const ITERATOR_TOOLTIP =
		'The JavaScript expression that will be evaluated to get the list of items to iterate over. Example: ["banana", "apple", flow_input.my_fruit].'
	const DEFAULT_PARALLELISM = 4
	const PARALLELISM_LABEL = 'Limit concurrent iterations'
	const SQUASH_PARALLEL_CONFLICT =
		'Squash and Run in parallel are mutually exclusive: squashing runs every iteration in sequence on a single worker. Turn the other one off to use this.'
	const PARALLELISM_TOOLTIP =
		'Cap how many iterations run at once, so a huge loop does not flood the workers. Without a cap every iteration starts at once.'

	const parallelismCapped = $derived(
		mod.value.type === 'forloopflow' && mod.value.parallelism != undefined
	)
</script>

{#snippet parallelismToggle()}
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={parallelismCapped}
		on:change={({ detail }) => {
			;(mod.value as ForloopFlow).parallelism = detail
				? { type: 'static', value: DEFAULT_PARALLELISM }
				: undefined
		}}
		options={{
			right: PARALLELISM_LABEL,
			rightTooltip: PARALLELISM_TOOLTIP,
			rightDocumentationLink: 'https://www.windmill.dev/docs/flows/flow_loops'
		}}
	/>
{/snippet}

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
			<div class="flex flex-row gap-2 items-center">
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
						variant="default"
						size="sm">Test an iteration</Button
					>
				</div>
			</div>
		</div>
	{/snippet}

	<div class="flex h-full min-h-0 flex-col">
		{#if mod.value.type === 'forloopflow'}
			<Tabs bind:selected={selectedTab} wrapperClass="shrink-0">
				<Tab value="loop" label="Loop" />
				<Tab value="settings" label="Run settings">
					{#snippet extra()}
						<StepSettingsBadges flowModule={mod} />
					{/snippet}
				</Tab>
			</Tabs>

			<div
				class="flex min-h-0 flex-1 flex-col gap-8 overflow-auto p-4"
				style="scrollbar-gutter: stable"
			>
				{#if selectedTab === 'loop'}
					<section>
						<PropPickerWrapper
							sidePane
							flow_input={stepPropPicker.pickableProperties.flow_input}
							notSelectable
							pickableProperties={stepPropPicker.pickableProperties}
							on:select={({ detail }) => {
								editor?.insertAtCursor(detail)
								editor?.focus()
							}}
						>
							<InputTransformForm
								bind:arg={
									() => (mod.value as ForloopFlow).iterator,
									(v) => {
										;(mod.value as ForloopFlow).iterator = v
									}
								}
								argName="iterator"
								label={ITERATOR_LABEL}
								headerTooltip={ITERATOR_TOOLTIP}
								error={iteratorMissing ? ITERATOR_MISSING : undefined}
								schema={iteratorSchema}
								noDynamicToggle
								extraLib={stepPropPicker.extraLib}
								pickableProperties={stepPropPicker.pickableProperties}
								previousModuleId={previousModule?.id}
								bind:suggestion
								bind:focused={iteratorFieldFocused}
								aiOnKeyUp={iteratorGen?.onKeyUp}
								bind:editor
							>
								{#snippet aiGen()}
									{#if enableAi}
										<IteratorGen
											bind:this={iteratorGen}
											focused={iteratorFieldFocused}
											arg={(mod.value as ForloopFlow).iterator}
											on:showExpr={(e) => (suggestion = e.detail || undefined)}
											on:setExpr={(e) => setExpr(e.detail)}
											pickableProperties={stepPropPicker.pickableProperties}
										/>
									{/if}
								{/snippet}
							</InputTransformForm>
						</PropPickerWrapper>
					</section>
					<section class="flex flex-col gap-6">
						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							bind:checked={mod.value.skip_failures}
							options={{
								right: 'Skip failures',
								rightTooltip:
									'If disabled, the flow will fail as soon as one of the iteration fail. Otherwise, the error will be collected as the result of the iteration. Regardless of this setting, if a flow level error handler is defined, it will process the error. (Workspace error handlers will NOT be used to process errors if enabled.)',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/flow_loops'
							}}
						/>
						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							bind:checked={mod.value.squash}
							on:change={({ detail }) => {
								;(mod.value as ForloopFlow).squash = detail
							}}
							disabled={mod.value.parallel}
							options={{
								title: mod.value.parallel ? SQUASH_PARALLEL_CONFLICT : undefined,
								right: 'Squash',
								rightTooltip:
									'Squashing a for loop runs all iterations on the same worker, using a single runner per step for the entire loop. This eliminates cold starts between iterations for supported languages (Bun, Deno, and Python).',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/flow_loops'
							}}
						/>
						<!-- Its own group: the setting's input belongs to the toggle above it, not
						     24px away like the next setting. -->
						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								textClass="text-xs font-normal text-primary"
								bind:checked={mod.value.parallel}
								on:change={({ detail }) => {
									// An absent `parallelism` means "no cap" to the worker, so switching
									// parallelism on must not seed one — the cap below is opted into.
									if (!detail) (mod.value as ForloopFlow).parallelism = undefined
								}}
								disabled={mod.value.squash}
								options={{
									title: mod.value.squash ? SQUASH_PARALLEL_CONFLICT : undefined,
									right: 'Run in parallel',
									rightTooltip: 'Run the iterations concurrently instead of one after the other.',
									rightDocumentationLink: 'https://www.windmill.dev/docs/flows/flow_loops'
								}}
							/>
							{#if mod.value.parallel}
								<div class="pl-9" transition:slideDynamic>
									<PropPickerWrapper
										sidePane
										flow_input={stepPropPicker.pickableProperties.flow_input}
										notSelectable
										pickableProperties={stepPropPicker.pickableProperties}
										on:select={({ detail }) => {
											parallelismEditor?.insertAtCursor(detail)
											parallelismEditor?.focus()
										}}
									>
										<!-- Keyed on the toggle: the form seeds its static/expression mode once, so re-enabling
										     the cap would hand a fresh static value to a form still in expression mode. -->
										{#key parallelismCapped}
											<InputTransformForm
												bind:arg={
													() => (mod.value as ForloopFlow).parallelism,
													(v) => {
														;(mod.value as ForloopFlow).parallelism = v
													}
												}
												argName="parallelism"
												collapsed={!parallelismCapped}
												header={parallelismToggle}
												schema={parallelismSchema}
												argExtra={{ min: 1, step: 1 }}
												animateAppear
												previousModuleId={previousModule?.id}
												bind:editor={parallelismEditor}
											/>
										{/key}
									</PropPickerWrapper>
								</div>
							{/if}
						</div>

						<FlowModuleEarlyStop blocks="stop-after" bind:flowModule={mod} />
					</section>
				{:else}
					<FlowRunSettings
						embedded
						loopSubset
						earlyStopBlocks="all-iters"
						bind:this={runSettings}
						bind:flowModule={mod}
						{parentModule}
						{previousModule}
						selectedId={mod.id}
					/>
				{/if}
			</div>
		{/if}
	</div>
</FlowCard>
