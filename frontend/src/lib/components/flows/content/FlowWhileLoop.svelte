<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	// import FlowRetries from './FlowRetries.svelte'
	import { Button, Drawer, Tab, TabContent, Alert } from '$lib/components/common'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { enterpriseLicense } from '$lib/stores'

	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleMock from './FlowModuleMock.svelte'
	import { Play } from 'lucide-svelte'
	import type { FlowModule, Job, WhileloopFlow } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import TabsV2 from '$lib/components/common/tabs/TabsV2.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	const { flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		mod: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
	}

	let { mod = $bindable(), previousModule, parentModule, noEditor }: Props = $props()

	let selected: string = $state('early-stop')

	let previewOpen = $state(false)
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)

	let previewIterationArgs = $derived(flowStateStore.val[mod.id]?.previewArgs ?? {})

	useUiIntent(`whileloopflow-${mod.id}`, {
		openTab: (tab) => {
			selected = tab
		}
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

<FlowCard {noEditor} title="While loop">
	{#snippet header()}
		<div class="grow">
			<input bind:value={mod.summary} placeholder={'Summary'} />
		</div>
	{/snippet}

	<Splitpanes horizontal class="h-full">
		<Pane size={50} minSize={20} class="p-4">
			{#if !noEditor}
				<Alert
					type="info"
					title="While loops"
					class="mb-4"
					size="xs"
					documentationLink="https://www.windmill.dev/docs/flows/while_loops"
				>
					Add steps inside the while loop but have one of them use early stop/break in their
					Advanced settings (or do it at the loop level that will watch the last step) to break out
					of the while loop (otherwise it will loop forever and you will have to cancel the flow
					manually).
				</Alert>
			{/if}

			{#if mod.value.type === 'whileloopflow'}
				<div class="flex flex-row gap-8 mt-2 mb-6">
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold"
							>Skip failures <Tooltip
								documentationLink="https://www.windmill.dev/docs/flows/while_loops"
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
						/>
					</div>
					<div class="flex-shrink-0">
						<div class="mb-2 text-sm font-bold"
							>Squash

							<Badge
								>Beta <Tooltip documentationLink="https://www.windmill.dev/docs/flows/while_loops">
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
								;(mod.value as WhileloopFlow).squash = detail
							}}
							options={{
								right: 'Squash'
							}}
							class="whitespace-nowrap"
						/>
					</div>
				</div>

				<div class="my-2 flex flex-row gap-2 items-center">
					<div class="flex w-full justify-end">
						<Button
							on:click={() => (previewOpen = true)}
							startIcon={{ icon: Play }}
							variant="accent"
							size="sm">Test an iteration</Button
						>
					</div>
				</div>
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
