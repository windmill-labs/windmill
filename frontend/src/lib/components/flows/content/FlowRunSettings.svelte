<script lang="ts">
	import { getContext } from 'svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'

	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { SecondsInput } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import s3Scripts from './s3Scripts/lib'

	import FlowRetries from './FlowRetries.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleTimeout from './FlowModuleTimeout.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowModuleCache from './FlowModuleCache.svelte'
	import { hasInlineConcurrency, stepSettingDefaults } from '../flowStepSettings'

	import FlowModuleDebounce from './FlowModuleDebounce.svelte'
	import FlowModuleMock from './FlowModuleMock.svelte'
	import WorkspaceScriptSettingInfo from './WorkspaceScriptSettingInfo.svelte'
	import { slideDynamic } from '$lib/transitions'

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		selectedId: string
		/** Lay out for embedding inside another scroll container (no own scroll/padding). */
		embedded?: boolean
		/** Container steps (loops and branches): keep only the subset of settings they support
		 *  (Flow control rows + Lifetime), hiding the rest of Execution policy. */
		loopSubset?: boolean
		/** Loops render the per-iteration break next to their own settings, so they ask
		 *  for the all-iterations predicate only. */
		earlyStopBlocks?: 'both' | 'all-iters'
		// For workspace-script steps: the concurrency/cache settings currently set on
		// the referenced script, and a shortcut to edit them. Undefined for
		// inline/subflow steps.
		referencedConcurrentLimit?: number | undefined
		referencedConcurrencyTimeWindowS?: number | undefined
		workspaceScriptCacheTtl?: number | undefined
		loadingWorkspaceScript?: boolean
		workspaceScriptError?: string | undefined
		canEditWorkspaceScript?: boolean
		workspaceScriptNoEditReason?: string | undefined
		onEditWorkspaceScript?: () => void
		/** Replace the inline script's code — provided by the step editor. */
		onApplyS3Snippet?: (code: string) => void
		/** Agent tools never go through the flow scheduler, so the settings `same_worker`
		 *  rules out for a flow step still apply to them. */
		isAgentTool?: boolean
	}

	let {
		flowModule = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		selectedId,
		embedded = false,
		loopSubset = false,
		earlyStopBlocks = 'both',
		referencedConcurrentLimit = undefined,
		referencedConcurrencyTimeWindowS = undefined,
		workspaceScriptCacheTtl = undefined,
		loadingWorkspaceScript = false,
		workspaceScriptError = undefined,
		canEditWorkspaceScript = false,
		workspaceScriptNoEditReason = undefined,
		onEditWorkspaceScript,
		onApplyS3Snippet,
		isAgentTool = false
	}: Props = $props()

	let rootEl: HTMLDivElement | undefined = $state()

	/** Scroll one setting into view. `key` is a StepSettingKey, so the AI tool's openTab
	 *  intent must name settings with those keys. */
	export function openSetting(key: string) {
		rootEl?.querySelector(`[data-setting="${key}"]`)?.scrollIntoView({ block: 'nearest' })
	}

	// Exact id: only the flow's error handler is 'failure'; a step the user named
	// something like handle_failure is an ordinary step and keeps every row.
	const isFailure = $derived(selectedId === 'failure')
	const isRawScript = $derived(flowModule.value.type === 'rawscript')
	const isWorkspaceScript = $derived(flowModule.value.type === 'script')
	const s3Language = $derived(
		flowModule.value.type === 'rawscript' &&
			(flowModule.value.language === 'python3' || flowModule.value.language === 'deno')
			? flowModule.value.language
			: undefined
	)
	let s3Kind = $state<'s3_client' | 'polars' | 'duckdb'>('s3_client')
	const s3Snippet = $derived(s3Language ? s3Scripts[s3Language][s3Kind] : undefined)
	const concurrencyOn = $derived(hasInlineConcurrency(flowModule))
	const concurrencyOff = $derived(!$enterpriseLicense || !concurrencyOn)
	// A resolved approval is recorded against the step the gate holds back, not this one, so
	// `continue_on_error` never sees it — the suspend option is the only way to continue past it.
	const suspendNeedsItsOwnContinueToggle = $derived(
		Boolean(flowModule.continue_on_error) &&
			Boolean(flowModule.suspend) &&
			!flowModule.suspend?.continue_on_disapprove_timeout
	)
</script>

{#snippet sectionHeader(title: string)}
	<div class="text-[11px] font-medium uppercase tracking-[0.04em] text-hint">
		{title}
	</div>
{/snippet}

<div
	bind:this={rootEl}
	class="flex flex-col gap-8 {embedded ? '' : 'flex-1 min-h-0 overflow-auto p-4 pb-8'}"
	style={embedded ? undefined : 'scrollbar-gutter: stable'}
>
	{#if !isFailure}
		<section class="flex flex-col gap-3">
			{@render sectionHeader('Flow control')}

			<div class="flex flex-col gap-6">
				<div data-setting="skip">
					<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
				</div>

				<div data-setting="early-stop">
					<FlowModuleEarlyStop bind:flowModule blocks={earlyStopBlocks} />
				</div>

				<div data-setting="suspend">
					<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
				</div>

				<div data-setting="sleep">
					<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule {isAgentTool} />
				</div>
			</div></section
		>
	{/if}

	<section class="flex flex-col gap-3">
		{@render sectionHeader('Execution policy')}

		<div class="flex flex-col gap-6">
			{#if !loopSubset}
				<div data-setting="retries">
					<FlowRetries bind:flowModuleRetry={flowModule.retry} bind:flowModule {isAgentTool} />
				</div>

				<div data-setting="error-handling" class="flex flex-col gap-2">
					<Toggle
						size="xs"
						textClass="text-xs font-normal text-primary"
						bind:checked={flowModule.continue_on_error}
						options={{
							right: 'Continue on error',
							rightTooltip:
								"The flow continues to the next step even if this step fails (after exhausting retries, if any). The step's error becomes its return, so a following branch can handle it."
						}}
					/>
					{#if suspendNeedsItsOwnContinueToggle}
						<Alert type="info" title="Does not cover the approval" size="xs">
							This only applies when the step's own code fails. A disapproval or an approval timeout
							is not a failure of this step, so it still stops the flow. To continue past those,
							turn on "Continue on disapproval/timeout" in the approval settings.
						</Alert>
					{/if}
				</div>

				<!-- The error handler runs outside the flow's control graph; only its own
			     failure handling applies. -->
				{#if !isFailure}
					<div data-setting="timeout">
						<FlowModuleTimeout previousModuleId={previousModule?.id} bind:flowModule />
					</div>

					{#if isRawScript || isWorkspaceScript}
						<div data-setting="concurrency">
							{#if flowModule.value.type === 'script'}
								<WorkspaceScriptSettingInfo
									label="Concurrency limit"
									active={referencedConcurrentLimit != undefined}
									valueText={referencedConcurrentLimit != undefined
										? `Max ${referencedConcurrentLimit} execution${
												referencedConcurrentLimit === 1 ? '' : 's'
											}${
												referencedConcurrencyTimeWindowS != undefined
													? ` within ${referencedConcurrencyTimeWindowS}s`
													: ''
											}`
										: undefined}
									loading={loadingWorkspaceScript}
									error={workspaceScriptError}
									canEdit={canEditWorkspaceScript}
									noEditReason={workspaceScriptNoEditReason}
									onEdit={onEditWorkspaceScript}
								/>
							{:else if flowModule.value.type === 'rawscript'}
								<div class="flex flex-col gap-2">
									<Toggle
										size="xs"
										textClass="text-xs font-normal text-primary"
										eeOnly
										disabled={!$enterpriseLicense}
										checked={concurrencyOn}
										on:change={() => {
											if (flowModule.value.type !== 'rawscript') return
											flowModule.value.concurrent_limit = concurrencyOn ? undefined : 1
										}}
										options={{
											right: 'Concurrency limit',
											rightTooltip: 'Allowed concurrency within a given timeframe.',
											rightDocumentationLink:
												'https://www.windmill.dev/docs/flows/concurrency_limit'
										}}
									/>
									{#if concurrencyOn}
										<div class="flex flex-col gap-2 pl-9" transition:slideDynamic>
											<Label label="Max number of executions within the time window">
												<input
													disabled={concurrencyOff}
													bind:value={flowModule.value.concurrent_limit}
													type="number"
													min="1"
													class="!w-24"
												/>
											</Label>
											<Label label="Time window in seconds">
												<SecondsInput
													disabled={concurrencyOff}
													bind:seconds={flowModule.value.concurrency_time_window_s}
													clearable
												/>
											</Label>
											<Label label="Custom concurrency key (optional)">
												{#snippet header()}
													<Tooltip>
														Concurrency keys are global, you can have them be workspace specific
														using the variable `$workspace`. You can also use an argument's value
														using `$args[name_of_arg]`</Tooltip
													>
												{/snippet}
												<input
													type="text"
													disabled={concurrencyOff}
													bind:value={flowModule.value.custom_concurrency_key}
													placeholder={`$workspace/script/${$pathStore}-$args[foo]`}
												/>
											</Label>
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/if}

					<div data-setting="priority" class="flex flex-col gap-2">
						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							eeOnly
							disabled={!$enterpriseLicense || isCloudHosted()}
							checked={flowModule.priority !== undefined}
							on:change={() => {
								if (flowModule.priority !== undefined) {
									flowModule.priority = undefined
								} else {
									flowModule.priority = stepSettingDefaults('priority')
								}
							}}
							options={{
								right: 'High priority',
								rightTooltip:
									'Jobs scheduled from this step take precedence over other jobs in the queue when the flow runs.'
							}}
						/>
						{#if flowModule.priority !== undefined}
							<div class="pl-9" transition:slideDynamic>
								<Label label="Priority number">
									{#snippet header()}
										<Tooltip>The higher the number, the higher the priority.</Tooltip>
									{/snippet}
									<input
										type="number"
										class="!w-24"
										bind:value={flowModule.priority}
										onchange={() => {
											if (flowModule.priority && flowModule.priority > 100) {
												flowModule.priority = 100
											} else if (flowModule.priority && flowModule.priority < 0) {
												flowModule.priority = 0
											}
										}}
									/>
								</Label>
							</div>
						{/if}
						<!-- The EE half of this limitation is the toggle's own badge; only the
						     cloud one needs saying. -->
						{#if isCloudHosted()}
							<Alert type="warning" title="Limitation" size="xs">
								Setting priority is not available on the cloud.
							</Alert>
						{/if}
					</div>

					<div data-setting="cache">
						<FlowModuleCache
							bind:flowModule
							{workspaceScriptCacheTtl}
							{loadingWorkspaceScript}
							{workspaceScriptError}
							{canEditWorkspaceScript}
							{workspaceScriptNoEditReason}
							{onEditWorkspaceScript}
						/>
					</div>

					<div data-setting="debounce">
						<FlowModuleDebounce bind:flowModule {selectedId} />
					</div>
				{/if}
			{/if}

			{#if !isFailure}
				<div data-setting="lifetime">
					<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
				</div>
			{/if}

			{#if loopSubset}
				<div data-setting="mock">
					<FlowModuleMock bind:flowModule />
				</div>
			{/if}
		</div></section
	>

	{#if s3Language && onApplyS3Snippet && !isFailure}
		<section class="flex flex-col gap-3" data-setting="s3">
			{@render sectionHeader('S3 snippets')}
			<p class="text-xs text-tertiary">
				Read and write S3 objects, and use Polars or DuckDB to run efficient ETL processes.
			</p>
			<div class="flex flex-row items-center justify-between gap-2">
				<ToggleButtonGroup bind:selected={s3Kind} class="w-auto">
					{#snippet children({ item })}
						{#if s3Language === 'deno'}
							<ToggleButton value="s3_client" small label="S3 lite client" {item} />
						{:else}
							<ToggleButton value="s3_client" small label="Boto3" {item} />
							<ToggleButton value="polars" small label="Polars" {item} />
							<ToggleButton value="duckdb" small label="DuckDB" {item} />
						{/if}
					{/snippet}
				</ToggleButtonGroup>
				<Button
					size="xs"
					variant="default"
					on:click={() => s3Snippet && onApplyS3Snippet?.(s3Snippet)}
				>
					Apply snippet
				</Button>
			</div>
			{#if s3Snippet}
				<div class="overflow-auto max-h-64 border rounded-md">
					<HighlightCode language={s3Language} code={s3Snippet} />
				</div>
			{/if}
		</section>
	{/if}
</div>
