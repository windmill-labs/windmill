<script lang="ts">
	import { run, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import Path from '$lib/components/Path.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert, Button, SecondsInput } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import ErrorHandlerToggleButtonV2 from '$lib/components/details/ErrorHandlerToggleButtonV2.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import Badge from '$lib/components/Badge.svelte'
	import AIFormSettings from '$lib/components/copilot/AIFormSettings.svelte'
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass } from '$lib/components/text_input/TextInput.svelte'
	import { slide } from 'svelte/transition'
	import DebounceLimit from '../DebounceLimit.svelte'
	import EEOnly from '$lib/components/EEOnly.svelte'

	interface Props {
		noEditor: boolean
		enableAi?: boolean
	}

	let { noEditor, enableAi }: Props = $props()

	const {
		flowStore,
		initialPathStore,
		previewArgs,
		pathStore,
		customUi,
		preserveOnBehalfOf,
		savedOnBehalfOfEmail
	} = getContext<FlowEditorContext>('FlowEditorContext')

	const WM_DEPLOYERS_GROUP = 'wm_deployers'
	let isDeployer = $derived($userStore?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	let showPreserveToggle = $derived(
		isDeployer &&
			flowStore.val.on_behalf_of_email &&
			$savedOnBehalfOfEmail &&
			$savedOnBehalfOfEmail !== $userStore?.email
	)

	function asSchema(x: any) {
		return x as Schema
	}
	let path: Path | undefined = $state(undefined)
	let dirtyPath = $state(false)

	let displayWorkerTagPicker = $state(false)

	run(() => {
		flowStore.val.tag ? (displayWorkerTagPicker = true) : null
	})

	let activeAdvancedOptions = $derived([
		{
			name: 'Fill flow inputs with AI',
			active: typeof flowStore.val.schema?.prompt_for_ai == 'string'
		},
		{
			name: 'High Priority',
			active: flowStore.val.value.priority !== undefined && flowStore.val.value.priority > 0
		},
		{ name: 'Error Handler Muted', active: Boolean(flowStore.val.ws_error_handler_muted) },
		{ name: 'Invisible to Others', active: Boolean(flowStore.val.visible_to_runner_only) },
		{ name: 'Shared Directory', active: Boolean(flowStore.val.value.same_worker) },
		{ name: 'Cache Results', active: Boolean(flowStore.val.value.cache_ttl) },
		{ name: 'Early Stop', active: Boolean(flowStore.val.value.skip_expr) },
		{ name: 'Early Return', active: Boolean(flowStore.val.value.early_return) },
		{ name: 'Dedicated Worker', active: Boolean(flowStore.val.dedicated_worker) },
		{ name: 'Concurrent Limit', active: Boolean(flowStore.val.value.concurrent_limit) },
		{ name: 'Run on Behalf of Last Editor', active: Boolean(flowStore.val.on_behalf_of_email) },
		{ name: 'Worker Tag', active: displayWorkerTagPicker }
	])

	let activeAdvancedOptionNames = $derived(
		activeAdvancedOptions.filter((option) => option.active).map((option) => option.name)
	)
	let numberOfAdvancedOptionsOn = $derived(activeAdvancedOptionNames.length)
</script>

<div class="h-full flex flex-col">
	<FlowCard {noEditor} title="Settings">
		<div class="grow min-h-0 p-4 h-full flex flex-col gap-6">
			<!-- Metadata Section -->
			<div class="gap-6 flex flex-col">
				<Label label="Summary">
					<MetadataGen
						bind:content={flowStore.val.summary}
						promptConfigName="flowSummary"
						flow={flowStore.val.value}
						onChange={() => {
							if ($initialPathStore == '' && flowStore.val.summary?.length > 0 && !dirtyPath) {
								path?.setName(
									flowStore.val.summary
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
							initialPath={$initialPathStore}
							namePlaceholder="flow"
							kind="flow"
						/>
					</Label>
				{/if}

				<Label label="Description">
					<MetadataGen
						bind:content={flowStore.val.description}
						promptConfigName="flowDescription"
						flow={flowStore.val.value}
						class="w-full"
						elementType="textarea"
						elementProps={{
							id: 'inp',
							placeholder: 'What this flow does and how to use it'
						}}
					/>
				</Label>
			</div>

			<!-- Deployable Section -->
			<Section
				label="Advanced"
				collapsable={true}
				small={true}
				class="h-full grow mt-2 min-h-0 flex flex-col gap-6"
			>
				<!-- Worker Group Section -->
				{#if customUi?.settingsTabs?.workerGroup != false}
					<div>
						<Toggle
							textClass="font-medium"
							size="xs"
							checked={displayWorkerTagPicker}
							on:change={() => {
								displayWorkerTagPicker = !displayWorkerTagPicker
								if (!displayWorkerTagPicker) {
									flowStore.val.tag = undefined
								}
							}}
							options={{
								right: 'Worker group tag (queue)',
								rightTooltip:
									"When a worker group tag is defined at the flow level, any steps inside the flow will run on any worker group that listen to that tag, regardless of the steps tag. If no worker group tags is defined, the flow controls will be executed with the default tag 'flow' and the steps will be executed with their respective tag",
								rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/worker_groups'
							}}
						/>

						{#if displayWorkerTagPicker}
							<div transition:slide={{ duration: 120 }} class="mt-2">
								<WorkerTagPicker bind:tag={flowStore.val.tag} popupPlacement="top-end" />
							</div>
						{/if}
					</div>

					<!-- <Tooltip
								>In this mode, every scripts of this flow is run on the workers dedicated to this flow
								that keep the scripts "hot" so that there is not cold start cost incurred. Steps can run
								at >1500 rps in this mode.</Tooltip
							> -->
				{/if}

				<!-- Metadata Advanced Section -->
				{#snippet badge()}
					{#if numberOfAdvancedOptionsOn > 0}
						<div class="flex grow min-w-0 w-full flex-wrap gap-1 ps-2">
							{#each activeAdvancedOptionNames as optionName}
								<Badge twBgColor="bg-surface-sunken" twTextColor="text-primary">{optionName}</Badge>
							{/each}
						</div>
					{/if}
				{/snippet}

				<!-- Cache Section -->
				{#if customUi?.settingsTabs?.cache != false}
					<div>
						<Toggle
							textClass="font-medium"
							size="xs"
							checked={Boolean(flowStore.val.value.cache_ttl)}
							on:change={() => {
								if (flowStore.val.value.cache_ttl && flowStore.val.value.cache_ttl != undefined) {
									flowStore.val.value.cache_ttl = undefined
								} else {
									flowStore.val.value.cache_ttl = 300
								}
							}}
							options={{
								right: 'Cache the results for each possible inputs',
								rightTooltip:
									'When enabled, the flow will cache the results of the flow for each possible set of inputs.',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/cache#cache-flows'
							}}
						/>
						{#if flowStore.val.value.cache_ttl}
							<div class="flex gap-x-4 flex-col gap-1 mt-2" transition:slide={{ duration: 120 }}>
								<div class="text-2xs text-secondary">How long to keep the cache valid</div>
								<SecondsInput bind:seconds={flowStore.val.value.cache_ttl} />
								<Toggle
									size="2xs"
									bind:checked={
										() => flowStore.val.value.cache_ignore_s3_path,
										(v) => (flowStore.val.value.cache_ignore_s3_path = v || undefined)
									}
									options={{
										right: 'Ignore S3 Object paths for caching purposes',
										rightTooltip:
											'If two S3 objects passed as input have the same content, they will hit the same cache entry, regardless of their path.'
									}}
								/>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Early Stop Section -->
				{#if customUi?.settingsTabs?.earlyStop != false}
					<div>
						<!-- documentationLink="https://www.windmill.dev/docs/flows/early_stop -->
						<Toggle
							textClass="font-medium"
							size="xs"
							checked={Boolean(flowStore.val.value.skip_expr)}
							on:change={() => {
								if (Boolean(flowStore.val.value.skip_expr) && flowStore.val.value.skip_expr) {
									flowStore.val.value.skip_expr = undefined
								} else {
									flowStore.val.value.skip_expr = 'flow_input.foo == undefined'
								}
							}}
							options={{
								right: 'Early stop if condition met',
								rightTooltip:
									'If the inputs meet the predefined condition, the flow will not run.' +
									'to decide if the flow should stop early.',
								rightDocumentationLink:
									'https://www.windmill.dev/docs/flows/early_stop#early-stop-for-flow'
							}}
						/>
						{#if flowStore.val.value.skip_expr}
							<div
								class="w-full border rounded-md flex flex-col mt-2 {flowStore.val.value.skip_expr
									? ''
									: 'bg-surface-secondary'}"
							>
								<div class="w-full rounded-md overflow-auto">
									<SimpleEditor
										lang="javascript"
										small
										bind:code={flowStore.val.value.skip_expr}
										class="small-editor"
										extraLib={`declare const flow_input = ${JSON.stringify(
											schemaToObject(asSchema(flowStore.val.schema), previewArgs.val)
										)};
									declare const WM_SCHEDULED_FOR: string;`}
									/>
									<div class="text-xs text-hint p-2">
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
							textClass="font-medium"
							size="xs"
							checked={Boolean(flowStore.val.value.early_return)}
							on:change={() => {
								if (Boolean(flowStore.val.value.early_return) && flowStore.val.value.early_return) {
									flowStore.val.value.early_return = undefined
								} else {
									flowStore.val.value.early_return = flowStore.val.value.modules?.[0]?.id ?? 'a'
								}
							}}
							options={{
								right: 'Early return for sync webhooks',
								rightTooltip:
									'If defined, sync endpoints will return early at the node defined here while the rest of the flow continue asynchronously.',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/early_return'
							}}
						/>
						{#if flowStore.val.value.early_return}
							<div
								class="max-w-[120px] flex flex-col mt-2 {flowStore.val.value.early_return
									? ''
									: 'bg-surface-secondary'}"
								transition:slide={{ duration: 120 }}
							>
								<select
									name="oauth_name"
									id="oauth_name"
									bind:value={flowStore.val.value.early_return}
									class="text-xs"
								>
									<option value={undefined}>Node's id</option>
									{#each flowStore.val.value?.modules?.map((x) => x.id) as name}
										<option value={name}>{name}</option>
									{/each}
								</select>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Shared Directory Section -->
				{#if customUi?.settingsTabs?.sharedDiretory != false}
					<Toggle
						textClass="font-medium"
						size="xs"
						bind:checked={flowStore.val.value.same_worker}
						options={{
							right: 'Same Worker + Shared directory on `./shared`',
							rightTooltip:
								'Steps will share a folder at `./shared` in which they can store heavier data and ' +
								'pass them to the next step. Beware that the `./shared` folder is not ' +
								'preserved across suspends and sleeps.',
							rightDocumentationLink:
								'https://www.windmill.dev/docs/core_concepts/persistent_storage/within_windmill#shared-directory'
						}}
					/>
				{/if}

				<!-- Visibility Section -->
				<Toggle
					textClass="font-medium"
					size="xs"
					checked={Boolean(flowStore.val.visible_to_runner_only)}
					on:change={() => {
						if (flowStore.val.visible_to_runner_only) {
							flowStore.val.visible_to_runner_only = undefined
						} else {
							flowStore.val.visible_to_runner_only = true
						}
					}}
					options={{
						right: 'Make runs invisible to others',
						rightTooltip:
							'When this option is enabled, manual executions of this script are invisible to users other than the user running it, including the owner(s). This setting can be overridden when this script is run manually from the advanced menu.',
						rightDocumentationLink:
							'https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs#invisible-runs'
					}}
				/>

				<!-- On behalf of last editor section -->
				<Toggle
					textClass="font-medium"
					size="xs"
					checked={Boolean(flowStore.val.on_behalf_of_email)}
					on:change={() => {
						if (flowStore.val.on_behalf_of_email) {
							flowStore.val.on_behalf_of_email = undefined
							$preserveOnBehalfOf = false
						} else {
							flowStore.val.on_behalf_of_email = $userStore?.email
						}
					}}
					options={{
						right: 'Run on behalf of last editor',
						rightTooltip:
							'When this option is enabled, the flow will be run with the permissions of the last editor.'
					}}
				/>
				{#if showPreserveToggle}
					<Toggle
						textClass="font-medium"
						size="xs"
						bind:checked={$preserveOnBehalfOf}
						options={{
							right: `Keep original author (${$savedOnBehalfOfEmail})`
						}}
					/>
				{/if}

				<!-- Error Handler Section -->
				<div class="flex flex-row items-center py-1">
					<ErrorHandlerToggleButtonV2
						kind="flow"
						scriptOrFlowPath={$pathStore}
						bind:errorHandlerMuted={flowStore.val.ws_error_handler_muted}
					/>
					{#if !$enterpriseLicense}
						<EEOnly />
					{/if}
				</div>

				<!-- Concurrency Section -->
				{#if customUi?.settingsTabs?.concurrency != false}
					<div>
						<Toggle
							textClass="font-medium"
							size="xs"
							disabled={!$enterpriseLicense}
							checked={Boolean(flowStore.val.value.concurrent_limit)}
							on:change={() => {
								if (flowStore.val.value.concurrent_limit) {
									flowStore.val.value.concurrent_limit = undefined
								} else {
									flowStore.val.value.concurrent_limit = 1
								}
							}}
							options={{
								right: 'Concurrency limits',
								rightTooltip: 'Allowed concurrency within a given timeframe',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/concurrency_limit'
							}}
							eeOnly={true}
						/>

						{#if flowStore.val.value.concurrent_limit}
							<div class="flex flex-col gap-6 mt-6" transition:slide={{ duration: 120 }}>
								<Label label="Max number of executions within the time window">
									<div class="flex flex-row gap-2 max-w-sm whitespace-nowrap">
										<input
											disabled={!$enterpriseLicense}
											bind:value={flowStore.val.value.concurrent_limit}
											type="number"
										/>
										<Button
											size="sm"
											color="light"
											on:click={() => {
												flowStore.val.value.concurrent_limit = undefined
											}}
											variant="border">Remove Limits</Button
										>
									</div>
								</Label>
								<Label label="Time window in seconds">
									<div class="-mt-5">
										<SecondsInput
											disabled={!$enterpriseLicense}
											bind:seconds={flowStore.val.value.concurrency_time_window_s}
										/>
									</div>
								</Label>
								<Label label="Custom concurrency key (optional)">
									{#snippet header()}
										<Tooltip>
											Concurrency keys are global, you can have them be workspace specific using the
											variable `$workspace`. You can also use an argument's value using
											`$args[name_of_arg]`</Tooltip
										>
									{/snippet}
									<!-- svelte-ignore a11y_autofocus -->
									<input
										type="text"
										autofocus
										disabled={!$enterpriseLicense}
										bind:value={flowStore.val.value.concurrency_key}
										placeholder={`$workspace/script/${$pathStore}-$args[foo]`}
									/>
								</Label>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Debouncing Section -->
				{#if customUi?.settingsTabs?.debouncing != false}
					<DebounceLimit
						size="xs"
						fontClass="font-medium"
						bind:debounce_delay_s={flowStore.val.value.debounce_delay_s}
						bind:debounce_key={flowStore.val.value.debounce_key}
						bind:debounce_args_to_accumulate={flowStore.val.value.debounce_args_to_accumulate}
						bind:max_total_debouncing_time={flowStore.val.value.max_total_debouncing_time}
						bind:max_total_debounces_amount={flowStore.val.value.max_total_debounces_amount}
						schema={asSchema(flowStore.val.schema)}
						placeholder={`$workspace/flow/${$pathStore}-$args[foo]`}
					/>
				{/if}

				<!-- Priority Section -->
				<Toggle
					textClass="font-medium"
					size="xs"
					disabled={!$enterpriseLicense || isCloudHosted()}
					checked={flowStore.val.value.priority !== undefined && flowStore.val.value.priority > 0}
					on:change={() => {
						if (flowStore.val.value.priority) {
							flowStore.val.value.priority = undefined
						} else {
							flowStore.val.value.priority = 100
						}
					}}
					options={{
						right: `Label as high priority`,
						rightTooltip: `All jobs scheduled by flows labeled as high priority take precedence over the other jobs in the jobs queue. Higher priority numbers are executed first. ${
							!$enterpriseLicense ? 'This is a feature only available on enterprise edition.' : ''
						}`,
						rightDocumentationLink: 'https://www.windmill.dev/docs/flows/priority'
					}}
				>
					{#snippet right()}
						<input
							type="number"
							class={twMerge(
								inputBaseClass,
								inputBorderClass(),
								'!w-16 text-xs ml-4 absolute left-52'
							)}
							disabled={flowStore.val.value.priority === undefined}
							bind:value={flowStore.val.value.priority}
							onfocus={bubble('focus')}
							onchange={() => {
								if (flowStore.val.value.priority && flowStore.val.value.priority > 100) {
									flowStore.val.value.priority = 100
								} else if (flowStore.val.value.priority && flowStore.val.value.priority < 0) {
									flowStore.val.value.priority = 0
								}
							}}
						/>
						{#if !$enterpriseLicense || isCloudHosted()}
							<EEOnly />
						{/if}
					{/snippet}
				</Toggle>

				<div>
					<Toggle
						textClass="font-medium"
						size="xs"
						disabled={!$enterpriseLicense || isCloudHosted()}
						checked={Boolean(flowStore.val.dedicated_worker)}
						on:change={() => {
							if (flowStore.val.dedicated_worker) {
								flowStore.val.dedicated_worker = undefined
							} else {
								flowStore.val.dedicated_worker = true
							}
						}}
						options={{
							right: 'Flow is run on dedicated workers',
							rightTooltip: 'When enabled, the flow will be executed on a dedicated worker.',
							rightDocumentationLink:
								'https://www.windmill.dev/docs/core_concepts/jobs#high-priority-jobs'
						}}
						eeOnly={true}
					/>

					{#if flowStore.val.dedicated_worker}
						<div class="mt-2">
							<Alert type="info" title="Require dedicated workers">
								A worker group needs to be configured to listen to this flow. Select it in the dedicated
								workers section of the worker group configuration.
							</Alert>
						</div>
					{/if}
				</div>
				{#if flowStore.val.schema && enableAi}
					<AIFormSettings
						bind:prompt={flowStore.val.schema.prompt_for_ai as string | undefined}
						type="flow"
					/>
				{/if}
			</Section>
		</div>
	</FlowCard>
</div>
