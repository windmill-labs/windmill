<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import { flowStore } from '$lib/components/flows/flowStore'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert } from '$lib/components/common'
	import { FlowGraph } from '$lib/components/graph'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import autosize from 'svelte-autosize'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	export let initialPath: string

	let topHeight = 0
</script>

<div class="h-full overflow-hidden">
	<FlowCard title="Settings">
		<div class="h-full flex-1">
			<Tabs bind:selected={$selectedId}>
				<Tab value="settings-metadata">Metadata</Tab>
				<Tab value="settings-schedule">Schedule</Tab>
				<Tab value="settings-same-worker">Shared Directory</Tab>
				<Tab value="settings-graph">Graph</Tab>

				<svelte:fragment slot="content">
					<TabContent value="settings-metadata" class="p-4">
						<Path bind:path={$flowStore.path} {initialPath} namePlaceholder="my_flow" kind="flow" />

						<label class="block my-4">
							<span class="text-gray-700 text-sm">Summary <Required required={false} /></span>
							<input
								type="text"
								bind:value={$flowStore.summary}
								placeholder="A short summary of the flow displayed when the flow is listed"
								id="flow-summary"
							/>
						</label>

						<label class="block my-4" for="inp">
							<span class="text-gray-700 text-sm">
								Description
								<Required required={false} />
								<textarea
									use:autosize
									type="text"
									class="text-sm"
									id="inp"
									bind:value={$flowStore.description}
									placeholder="A description to help users understand what this flow does and how to use it."
									rows="3"
								/>
							</span>
						</label>
					</TabContent>
					<TabContent value="settings-schedule" class="p-4">
						<Alert type="info" title="Primary Schedule">
							Flows can be triggered by any schedules, their webhooks or their UI but they only have
							only one primary schedules with which they share the same path. The primary schedule
							can be set here.
						</Alert>
						<div class="mt-4" />
						<FlowSchedules />
					</TabContent>

					<TabContent value="settings-same-worker" class="p-4 flex flex-col">
						<Alert type="info" title="Shared Directory">
							Steps will share a folder at `./shared` in which they can store heavier data and pass
							them to the next step. <br /><br />Beware that the `./shared` folder is not preserved
							across suspends and sleeps.
						</Alert>
						<span class="my-2 text-sm font-bold">Shared Directoryr</span>
						<Toggle
							bind:checked={$flowStore.value.same_worker}
							options={{
								right: 'Shared Directory on `./shared`'
							}}
						/>
					</TabContent>
					<TabContent value="settings-graph">
						<div
							bind:clientHeight={topHeight}
							class="max-w-full w-full overflow-hidden h-screen bg-gray-50"
						>
							{#if $flowStore.value.modules}
								<FlowGraph
									on:click={(e) => {
										if (e.detail.id) {
											$selectedId = e.detail.id
										}
									}}
									minHeight={topHeight}
									modules={$flowStore.value.modules}
									failureModule={$flowStore.value.failure_module}
									notSelectable
								/>
							{/if}
						</div>
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</div>
	</FlowCard>
</div>
