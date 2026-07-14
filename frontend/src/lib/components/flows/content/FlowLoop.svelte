<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import { Button, Drawer } from '$lib/components/common'
	import { getStepPropPicker } from '../previousResults'

	import { Play, FunctionSquare } from 'lucide-svelte'
	import type { FlowModule, ForloopFlow, Job } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import IteratorGen from '$lib/components/copilot/IteratorGen.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'

	import FlowExpressionEditor from './FlowExpressionEditor.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import { emptySchema } from '$lib/utils'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

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
	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state(undefined)
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

	// UI Intent handling for AI tool control: forward the requested tab to the
	// matching Run-settings accordion row (keys match the old tab names).
	useUiIntent(`forloopflow-${mod.id}`, {
		openTab: (tab) => {
			advancedSettings?.openSetting(tab)
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
						variant="accent"
						size="sm">Test an iteration</Button
					>
				</div>
			</div>
		</div>
	{/snippet}

	<div class="flex h-full min-h-0 flex-col gap-6 overflow-auto p-4">
		{#if mod.value.type === 'forloopflow'}
			<section>
				{#if mod.value.iterator.type == 'javascript'}
					<FlowExpressionEditor
						bind:code={
							() => {
								const it = (mod.value as ForloopFlow).iterator
								return it.type === 'javascript' ? it.expr : ''
							},
							(v) => {
								;(mod.value as ForloopFlow).iterator = { type: 'javascript', expr: v }
							}
						}
						label="Iterator expression"
						documentationLink="https://www.windmill.dev/docs/flows/flow_loops"
						pickableProperties={stepPropPicker.pickableProperties}
						extraLib={stepPropPicker.extraLib}
						id="flow-editor-iterator-expression"
						bind:focused={iteratorFieldFocused}
						bind:editor
						{suggestion}
						onKeyUp={iteratorGen?.onKeyUp}
					>
						{#snippet tooltip()}
							The JavaScript expression that will be evaluated to get the list of items to iterate
							over. Example : ["banana", "apple", flow_input.my_fruit].
						{/snippet}
						{#snippet headerExtra()}
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
					</FlowExpressionEditor>
				{:else}
					<Button
						on:click={() => {
							if (mod.value.type === 'forloopflow') mod.value.iterator.type = 'javascript'
						}}
					/>
				{/if}
			</section>

			<section class="flex flex-col gap-4">
				<div class="flex flex-row flex-wrap gap-6">
					<div class="flex-shrink-0">
						<div class="mb-2 text-xs font-semibold text-emphasis"
							>Skip failures <Tooltip
								documentationLink="https://www.windmill.dev/docs/flows/flow_loops"
								>If disabled, the flow will fail as soon as one of the iteration fail. Otherwise,
								the error will be collected as the result of the iteration. Regardless of this
								setting, if a flow level error handler is defined, it will process the error.
								(Workspace error handlers will NOT be used to process errors if enabled.)</Tooltip
							></div
						>
						<Toggle bind:checked={mod.value.skip_failures} />
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-xs font-semibold text-emphasis"
							>Squash
							<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_loops">
								Squashing a for loop runs all iterations on the same worker, using a single runner
								per step for the entire loop. This eliminates cold starts between iterations for
								supported languages (Bun, Deno, and Python).
							</Tooltip>
						</div>
						<Toggle
							bind:checked={mod.value.squash}
							on:change={({ detail }) => {
								;(mod.value as ForloopFlow).squash = detail
							}}
							disabled={mod.value.parallel}
						/>
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-xs font-semibold text-emphasis">Run in parallel</div>
						<Toggle
							bind:checked={mod.value.parallel}
							on:change={({ detail }) => {
								if (detail === false) {
									;(mod.value as ForloopFlow).parallelism = undefined
								}
							}}
							disabled={mod.value.squash}
						/>
					</div>
					<div class="flex-shrink-0" class:opacity-50={!mod.value.parallel}>
						<div class="mb-2 text-xs font-semibold text-emphasis"
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
										if (value === '' || value === null || value === undefined) {
											;(mod.value as ForloopFlow).parallelism = undefined
										} else {
											;(mod.value as ForloopFlow).parallelism = {
												type: 'static',
												value
											}
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
					<FlowExpressionEditor
						bind:code={
							() => {
								const p = (mod.value as ForloopFlow).parallelism
								return p && p.type === 'javascript' ? p.expr : ''
							},
							(v) => {
								;(mod.value as ForloopFlow).parallelism = { type: 'javascript', expr: v }
							}
						}
						label="Parallelism expression"
						pickableProperties={stepPropPicker.pickableProperties}
						flow_input={stepPropPicker.pickableProperties.flow_input}
						extraLib={stepPropPicker.extraLib}
						bind:editor={parallelismEditor}
						id="flow-editor-parallel-expression"
					>
						{#snippet tooltip()}
							JavaScript expression that defines the maximum number of parallel executions. Example:
							flow_input.max_parallel || 3
						{/snippet}
					</FlowExpressionEditor>
				{/if}
			</section>

			<section>
				<FlowModuleAdvancedSettings
					embedded
					loopSubset
					bind:this={advancedSettings}
					bind:flowModule={mod}
					{parentModule}
					{previousModule}
					selectedId={mod.id}
				/>
			</section>
		{/if}
	</div>
</FlowCard>
