<script lang="ts">
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Button, Drawer, Alert } from '$lib/components/common'

	import { Play } from 'lucide-svelte'
	import type { FlowModule, Job, WhileloopFlow } from '$lib/gen'
	import FlowLoopIterationPreview from '$lib/components/FlowLoopIterationPreview.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'

	const { flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		mod: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
	}

	let { mod = $bindable(), previousModule, parentModule, noEditor }: Props = $props()

	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state(undefined)

	let previewOpen = $state(false)
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)

	let previewIterationArgs = $derived(flowStateStore.val[mod.id]?.previewArgs ?? {})

	// UI Intent handling for AI tool control: forward the requested tab to the
	// matching Run-settings accordion row (keys match the old tab names).
	useUiIntent(`whileloopflow-${mod.id}`, {
		openTab: (tab) => {
			advancedSettings?.openSetting(tab)
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
			<div class="flex flex-row gap-2 items-center">
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
		{#if !noEditor}
			<Alert
				type="info"
				title="While loops"
				size="xs"
				documentationLink="https://www.windmill.dev/docs/flows/while_loops"
			>
				Add steps inside the while loop but have one of them use early stop/break in their Advanced
				settings (or do it at the loop level that will watch the last step) to break out of the
				while loop (otherwise it will loop forever and you will have to cancel the flow manually).
			</Alert>
		{/if}

		{#if mod.value.type === 'whileloopflow'}
			<section class="flex flex-col gap-4">
				<div class="flex flex-row flex-wrap gap-6">
					<div class="flex-shrink-0">
						<div class="mb-2 text-xs font-semibold text-emphasis"
							>Skip failures <Tooltip
								documentationLink="https://www.windmill.dev/docs/flows/while_loops"
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
							<Tooltip documentationLink="https://www.windmill.dev/docs/flows/while_loops">
								Squashing a while loop runs all iterations on the same worker, using a single runner
								per step for the entire loop. This eliminates cold starts between iterations for
								supported languages (Bun, Deno, and Python).
							</Tooltip>
						</div>
						<Toggle
							bind:checked={mod.value.squash}
							on:change={({ detail }) => {
								;(mod.value as WhileloopFlow).squash = detail
							}}
						/>
					</div>
				</div>
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
