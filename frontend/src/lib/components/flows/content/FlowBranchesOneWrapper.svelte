<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import StepSettingsBadges from './StepSettingsBadges.svelte'
	import { tick } from 'svelte'

	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import ChoiceBranchList from './ChoiceBranchList.svelte'
	import FlowRunSettings from './FlowRunSettings.svelte'
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

	let runSettings: FlowRunSettings | undefined = $state(undefined)
	let selectedTab = $state('branches')

	useUiIntent(`branchone-${flowModule.id}`, {
		openTab: async (tab) => {
			// Every setting the intent can name lives in the other tab, which only mounts
			// `runSettings` once selected.
			selectedTab = 'settings'
			await tick()
			runSettings?.openSetting(tab)
		}
	})
</script>

<div class="h-full" id="flow-editor-branch-one-wrapper">
	<FlowCard
		{noEditor}
		title="Run one branch"
		subtitle="The first branch whose predicate is true runs. The result of this step is that branch's result."
		subtitleDocLink="https://www.windmill.dev/docs/flows/flow_branches#branch-one"
	>
		<div class="flex h-full min-h-0 flex-col">
			<Tabs bind:selected={selectedTab} wrapperClass="shrink-0">
				<Tab value="branches" label="Branches" />
				<Tab value="settings" label="Run settings">
					{#snippet extra()}
						<StepSettingsBadges {flowModule} />
					{/snippet}
				</Tab>
			</Tabs>

			<div
				class="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-4"
				style="scrollbar-gutter: stable"
			>
				{#if selectedTab === 'branches'}
					<ChoiceBranchList {flowModule} {previousModule} {enableAi} />
				{:else}
					<FlowRunSettings
						embedded
						loopSubset
						bind:this={runSettings}
						bind:flowModule
						{parentModule}
						{previousModule}
						selectedId={flowModule.id}
					/>
				{/if}
			</div>
		</div>
	</FlowCard>
</div>
