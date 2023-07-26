<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import { BROWSER } from 'esm-env'
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
	import { workerTags, workspaceStore } from '$lib/stores'
	import { copyToClipboard } from '$lib/utils'
	import { Icon } from 'svelte-awesome'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { WorkerService } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'

	const { selectedId, flowStore, initialPath } = getContext<FlowEditorContext>('FlowEditorContext')

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}

	let hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
	$: url = `${hostname}/api/w/${$workspaceStore}/jobs/run/f/${$flowStore?.path}`
	$: syncedUrl = `${hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${$flowStore?.path}`

	$: if ($selectedId == 'settings-worker-group') {
		$workerTags = undefined
		loadWorkerGroups()
	}

	let path: Path | undefined = undefined
	$: {
		if (initialPath == '' && $flowStore.summary?.length > 0) {
			path?.setName(
				$flowStore.summary
					.toLowerCase()
					.replace(/[^a-z0-9_]/g, '_')
					.replace(/-+/g, '_')
					.replace(/^-|-$/g, '')
			)
		}
	}
</script>

<div class="h-full overflow-hidden">
	<FlowCard title="Settings">
		<div class="h-full flex-1">
			<Tabs bind:selected={$selectedId}>
				<Tab value="settings-metadata">Metadata</Tab>
				<Tab value="settings-schedule">Schedule</Tab>
				<Tab value="settings-same-worker">Shared Directory</Tab>
				<Tab value="settings-worker-group">Worker Group</Tab>

				<svelte:fragment slot="content">
					<TabContent value="settings-metadata" class="p-4 h-full">
						<div class="overflow-auto h-full">
							<label class="block mb-10 mt-2">
								<span class="text-secondary text-sm font-bold"
									>Summary <Required required={false} /></span
								>
								<input
									type="text"
									bind:value={$flowStore.summary}
									placeholder="Short summary to be displayed when listed"
									id="flow-summary"
								/>
							</label>

							<span class="text-secondary text-sm font-bold"> Path </span>
							<Path
								bind:this={path}
								bind:path={$flowStore.path}
								{initialPath}
								namePlaceholder="flow"
								kind="flow"
							/>

							<label class="block mt-10 mb-6" for="inp">
								<span class="text-secondary text-sm font-bold">
									Description
									<Required required={false} />
								</span>

								<textarea
									use:autosize
									type="text"
									class="text-sm"
									id="inp"
									bind:value={$flowStore.description}
									placeholder="What this flow does and how to use it."
									rows="3"
								/>
							</label>
							<Slider text="How to trigger flows?">
								<div class="text-sm text-tertiary border p-4 mb-20">
									On-demand:
									<ul class="pt-4">
										<li>
											1. <a
												href="https://www.windmill.dev/docs/core_concepts/auto_generated_uis"
												target="_blank">Auto-generated UIs</a
											>
										</li>
										<li>
											2. <a href="/apps/add?nodraft=true" target="_blank"> App Editor</a> for customized-UIs
										</li>
										<li>
											3. <a href="/schedules" target="_blank">Scheduling</a>
										</li>
										<li>
											4. <a href="https://www.windmill.dev/docs/advanced/cli" target="_blank"
												>Windmill CLI</a
											>
										</li>
										<br />
										<li class="mt-2">
											<div class="flex flex-col gap-2">
												<p> From external events: </p>
											</div>
										</li>
										<li class="mt-2">
											5. Send a <a
												href="https://www.windmill.dev/docs/core_concepts/webhooks"
												target="_blank">webhook</a
											>
											after each event:
											<ul class="list-disc pl-4"
												><li
													>Async <Tooltip
														>Return an uuid instantly that you can use to fetch status and result</Tooltip
													>:
													<a
														on:click={(e) => {
															e.preventDefault()
															copyToClipboard(url)
														}}
														href={url}
														class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
													>
														{url}
														<span class="text-secondary ml-2">
															<Icon data={faClipboard} />
														</span>
													</a>
												</li>
												<li
													>Sync <Tooltip>Wait for result within a timeout of 20s</Tooltip>:
													<a
														on:click={(e) => {
															e.preventDefault()
															copyToClipboard(syncedUrl)
														}}
														href={syncedUrl}
														class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
													>
														{syncedUrl}
														<span class="text-secondary ml-2">
															<Icon data={faClipboard} />
														</span>
													</a>
												</li>
											</ul></li
										>
										<br />
										<li>
											6. Use a <a
												href="https://www.windmill.dev/docs/flows/flow_trigger"
												target="_blank">trigger script</a
											>
											and schedule this flow to run as frequently as needed and compare a state persisted
											in Windmill to the state of the external system. If a difference is detected, then
											the rest of the flow is triggered. Oftentimes, the second step of a flow is a for-loop
											that will iterate over every elements. When using a trigger, a default schedule
											will be created.
											<img
												class="shadow-lg border rounded"
												alt="static button"
												src="/trigger_button.png"
											/>
										</li></ul
									>
								</div>
							</Slider>
						</div>
					</TabContent>
					<TabContent value="settings-schedule" class="p-4">
						<Alert
							type="info"
							title="Primary Schedule"
							documentationLink="https://www.windmill.dev/docs/core_concepts/scheduling"
						>
							Flows can be triggered by any schedules, their webhooks or their UI but they have only
							one primary schedulfs with which they share the same path. The primary schedule can be
							set here.
						</Alert>
						<div class="mt-4" />
						<FlowSchedules />
					</TabContent>

					<TabContent value="settings-same-worker" class="p-4 flex flex-col">
						<Alert type="info" title="Shared Directory">
							Steps will share a folder at `./shared` in which they can store heavier data and pass
							them to the next step. <br /><br />Beware that the `./shared` folder is not preserved
							across suspends and sleeps. <br /><br />
							Furthermore, steps' worker groups is not respected and only the flow's worker group will
							be respected.
						</Alert>
						<span class="my-4 text-lg font-bold">Shared Directory</span>
						<Toggle
							bind:checked={$flowStore.value.same_worker}
							options={{
								right: 'Shared Directory on `./shared`'
							}}
						/>
					</TabContent>
					<TabContent value="settings-worker-group" class="p-4 flex flex-col">
						<Alert type="info" title="Worker Group">
							When a worker group is defined at the flow level, any steps inside the flow will run
							on that worker group, regardless of the steps' worker group. If no worker group is
							defined, the flow controls will be executed by the default worker group 'flow' and the
							steps will be executed in their respective worker group.
						</Alert>
						<span class="my-4 text-lg font-bold">Worker Group</span>
						{#if $workerTags}
							{#if $workerTags?.length > 0}
								<div class="w-40">
									<select
										placeholder="Worker group"
										bind:value={$flowStore.tag}
										on:change={(e) => {
											if ($flowStore.tag == '') {
												$flowStore.tag = undefined
											}
										}}
									>
										{#if $flowStore.tag}
											<option value="">reset to default</option>
										{:else}
											<option value="" disabled selected>Worker Group</option>
										{/if}
										{#each $workerTags ?? [] as tag (tag)}
											<option value={tag}>{tag}</option>
										{/each}
									</select>
								</div>
							{:else}
								<div class="text-sm text-secondary italic mb-2">
									No custom worker group defined on this instance. See <a
										href="https://www.windmill.dev/docs/core_concepts/worker_groups"
										target="_blank">documentation</a
									>
								</div>
							{/if}
						{:else}
							<Loader2 class="animate-spin" />
						{/if}
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</div>
	</FlowCard>
</div>
