<script lang="ts">
	import { getContext } from 'svelte'
	import { slide } from 'svelte/transition'
	import { ChevronRight } from 'lucide-svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'

	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { SecondsInput } from '$lib/components/common'

	import FlowRetries from './FlowRetries.svelte'
	import FlowModuleEarlyStop from './FlowModuleEarlyStop.svelte'
	import FlowModuleSkip from './FlowModuleSkip.svelte'
	import FlowModuleSuspend from './FlowModuleSuspend.svelte'
	import FlowModuleSleep from './FlowModuleSleep.svelte'
	import FlowModuleTimeout from './FlowModuleTimeout.svelte'
	import FlowModuleDeleteAfterUse from './FlowModuleDeleteAfterUse.svelte'
	import FlowModuleCache from './FlowModuleCache.svelte'
	import { stepSettingsByKey, type StepSettingView } from '../flowStepSettings'
	import FlowModuleDebounce from './FlowModuleDebounce.svelte'
	import WorkspaceScriptSettingInfo from './WorkspaceScriptSettingInfo.svelte'

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		selectedId: string
		/** Lay out for embedding inside another scroll container (no own scroll/padding). */
		embedded?: boolean
		/** For loop modules: keep only the subset of settings loops support
		 *  (Flow control rows + Lifetime), hiding the rest of Execution policy. */
		loopSubset?: boolean
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
	}

	let {
		flowModule = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		selectedId,
		embedded = false,
		loopSubset = false,
		referencedConcurrentLimit = undefined,
		referencedConcurrencyTimeWindowS = undefined,
		workspaceScriptCacheTtl = undefined,
		loadingWorkspaceScript = false,
		workspaceScriptError = undefined,
		canEditWorkspaceScript = false,
		workspaceScriptNoEditReason = undefined,
		onEditWorkspaceScript
	}: Props = $props()

	// Accordion: at most one row open at a time.
	let expanded: string | undefined = $state()
	// Below this the value summary wraps to a second line instead of sitting inline.
	let panelWidth = $state(0)
	let narrow = $derived(panelWidth > 0 && panelWidth < 560)

	// Node-header quick-toggles open the tab then reveal the matching row.
	export function openSetting(key: string) {
		expanded = key === 'runtime' ? 'concurrency' : key
	}

	const isFailure = $derived(selectedId.includes('failure'))
	const isRawScript = $derived(flowModule.value.type === 'rawscript')
	const isWorkspaceScript = $derived(flowModule.value.type === 'script')
	const concurrencyOff = $derived(
		!$enterpriseLicense ||
			flowModule.value.type !== 'rawscript' ||
			!flowModule.value.concurrent_limit
	)

	// Every surface that answers "is this setting configured, and what is it called?"
	// reads flowStepSettings, so these rows and the graph badges cannot disagree.
	const settings = $derived(
		stepSettingsByKey(flowModule, {
			concurrent_limit: referencedConcurrentLimit,
			concurrency_time_window_s: referencedConcurrencyTimeWindowS,
			cache_ttl: workspaceScriptCacheTtl
		})
	)
</script>

{#snippet sectionHeader(title: string)}
	<div class="mb-2 text-[11px] font-medium uppercase tracking-[0.04em] text-tertiary">
		{title}
	</div>
{/snippet}

{#snippet rowHeader(s: StepSettingView | undefined)}
	{#if s}
		{@const Icon = s.icon}
	<button
		type="button"
		aria-expanded={expanded === s.key}
		onclick={() => (expanded = expanded === s.key ? undefined : s.key)}
		class="flex w-full items-center gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-surface-hover"
	>
		<Icon size={16} class="shrink-0 text-secondary" />
		<div class="flex min-w-0 grow flex-col">
			<span class="text-xs font-normal leading-tight text-emphasis">{s.label}</span>
			{#if narrow && s.summary.state !== 'default'}
				<span
					class="mt-0.5 truncate text-xs font-normal leading-tight {s.summary.mono
						? 'font-mono'
						: ''} {s.summary.state === 'invalid' ? 'text-red-500' : 'text-accent'}"
				>
					{s.summary.text}
				</span>
			{/if}
		</div>
		{#if !narrow}
			<span
				class="min-w-0 truncate text-xs font-normal {s.summary.mono && s.summary.state === 'configured'
					? 'font-mono'
					: ''} {s.summary.state === 'configured'
					? 'text-accent'
					: s.summary.state === 'invalid'
						? 'text-red-500'
						: 'text-emphasis'}"
			>
				{s.summary.text}
			</span>
		{/if}
		<ChevronRight
			size={14}
			class="shrink-0 text-tertiary transition-transform duration-150 {expanded === s.key
				? 'rotate-90'
				: ''}"
		/>
	</button>
	{/if}
{/snippet}

<div
	bind:clientWidth={panelWidth}
	class="flex flex-col gap-5 {embedded ? '' : 'flex-1 min-h-0 overflow-auto p-4'}"
>
	{#if !isFailure}
		<section>
			{@render sectionHeader('Flow control')}
			<div
				class="divide-y divide-border-light/50 overflow-hidden rounded-md border border-border-light/50 bg-surface-tertiary"
			>
				<div>
					{@render rowHeader(settings['skip'])}
					{#if expanded === 'skip'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['early-stop'])}
					{#if expanded === 'early-stop'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleEarlyStop bind:flowModule />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['suspend'])}
					{#if expanded === 'suspend'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['sleep'])}
					{#if expanded === 'sleep'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleSleep previousModuleId={previousModule?.id} bind:flowModule />
						</div>
					{/if}
				</div>
			</div>
		</section>
	{/if}

	<section>
		{@render sectionHeader('Execution policy')}
		<div
			class="divide-y divide-border-light/50 overflow-hidden rounded-md border border-border-light/50 bg-surface-tertiary"
		>
			{#if !loopSubset}
				<div>
					{@render rowHeader(settings['retries'])}
					{#if expanded === 'retries'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowRetries bind:flowModuleRetry={flowModule.retry} bind:flowModule />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['error-handling'])}
					{#if expanded === 'error-handling'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
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
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['timeout'])}
					{#if expanded === 'timeout'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleTimeout previousModuleId={previousModule?.id} bind:flowModule />
						</div>
					{/if}
				</div>

				{#if isRawScript || isWorkspaceScript}
					<div>
						{@render rowHeader(settings['concurrency'])}
						{#if expanded === 'concurrency'}
							<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
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
									<div class="flex flex-col gap-3">
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
												rightDocumentationLink:
													'https://www.windmill.dev/docs/flows/concurrency_limit'
											}}
										/>
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
												disabled={concurrencyOff}
												bind:seconds={flowModule.value.concurrency_time_window_s}
												clearable
											/>
										</Label>
										<Label label="Custom concurrency key (optional)">
											{#snippet header()}
												<Tooltip>
													Concurrency keys are global, you can have them be workspace specific using
													the variable `$workspace`. You can also use an argument's value using
													`$args[name_of_arg]`</Tooltip
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

				<div>
					{@render rowHeader(settings['priority'])}
					{#if expanded === 'priority'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
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
								<Label label="Priority number">
									{#snippet header()}
										<Tooltip>The higher the number, the higher the priority.</Tooltip>
									{/snippet}
									<input
										type="number"
										class="!w-24"
										disabled={flowModule.priority === undefined}
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
								{#if !$enterpriseLicense || isCloudHosted()}
									<Alert type="warning" title="Limitation" size="xs">
										Setting priority is only available for enterprise edition and not available on
										the cloud.
									</Alert>
								{/if}
							</div>
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['cache'])}
					{#if expanded === 'cache'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
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
					{/if}
				</div>

				<div>
					{@render rowHeader(settings['debounce'])}
					{#if expanded === 'debounce'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleDebounce bind:flowModule {selectedId} />
						</div>
					{/if}
				</div>
			{/if}

			<div>
				{@render rowHeader(settings['lifetime'])}
				{#if expanded === 'lifetime'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
					</div>
				{/if}
			</div>
		</div>
	</section>
</div>
