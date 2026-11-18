<script lang="ts">
	import { getContext, tick } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button, Drawer, Alert, Tab, Tabs } from '$lib/components/common'

	import { Play } from 'lucide-svelte'
	import type { FlowModule, Job, WhileloopFlow } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowRunSettings from './FlowRunSettings.svelte'
	import StepSettingsBadges from './StepSettingsBadges.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { SecondsInput } from '$lib/components/common'
	import { slideDynamic } from '$lib/transitions'

	const { flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		mod: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
	}

	let { mod = $bindable(), previousModule, parentModule, noEditor }: Props = $props()

	let runSettings: FlowRunSettings | undefined = $state(undefined)

	let previewOpen = $state(false)
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)

	let previewIterationArgs = $derived(flowStateStore.val[mod.id]?.previewArgs ?? {})

	let selectedTab = $state('loop')
	// Keeps "Limit iterations" open while its field is emptied mid-edit; the model holds no
	// limit meanwhile, and leaving the field empty turns the limit off.
	let iterationsFieldEmpty = $state(false)
	let isIterationsLimited = $derived(
		(mod.value.type === 'whileloopflow' && mod.value.max_iterations !== undefined) ||
			iterationsFieldEmpty
	)

	useUiIntent(`whileloopflow-${mod.id}`, {
		openTab: async (tab) => {
			// Every setting the intent can name lives in the other tab, which only mounts
			// `runSettings` once selected.
			selectedTab = 'settings'
			await tick()
			runSettings?.openSetting(tab)
		}
	})
</script>

<Drawer bind:open={previewOpen} alwaysOpen size="75%">
	<FlowLoopIterationPreview
		modules={mod.value.type == 'whileloopflow' ? mod.value.modules : []}
		open={previewOpen}
		previewArgs={previewIterationArgs}
		whileLoop
		bind:job
		bind:jobId
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>

<FlowCard {noEditor} title="While loop">
	{#snippet header()}
		<div class="grow">
			<div class="flex flex-row gap-2 items-center">
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
		{#if mod.value.type === 'whileloopflow'}
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
					{#if !noEditor}
						<Alert
							type="info"
							title="While loops"
							size="xs"
							documentationLink="https://www.windmill.dev/docs/flows/while_loops"
						>
							Add steps inside the while loop but have one of them use early stop/break in their Run
							settings (or do it at the loop level that will watch the last step) to break out of
							the while loop. Without a break or a limit below, it loops until you cancel the flow.
						</Alert>
					{/if}

					<section class="flex flex-col gap-6">
						<FlowModuleEarlyStop blocks="stop-after" bind:flowModule={mod} />

						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								textClass="text-xs font-normal text-primary"
								checked={isIterationsLimited}
								on:change={({ detail }) => {
									iterationsFieldEmpty = false
									;(mod.value as WhileloopFlow).max_iterations = detail ? 10 : undefined
								}}
								options={{
									right: 'Limit iterations',
									rightTooltip:
										'Checked after each iteration. When the limit is reached and the break condition did not stop the loop, the loop fails with an error named LoopExhausted, handled like any step failure (retries, error handler, or Continue on error then a branch on the error).'
								}}
							/>
							{#if isIterationsLimited}
								<div class="flex flex-col gap-2 pl-9" transition:slideDynamic>
									<Label label="Maximum iterations">
										<TextInput
											class="w-24"
											bind:value={
												() => (mod.value as WhileloopFlow).max_iterations,
												(v) => {
													const n = Math.floor(Number(v))
													iterationsFieldEmpty = !(n >= 1)
													;(mod.value as WhileloopFlow).max_iterations = n >= 1 ? n : undefined
												}
											}
											inputProps={{
												type: 'number',
												min: 1,
												onblur: () => (iterationsFieldEmpty = false)
											}}
										/>
									</Label>
								</div>
							{/if}
						</div>

						<div class="flex flex-col gap-2">
							<Toggle
								size="xs"
								textClass="text-xs font-normal text-primary"
								checked={mod.value.max_duration_secs !== undefined}
								on:change={({ detail }) => {
									;(mod.value as WhileloopFlow).max_duration_secs = detail ? 600 : undefined
								}}
								options={{
									right: 'Limit duration',
									rightTooltip:
										'Checked after each iteration, so the loop can run up to one iteration past the limit. Set a timeout on the steps inside to cap an iteration. When the limit is reached before the break condition, the loop fails with LoopExhausted, handled like any step failure.'
								}}
							/>
							{#if mod.value.max_duration_secs !== undefined}
								<div class="flex flex-col gap-2 pl-9" transition:slideDynamic>
									<Label label="Maximum duration">
										<SecondsInput
											bind:seconds={
												() => (mod.value as WhileloopFlow).max_duration_secs,
												(v) =>
													((mod.value as WhileloopFlow).max_duration_secs = Math.max(1, v ?? 1))
											}
										/>
									</Label>
								</div>
							{/if}
						</div>

						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							bind:checked={mod.value.skip_failures}
							options={{
								right: 'Skip failures',
								rightTooltip:
									'If disabled, the flow will fail as soon as one of the iteration fail. Otherwise, the error will be collected as the result of the iteration. Regardless of this setting, if a flow level error handler is defined, it will process the error. (Workspace error handlers will NOT be used to process errors if enabled.)',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/while_loops'
							}}
						/>
						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							bind:checked={mod.value.squash}
							on:change={({ detail }) => {
								;(mod.value as WhileloopFlow).squash = detail
							}}
							options={{
								right: 'Squash',
								rightTooltip:
									'Squashing a while loop runs all iterations on the same worker, using a single runner per step for the entire loop. This eliminates cold starts between iterations for supported languages (Bun, Deno, and Python).',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/while_loops'
							}}
						/>
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
