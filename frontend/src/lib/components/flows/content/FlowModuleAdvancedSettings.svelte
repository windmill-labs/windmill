<script lang="ts">
	import { createBubbler } from 'svelte/legacy'
	import { getContext } from 'svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'

	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Button, SecondsInput } from '$lib/components/common'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	import FlowRetries from './FlowRetries.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleTimeout from './FlowModuleTimeout.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowModuleCache from './FlowModuleCache.svelte'
	import FlowModuleDebounce from './FlowModuleDebounce.svelte'
	import s3Scripts from './s3Scripts/lib'

	const bubble = createBubbler()

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		selectedId: string
		/** Monaco editor of the step, used to apply S3 snippets. */
		editor?: any
	}

	let {
		flowModule = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		selectedId,
		editor = undefined
	}: Props = $props()

	let s3Kind = $state('s3_client')

	let root: HTMLDivElement | undefined = $state()

	// Called by the node header's quick-toggles (Suspend/Sleep/…) to reveal the
	// matching row now that the Advanced tab is a single scrolling column.
	export function scrollToSection(section: string) {
		const anchor = section === 'runtime' ? 'concurrency' : section
		root
			?.querySelector(`#advanced-${anchor}`)
			?.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	const isS3Lang = $derived(
		flowModule.value['language'] === 'python3' || flowModule.value['language'] === 'deno'
	)
</script>

{#snippet groupTitle(title: string)}
	<div class="text-xs text-secondary capitalize">{title}</div>
{/snippet}

{#snippet rowLabel(text: string, tip?: string, doc?: string)}
	<div class="flex flex-row items-center gap-1 text-xs font-normal text-primary">
		{text}
		{#if tip}<Tooltip documentationLink={doc}>{tip}</Tooltip>{/if}
	</div>
{/snippet}

{#snippet errorHandling()}
	<div id="advanced-retries" class="flex flex-col gap-3">
		<Toggle
			size="xs"
			textClass="text-xs font-normal text-primary"
			bind:checked={flowModule.continue_on_error}
			options={{
				right: 'Continue to the next step even if this step fails',
				rightTooltip:
					"The flow continues to the next step even if this step fails (after exhausting retries, if any). The step's error becomes its return, so a following branch can handle it."
			}}
		/>
		<FlowRetries bind:flowModuleRetry={flowModule.retry} bind:flowModule />
	</div>
{/snippet}

<div bind:this={root} class="flex-1 min-h-0 overflow-auto p-4 flex flex-col gap-5">
	{#if selectedId.includes('failure')}
		{@render errorHandling()}
	{:else}
		<!-- Control flow -->
		<div class="flex flex-col gap-3">
			{@render groupTitle('Control flow')}
			<div id="advanced-early-stop">
				<FlowModuleEarlyStop bind:flowModule />
			</div>
			<div id="advanced-skip">
				<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
			</div>
			<div id="advanced-suspend">
				<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
			</div>
			<div id="advanced-sleep">
				<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
			</div>
		</div>

		<!-- Runtime -->
		<div class="flex flex-col gap-3">
			{@render groupTitle('Runtime')}
			{@render errorHandling()}

			<div id="advanced-concurrency">
				<div class="flex flex-col gap-3">
					{#if flowModule.value.type === 'rawscript'}
						<Toggle
							size="xs"
							textClass="text-xs font-normal text-primary"
							eeOnly
							disabled={!$enterpriseLicense}
							checked={Boolean(flowModule.value.concurrent_limit)}
							on:change={() => {
								if (flowModule.value.type !== 'rawscript') return
								if (flowModule.value.concurrent_limit) {
									flowModule.value.concurrent_limit = undefined
								} else {
									flowModule.value.concurrent_limit = 1
								}
							}}
							options={{
								right: 'Limit the number of concurrent executions',
								rightTooltip: 'Allowed concurrency within a given timeframe.',
								rightDocumentationLink: 'https://www.windmill.dev/docs/flows/concurrency_limit'
							}}
						/>
						{#if flowModule.value.concurrent_limit}
							<Label label="Max number of executions within the time window">
								<input
									disabled={!$enterpriseLicense}
									bind:value={flowModule.value.concurrent_limit}
									type="number"
									class="!w-24"
								/>
							</Label>
							<Label label="Time window in seconds">
								<SecondsInput
									disabled={!$enterpriseLicense}
									bind:seconds={flowModule.value.concurrency_time_window_s}
									clearable
								/>
							</Label>
							<Label label="Custom concurrency key (optional)">
								{#snippet header()}
									<Tooltip>
										Concurrency keys are global, you can have them be workspace specific using the
										variable `$workspace`. You can also use an argument's value using
										`$args[name_of_arg]`</Tooltip
									>
								{/snippet}
								<input
									type="text"
									disabled={!$enterpriseLicense}
									bind:value={flowModule.value.custom_concurrency_key}
									placeholder={`$workspace/script/${$pathStore}-$args[foo]`}
								/>
							</Label>
						{/if}
					{:else}
						<Alert type="warning" title="Limitation" size="xs">
							The concurrency limit of a workspace script is only settable in the script metadata
							itself. For hub scripts, this feature is non available yet.
						</Alert>
					{/if}
				</div>
			</div>

			<div id="advanced-timeout">
				<FlowModuleTimeout previousModuleId={previousModule?.id} bind:flowModule />
			</div>

			<div id="advanced-priority">
				<div class="flex flex-col gap-3">
					<Toggle
						size="xs"
						textClass="text-xs font-normal text-primary"
						eeOnly
						disabled={!$enterpriseLicense || isCloudHosted()}
						checked={flowModule.priority !== undefined && flowModule.priority > 0}
						on:change={() => {
							if (flowModule.priority) {
								flowModule.priority = undefined
							} else {
								flowModule.priority = 100
							}
						}}
						options={{
							right: 'Run this step as a high priority job',
							rightTooltip:
								'Jobs scheduled from this step take precedence over other jobs in the queue when the flow runs.'
						}}
					/>
					{#if flowModule.priority !== undefined}
						<Label label="Priority number">
							{#snippet header()}
								<Tooltip>The higher the number, the higher the priority.</Tooltip>
							{/snippet}
							<input
								type="number"
								class="!w-24"
								bind:value={flowModule.priority}
								onfocus={bubble('focus')}
								onchange={() => {
									if (flowModule.priority && flowModule.priority > 100) {
										flowModule.priority = 100
									} else if (flowModule.priority && flowModule.priority < 0) {
										flowModule.priority = 0
									}
								}}
							/>
						</Label>
					{/if}
					{#if !$enterpriseLicense || isCloudHosted()}
						<Alert type="warning" title="Limitation" size="xs">
							Setting priority is only available for enterprise edition and not available on the
							cloud.
						</Alert>
					{/if}
				</div>
			</div>

			<div id="advanced-lifetime">
				<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
			</div>
		</div>

		<!-- Data -->
		<div class="flex flex-col gap-3">
			{@render groupTitle('Data')}
			<div id="advanced-cache">
				<FlowModuleCache bind:flowModule />
			</div>
			<div id="advanced-debounce">
				<FlowModuleDebounce bind:flowModule {selectedId} />
			</div>
			{#if isS3Lang}
				<div id="advanced-s3" class="flex flex-col gap-3">
					{@render rowLabel(
						'S3 snippets',
						'Read/Write objects from/to S3 and leverage Polars and DuckDB to run efficient ETL processes.'
					)}
					<div class="flex gap-2 justify-between items-center">
						<ToggleButtonGroup bind:selected={s3Kind} class="w-auto">
							{#snippet children({ item })}
								{#if flowModule.value['language'] === 'deno'}
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
							variant="border"
							on:click={() => editor?.setCode(s3Scripts[flowModule.value['language']][s3Kind])}
						>
							Apply snippet
						</Button>
					</div>
					<HighlightCode
						language={flowModule.value['language']}
						code={s3Scripts[flowModule.value['language']][s3Kind]}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>
