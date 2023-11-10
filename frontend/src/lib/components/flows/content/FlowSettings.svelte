<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import { BROWSER } from 'esm-env'
	import Path from '$lib/components/Path.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert, Button, SecondsInput } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import autosize from 'svelte-autosize'
	import Slider from '$lib/components/Slider.svelte'
	import { enterpriseLicense, workerTags, workspaceStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { copyToClipboard } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { WorkerService } from '$lib/gen'
	import { Clipboard, Loader2 } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import ErrorHandlerToggleButton from '$lib/components/details/ErrorHandlerToggleButton.svelte'

	export let noEditor: boolean

	const { selectedId, flowStore, initialPath, previewArgs, pathStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}

	let hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
	$: url = `${hostname}/api/w/${$workspaceStore}/jobs/run/f/${$pathStore}`
	$: syncedUrl = `${hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${$pathStore}`

	$: if ($selectedId == 'settings-worker-group') {
		$workerTags = undefined
		loadWorkerGroups()
	}
	$: isStopAfterIfEnabled = Boolean($flowStore.value.skip_expr)

	function asSchema(x: any) {
		return x as Schema
	}
	let path: Path | undefined = undefined
	let dirtyPath = false
</script>

<div class="h-full overflow-hidden">
	<FlowCard {noEditor} title="Settings">
		<div class="h-full flex-1">
			<Tabs bind:selected={$selectedId}>
				<Tab value="settings-metadata">Metadata</Tab>
				{#if !noEditor}
					<Tab value="settings-schedule">Schedule</Tab>
				{/if}
				<Tab value="settings-same-worker">Shared Directory</Tab>
				<Tab value="settings-early-stop">Early Stop</Tab>
				<Tab value="settings-worker-group">Worker Group</Tab>
				<Tab value="settings-concurrency">Concurrency</Tab>
				<Tab value="settings-cache">Cache</Tab>

				<svelte:fragment slot="content">
					<TabContent value="settings-metadata" class="p-4 h-full">
						<div class="h-full gap-8 flex flex-col">
							<Label label="Summary">
								<input
									type="text"
									autofocus
									bind:value={$flowStore.summary}
									placeholder="Short summary to be displayed when listed"
									id="flow-summary"
									on:keyup={() => {
										if (initialPath == '' && $flowStore.summary?.length > 0 && !dirtyPath) {
											path?.setName(
												$flowStore.summary
													.toLowerCase()
													.replace(/[^a-z0-9_]/g, '_')
													.replace(/-+/g, '_')
													.replace(/^-|-$/g, '')
											)
										}
									}}
								/>
							</Label>

							{#if !noEditor}
								<Label label="Path">
									<Path
										autofocus={false}
										bind:this={path}
										bind:dirty={dirtyPath}
										bind:path={$pathStore}
										{initialPath}
										namePlaceholder="flow"
										kind="flow"
									/>
								</Label>
							{/if}

							<Label label="Description">
								<textarea
									use:autosize
									type="text"
									class="text-sm"
									id="inp"
									bind:value={$flowStore.description}
									placeholder="What this flow does and how to use it."
									rows="3"
								/>
							</Label>

							<!-- TODO: Add EE-only badge when we have it -->
							<Toggle
								disabled={!$enterpriseLicense || isCloudHosted()}
								checked={$flowStore.value.priority !== undefined && $flowStore.value.priority > 0}
								on:change={() => {
									if ($flowStore.value.priority) {
										$flowStore.value.priority = undefined
									} else {
										$flowStore.value.priority = 100
									}
								}}
								options={{
									right: `Label as high priority`,
									rightTooltip: `All jobs scheduled by flows labeled as high priority take precedence over the other jobs in the jobs queue. ${
										!$enterpriseLicense
											? 'This is a feature only available on enterprise edition.'
											: ''
									}`
								}}
							>
								<svelte:fragment slot="right">
									<input
										type="number"
										class="!w-16 ml-4"
										disabled={$flowStore.value.priority === undefined}
										bind:value={$flowStore.value.priority}
										on:focus
										on:change={() => {
											if ($flowStore.value.priority && $flowStore.value.priority > 100) {
												$flowStore.value.priority = 100
											} else if ($flowStore.value.priority && $flowStore.value.priority < 0) {
												$flowStore.value.priority = 0
											}
										}}
									/>
								</svelte:fragment>
							</Toggle>

							<div class="flex flex-row items-center gap-1">
								<ErrorHandlerToggleButton
									kind="flow"
									scriptOrFlowPath={$pathStore}
									bind:errorHandlerMuted={$flowStore.ws_error_handler_muted}
									iconOnly={false}
								/>
							</div>

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
															<Clipboard />
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
															<Clipboard />
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
					<TabContent value="settings-schedule" class="p-4 h-full overflow-scroll">
						<Alert
							type="info"
							title="Primary Schedule"
							documentationLink="https://www.windmill.dev/docs/core_concepts/scheduling"
						>
							Flows can be triggered by any schedules, their webhooks or their UI but they have only
							one primary schedule with which they share the same path. The primary schedule can be
							set here.
						</Alert>
						<div class="mt-4" />
						<FlowSchedules />
					</TabContent>

					<TabContent value="settings-same-worker" class="p-4 flex flex-col">
						<Section label="Shared Directory" class="flex flex-col">
							<Alert type="info" title="Shared Directory">
								Steps will share a folder at `./shared` in which they can store heavier data and
								pass them to the next step. <br /><br />Beware that the `./shared` folder is not
								preserved across suspends and sleeps. <br /><br />
								Furthermore, steps' worker groups is not respected and only the flow's worker group will
								be respected.
							</Alert>
							<Toggle
								bind:checked={$flowStore.value.same_worker}
								options={{
									right: 'Shared Directory on `./shared`'
								}}
							/>
						</Section>
					</TabContent>
					<TabContent value="settings-cache" class="p-4 flex flex-col">
						<h2 class="border-b pb-1 mb-4 flex items-center gap-4"
							>Cache <Toggle
								size="xs"
								checked={Boolean($flowStore.value.cache_ttl)}
								on:change={() => {
									if ($flowStore.value.cache_ttl && $flowStore.value.cache_ttl != undefined) {
										$flowStore.value.cache_ttl = undefined
									} else {
										$flowStore.value.cache_ttl = 300
									}
								}}
								options={{
									right: 'Cache the results for each possible inputs'
								}}
							/></h2
						>

						<div class="flex gap-x-4 flex-col gap-2">
							<div class="text-xs">How long to keep the cache valid</div>
							<div>
								{#if $flowStore.value.cache_ttl}
									<SecondsInput bind:seconds={$flowStore.value.cache_ttl} />
								{:else}
									<SecondsInput disabled />
								{/if}
							</div>
						</div>
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
									No custom worker group tag defined on this instance in "Workers {'->'} Assignable Tags".
									See
									<a
										href="https://www.windmill.dev/docs/core_concepts/worker_groups"
										target="_blank">documentation</a
									>
								</div>
							{/if}
						{:else}
							<Loader2 class="animate-spin" />
						{/if}
					</TabContent>
					<TabContent value="settings-early-stop" class="p-4">
						<Section label="Early stop">
							<svelte:fragment slot="header">
								<Tooltip documentationLink="https://www.windmill.dev/docs/flows/early_stop">
									If defined, at the beginning of the step the predicate expression will be
									evaluated to decide if the flow should stop early.
								</Tooltip>
							</svelte:fragment>
							<Toggle
								checked={isStopAfterIfEnabled}
								on:change={() => {
									if (isStopAfterIfEnabled && $flowStore.value.skip_expr) {
										$flowStore.value.skip_expr = undefined
									} else {
										$flowStore.value.skip_expr = 'flow_input.foo == undefined'
									}
								}}
								options={{
									right: 'Early stop if condition met'
								}}
							/>

							<div
								class="w-full border mt-2 p-2 flex flex-col {$flowStore.value.skip_expr
									? ''
									: 'bg-surface-secondary'}"
							>
								{#if $flowStore.value.skip_expr}
									<div class="border w-full">
										<SimpleEditor
											lang="javascript"
											bind:code={$flowStore.value.skip_expr}
											class="small-editor"
											extraLib={`declare const flow_input = ${JSON.stringify(
												schemaToObject(asSchema($flowStore.schema), $previewArgs)
											)};`}
										/>
									</div>
								{:else}
									<textarea disabled rows="3" class="min-h-[80px]" />
								{/if}
							</div>
						</Section>
					</TabContent>

					<TabContent value="settings-concurrency" class="p-4 flex flex-col">
						<Section label="Concurrency Limits">
							<svelte:fragment slot="header">
								<Tooltip>Allowed concurrency within a given timeframe</Tooltip>
							</svelte:fragment>
							{#if !$enterpriseLicense}
								<Alert
									title="Concurrency limits are going to become an Enterprise Edition feature"
									type="warning"
								/>
							{/if}
							<div class="flex flex-col gap-4">
								<Label label="Max number of executions within the time window">
									<div class="flex flex-row gap-2 max-w-sm">
										<input bind:value={$flowStore.value.concurrent_limit} type="number" />
										<Button
											size="sm"
											color="light"
											on:click={() => {
												$flowStore.value.concurrent_limit = undefined
											}}
											variant="border">Remove Limits</Button
										>
									</div>
								</Label>
								<Label label="Time window in seconds">
									<SecondsInput bind:seconds={$flowStore.value.concurrency_time_window_s} />
								</Label>
							</div>
						</Section>
					</TabContent>
				</svelte:fragment>
			</Tabs>
		</div>
	</FlowCard>
</div>
