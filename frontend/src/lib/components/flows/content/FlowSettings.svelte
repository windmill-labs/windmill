<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import autosize from 'svelte-autosize'
	import Slider from '$lib/components/Slider.svelte'
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import { copyToClipboard } from '$lib/utils'
	import { Icon } from 'svelte-awesome'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let initialPath: string

	$: url = `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/f/${$flowStore?.path}`
</script>

<div class="h-full overflow-hidden">
	<FlowCard title="Settings">
		<div class="h-full flex-1">
			<Tabs bind:selected={$selectedId} id="flow-tutorial-3">
				<Tab value="settings-metadata">Metadata</Tab>
				<Tab value="settings-schedule">Schedule</Tab>
				<Tab value="settings-same-worker">Shared Directory</Tab>

				<svelte:fragment slot="content">
					<TabContent value="settings-metadata" class="p-4 h-full">
						<div class="overflow-auto h-full">
							<Path bind:path={$flowStore.path} {initialPath} namePlaceholder="flow" kind="flow" />

							<label class="block my-4">
								<span class="text-gray-700 text-sm">Summary <Required required={false} /></span>
								<input
									type="text"
									bind:value={$flowStore.summary}
									placeholder="Short summary to be displayed when listed"
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
										placeholder="What this flow does and how to use it."
										rows="3"
									/>
								</span>
							</label>
							<Slider text="How to trigger from external events?">
								<div class="text-sm text-gray-600 border p-4">
									There are 2 ways to trigger a flow based on external events:
									<ul class="pt-4">
										<li
											>1. Send a webhook after each event: <a
												on:click={(e) => {
													e.preventDefault()
													copyToClipboard(url)
												}}
												href={$page.url.protocol + '//' + url}
												class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
											>
												{url}
												<span class="text-gray-700 ml-2">
													<Icon data={faClipboard} />
												</span>
											</a>
										</li>
										<li class="mt-2">
											<div class="flex flex-col gap-2">
												<p>
													2. Use a trigger script and schedule this flow to run as frequently as
													needed and compare a state persisted in Windmill to the state of the
													external system. If a difference is detected, then the rest of the flow is
													triggered. Oftentimes, the second step of a flow is a for-loop that will
													iterate over every elements. When using a trigger, a default schedule will
													be created.
												</p>
												<div>
													<img
														class="shadow-lg border rounded"
														alt="static button"
														src="/trigger_button.png"
													/>
												</div>
											</div>
										</li></ul
									>
								</div>
							</Slider>
						</div>
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
						<span class="my-2 text-sm font-bold">Shared Directory</span>
						<Toggle
							bind:checked={$flowStore.value.same_worker}
							options={{
								right: 'Shared Directory on `./shared`'
							}}
						/>
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</div>
	</FlowCard>
</div>
