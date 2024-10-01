<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { base } from '$lib/base'
	import Path from '$lib/components/Path.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert, Button, SecondsInput } from '$lib/components/common'
	import { getContext, beforeUpdate, afterUpdate } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import Slider from '$lib/components/Slider.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { copyToClipboard } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Clipboard } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import ErrorHandlerToggleButtonV2 from '$lib/components/details/ErrorHandlerToggleButtonV2.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import Badge from '$lib/components/Badge.svelte'

	export let noEditor: boolean

	const { flowStore, initialPath, previewArgs, pathStore, customUi } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
	$: url = `${hostname}/api/w/${$workspaceStore}/jobs/run/f/${$pathStore}`
	$: syncedUrl = `${hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${$pathStore}`

	function asSchema(x: any) {
		return x as Schema
	}
	let path: Path | undefined = undefined
	let dirtyPath = false

	let autoscroll = false

	let scrollableDiv: HTMLDivElement | undefined = undefined

	$: displayWorkerTagPicker = Boolean($flowStore.tag)

	$: activeAdvancedOptions = [
		{
			name: 'High Priority',
			active: $flowStore.value.priority !== undefined && $flowStore.value.priority > 0
		},
		{ name: 'Error Handler Muted', active: Boolean($flowStore.ws_error_handler_muted) },
		{ name: 'Invisible to Others', active: Boolean($flowStore.visible_to_runner_only) },
		{ name: 'Shared Directory', active: Boolean($flowStore.value.same_worker) },
		{ name: 'Cache Results', active: Boolean($flowStore.value.cache_ttl) },
		{ name: 'Early Stop', active: Boolean($flowStore.value.skip_expr) },
		{ name: 'Early Return', active: Boolean($flowStore.value.early_return) },
		{ name: 'Dedicated Worker', active: Boolean($flowStore.dedicated_worker) },
		{ name: 'Concurrent Limit', active: Boolean($flowStore.value.concurrent_limit) },
		{ name: 'Worker Tag', active: displayWorkerTagPicker }
	]

	$: numberOfAdvancedOptionsOn = activeAdvancedOptions.filter((option) => option.active).length
	$: activeAdvancedOptionNames = activeAdvancedOptions
		.filter((option) => option.active)
		.map((option) => option.name)

	beforeUpdate(() => {
		if (scrollableDiv) {
			const scrollableDistance = scrollableDiv.scrollHeight - scrollableDiv.offsetHeight
			autoscroll = scrollableDiv.scrollTop > scrollableDistance - 20
			console.log('autoscroll', autoscroll)
		}
	})

	afterUpdate(() => {
		if (autoscroll && scrollableDiv) {
			scrollableDiv.scrollTo(0, scrollableDiv.scrollHeight)
		}
	})
</script>

<div class="h-full overflow-y-auto flex flex-col" bind:this={scrollableDiv}>
	<FlowCard {noEditor} title="Settings">
		<div class="grow min-h-0 p-4 h-full flex flex-col gap-8">
			<!-- Metadata Section -->
			<Section label="Metadata" small={true}>
				<div class="h-full gap-8 flex flex-col">
					<Label label="Summary">
						<MetadataGen
							bind:content={$flowStore.summary}
							promptConfigName="flowSummary"
							flow={$flowStore.value}
							on:change={() => {
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
							elementProps={{
								type: 'text',
								id: 'flow-summary',
								placeholder: 'Short summary to be displayed when listed'
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
						<MetadataGen
							bind:content={$flowStore.description}
							promptConfigName="flowDescription"
							flow={$flowStore.value}
							class="w-full"
							elementType="textarea"
							elementProps={{
								id: 'inp',
								placeholder: 'What this flow does and how to use it.'
							}}
						/>
					</Label>

					<Label label="Triggers">
						<Slider text="Triggers">
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
										2. <a href="{base}/apps/add?nodraft=true" target="_blank"> App Editor</a> for customized-UIs
									</li>
									<li>
										3. <a href="{base}/schedules" target="_blank">Scheduling</a>
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
										that will iterate over every elements. When using a trigger, a default schedule will
										be created.
										<img
											class="shadow-lg border rounded"
											alt="static button"
											src="{base}/trigger_button.png"
										/>
									</li></ul
								>
							</div>
						</Slider>
					</Label>
				</div>
			</Section>

			<!-- Deployable Section -->
			<Section
				label="Advanced"
				collapsable={true}
				small={true}
				class="h-full grow  min-h-0 flex flex-col gap-4"
			>
				<!-- Metadata Advanced Section -->
				<!-- TODO: Add EE-only badge when we have it -->
				<svelte:fragment slot="badge">
					{#if numberOfAdvancedOptionsOn > 0}
						<div class="flex grow min-w-0 w-full flex-wrap gap-1">
							{#each activeAdvancedOptionNames as optionName}
								<Badge twBgColor="bg-nord-300" twTextColor="text-white">{optionName}</Badge>
							{/each}
						</div>
					{/if}
				</svelte:fragment>

				<!-- Priority Section -->
				<Toggle
					color="nord"
					lightToogle={true}
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
							!$enterpriseLicense ? 'This is a feature only available on enterprise edition.' : ''
						}`
					}}
					textDisabled={true}
					class="py-1 relative"
				>
					<svelte:fragment slot="right">
						<input
							type="number"
							class="!w-16 text-xs ml-4 absolute left-52"
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

				<!-- Visibility Section -->
				<Toggle
					color="nord"
					lightToogle={true}
					size="sm"
					checked={Boolean($flowStore.visible_to_runner_only)}
					on:change={() => {
						if ($flowStore.visible_to_runner_only) {
							$flowStore.visible_to_runner_only = undefined
						} else {
							$flowStore.visible_to_runner_only = true
						}
					}}
					options={{
						right: 'Make runs invisible to others',
						rightTooltip:
							'When this option is enabled, manual executions of this script are invisible to users other than the user running it, including the owner(s). This setting can be overridden when this script is run manually from the advanced menu.'
					}}
					class="py-1"
					textDisabled={true}
				/>

				<!-- Error Handler Section -->
				<div class="flex flex-row items-center py-1">
					<ErrorHandlerToggleButtonV2
						color="nord"
						kind="flow"
						scriptOrFlowPath={$pathStore}
						bind:errorHandlerMuted={$flowStore.ws_error_handler_muted}
						textDisabled={true}
					/>
				</div>

				<!-- Shared Directory Section -->
				{#if customUi?.settingsTabs?.sharedDiretory != false}
					<Toggle
						color="nord"
						lightToogle={true}
						bind:checked={$flowStore.value.same_worker}
						options={{
							right: 'Shared Directory on `./shared`',
							rightTooltip:
								'Steps will share a folder at `./shared` in which they can store heavier data and ' +
								'pass them to the next step. Beware that the `./shared` folder is not ' +
								'preserved across suspends and sleeps.'
						}}
						class="py-1"
						textDisabled={true}
					/>
				{/if}

				<!-- Cache Section -->
				{#if customUi?.settingsTabs?.cache != false}
					<div>
						<Toggle
							color="nord"
							lightToogle={true}
							size="sm"
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
							class="py-1"
							textDisabled={true}
						/>
						{#if $flowStore.value.cache_ttl}
							<div class="flex gap-x-4 flex-col gap-1">
								<div class="text-sm text-secondary">How long to keep the cache valid</div>
								<div>
									{#if $flowStore.value.cache_ttl}
										<SecondsInput bind:seconds={$flowStore.value.cache_ttl} />
									{:else}
										<SecondsInput disabled />
									{/if}
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Early Stop Section -->
				{#if customUi?.settingsTabs?.earlyStop != false}
					<div>
						<!-- documentationLink="https://www.windmill.dev/docs/flows/early_stop -->
						<Toggle
							color="nord"
							lightToogle={true}
							checked={Boolean($flowStore.value.skip_expr)}
							on:change={() => {
								if (Boolean($flowStore.value.skip_expr) && $flowStore.value.skip_expr) {
									$flowStore.value.skip_expr = undefined
								} else {
									$flowStore.value.skip_expr = 'flow_input.foo == undefined'
								}
							}}
							options={{
								right: 'Early stop if condition met',
								rightTooltip:
									'If defined, at the beginning of the step the predicate expression will be evaluated' +
									'to decide if the flow should stop early.'
							}}
							class="py-1"
							textDisabled={true}
						/>
						{#if $flowStore.value.skip_expr}
							<div
								class="w-full border flex flex-col {$flowStore.value.skip_expr
									? ''
									: 'bg-surface-secondary'}"
							>
								<div class="border w-full">
									<SimpleEditor
										lang="javascript"
										bind:code={$flowStore.value.skip_expr}
										class="small-editor"
										extraLib={`declare const flow_input = ${JSON.stringify(
											schemaToObject(asSchema($flowStore.schema), $previewArgs)
										)};
									declare const WM_SCHEDULED_FOR: string;`}
									/>
									<div class="text-xs text-tertiary mt-2">
										You can use the variable `flow_input` to access the inputs of the flow. <br
										/>The variable `WM_SCHEDULED_FOR` contains the time the flow was scheduled for
										which you can use to stop early non fresh jobs:
										<pre>new Date().getTime() - new Date(WM_SCHEDULED_FOR).getTime() {'>'} X</pre>
									</div>
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Early Return Section -->
				{#if customUi?.settingsTabs?.earlyReturn != false}
					<div>
						<Toggle
							color="nord"
							lightToogle={true}
							checked={Boolean($flowStore.value.early_return)}
							on:change={() => {
								if (Boolean($flowStore.value.early_return) && $flowStore.value.early_return) {
									$flowStore.value.early_return = undefined
								} else {
									$flowStore.value.early_return = $flowStore.value.modules?.[0]?.id ?? 'a'
								}
							}}
							options={{
								right: 'Early return sync endpoint at a top-level step',
								rightTooltip:
									'If defined, sync endpoints will return early at the node defined here while the rest of the flow continue asynchronously.'
							}}
							class="py-1"
							textDisabled={true}
						/>
						{#if $flowStore.value.early_return}
							<div
								class="max-w-[120px] flex flex-col {$flowStore.value.early_return
									? ''
									: 'bg-surface-secondary'}"
							>
								<select
									name="oauth_name"
									id="oauth_name"
									bind:value={$flowStore.value.early_return}
									class="text-xs"
								>
									<option value={undefined}>Node's id</option>
									{#each $flowStore.value?.modules?.map((x) => x.id) as name}
										<option value={name}>{name}</option>
									{/each}
								</select>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Worker Group Section -->
				{#if customUi?.settingsTabs?.workerGroup != false}
					<div>
						<Toggle
							color="nord"
							lightToogle={true}
							disabled={!$enterpriseLicense}
							size="sm"
							checked={displayWorkerTagPicker}
							on:change={() => {
								displayWorkerTagPicker = !displayWorkerTagPicker
							}}
							options={{
								right: 'Worker Group Tag (Queue)',
								rightTooltip:
									"When a worker group tag is defined at the flow level, any steps inside the flow will run on any worker group that listen to that tag, regardless of the steps tag. If no worker group tags is defined, the flow controls will be executed with the default tag 'flow' and the steps will be executed with their respective tag"
							}}
							class="py-1"
							textDisabled={true}
						/>

						{#if displayWorkerTagPicker}
							<WorkerTagPicker bind:tag={$flowStore.tag} />
						{/if}
					</div>

					<div>
						<Toggle
							color="nord"
							lightToogle={true}
							disabled={!$enterpriseLicense || isCloudHosted()}
							size="sm"
							checked={Boolean($flowStore.dedicated_worker)}
							on:change={() => {
								if ($flowStore.dedicated_worker) {
									$flowStore.dedicated_worker = undefined
								} else {
									$flowStore.dedicated_worker = true
								}
							}}
							options={{
								right: 'Flow is run on dedicated workers'
							}}
							class="py-1"
							textDisabled={true}
						/>
						{#if $flowStore.dedicated_worker}
							<div>
								<Alert type="info" title="Require dedicated workers">
									One worker in a worker group needs to be configured with dedicated worker set to: <pre
										>{$workspaceStore}:flow/{$pathStore}</pre
									>
								</Alert>
							</div>
						{/if}
					</div>

					<!-- <Tooltip
						>In this mode, every scripts of this flow is run on the workers dedicated to this flow
						that keep the scripts "hot" so that there is not cold start cost incurred. Steps can run
						at >1500 rps in this mode.</Tooltip
					> -->
				{/if}

				<!-- Concurrency Section -->
				{#if customUi?.settingsTabs?.concurrency != false}
					<div>
						<Toggle
							color="nord"
							lightToogle={true}
							disabled={!$enterpriseLicense}
							size="sm"
							checked={Boolean($flowStore.value.concurrent_limit)}
							on:change={() => {
								if ($flowStore.value.concurrent_limit) {
									$flowStore.value.concurrent_limit = undefined
								} else {
									$flowStore.value.concurrent_limit = 1
								}
							}}
							options={{
								right: 'Concurrency Limits',
								rightTooltip: 'Allowed concurrency within a given timeframe'
							}}
							class="py-1"
							textDisabled={true}
						/>

						{#if $flowStore.value.concurrent_limit}
							<div class="flex flex-col gap-4">
								<Label label="Max number of executions within the time window">
									<div class="flex flex-row gap-2 max-w-sm">
										<input
											disabled={!$enterpriseLicense}
											bind:value={$flowStore.value.concurrent_limit}
											type="number"
										/>
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
									<SecondsInput
										disabled={!$enterpriseLicense}
										bind:seconds={$flowStore.value.concurrency_time_window_s}
									/>
								</Label>
								<Label label="Custom concurrency key (optional)">
									<svelte:fragment slot="header">
										<Tooltip>
											Concurrency keys are global, you can have them be workspace specific using the
											variable `$workspace`. You can also use an argument's value using
											`$args[name_of_arg]`</Tooltip
										>
									</svelte:fragment>
									<input
										type="text"
										autofocus
										disabled={!$enterpriseLicense}
										bind:value={$flowStore.value.concurrency_key}
										placeholder={`$workspace/script/${$pathStore}-$args[foo]`}
									/>
								</Label>
							</div>
						{/if}
					</div>
				{/if}
			</Section>
		</div>
	</FlowCard>
</div>
