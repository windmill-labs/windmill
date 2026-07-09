<script lang="ts">
	import { Alert, Badge } from '$lib/components/common'

	import type { BranchOne, FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import BranchPredicateEditor from './BranchPredicateEditor.svelte'
	import FlowModuleAdvancedSettings from './FlowModuleAdvancedSettings.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'

	interface Props {
		flowModule: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
		enableAi?: boolean
	}

	let {
		flowModule = $bindable(),
		previousModule,
		parentModule,
		noEditor,
		enableAi = false
	}: Props = $props()

	let value = $state(flowModule.value as BranchOne)
	$effect(() => {
		value = flowModule.value as BranchOne
	})

	let advancedSettings: FlowModuleAdvancedSettings | undefined = $state(undefined)

	// UI Intent handling for AI tool control: forward the requested tab to the
	// matching Run-settings accordion row (keys match the old tab names).
	useUiIntent(`branchone-${flowModule.id}`, {
		openTab: (tab) => {
			advancedSettings?.openSetting(tab)
		}
	})
</script>

<div class="h-full" id="flow-editor-branch-one-wrapper">
	<FlowCard {noEditor} title="Run one branch">
		<div class="flex h-full min-h-0 flex-col gap-6 overflow-auto p-4">
			{#if !noEditor}
				<Alert
					type="info"
					title="Only first branch whose condition is true will be run"
					tooltip="Branch one"
					documentationLink="https://www.windmill.dev/docs/flows/flow_branches#branch-one"
				>
					The result of this step is the result of the branch.
				</Alert>
			{/if}

			<section>
				<div class="mb-2 text-[11px] font-medium uppercase tracking-[0.04em] text-tertiary">
					{value.branches.length + 1} branch{value.branches.length + 1 > 1 ? 'es' : ''}
				</div>
				<div class="flex flex-col gap-2">
					{#each value.branches as branch, i}
						<div class="flex flex-col gap-3 rounded-md border bg-surface-tertiary p-3">
							<div class="flex items-center gap-2">
								<Badge color="blue" class="text-xs">Branch {i + 1}</Badge>
								<input
									class="w-full"
									type="text"
									bind:value={branch.summary}
									placeholder="Summary"
								/>
							</div>
							<BranchPredicateEditor
								{branch}
								on:updateSummary={(e) => {
									if (!branch.summary) {
										branch.summary = e.detail
									}
								}}
								parentModule={flowModule}
								{previousModule}
								{enableAi}
							/>
						</div>
					{/each}
					<div class="flex items-center gap-2 rounded-md border bg-surface-tertiary p-3">
						<Badge color="blue" class="text-xs">Default</Badge>
						<p class="text-xs italic text-tertiary"
							>If none of the predicates match, this branch is chosen</p
						>
					</div>
				</div>
				<p class="mt-2 text-xs text-tertiary">Add branches and steps directly on the graph.</p>
			</section>

			<section>
				<FlowModuleAdvancedSettings
					embedded
					loopSubset
					bind:this={advancedSettings}
					bind:flowModule
					{parentModule}
					{previousModule}
					selectedId={flowModule.id}
				/>
			</section>
		</div>
	</FlowCard>
</div>
