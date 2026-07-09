<script lang="ts">
	import { getContext } from 'svelte'
	import { slide } from 'svelte/transition'
	import {
		RefreshCw,
		ShieldAlert,
		Timer,
		Gauge,
		ChevronsUp,
		Database,
		Combine,
		Trash2,
		SkipForward,
		CircleStop,
		Hand,
		Moon,
		ChevronRight
	} from 'lucide-svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { validateRetryConfig } from '$lib/utils'
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
	import FlowModuleDebounce from './FlowModuleDebounce.svelte'

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')

	interface Props {
		flowModule: FlowModule
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		selectedId: string
	}

	let {
		flowModule = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		selectedId
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
	const concurrencyOff = $derived(
		!$enterpriseLicense ||
			flowModule.value.type !== 'rawscript' ||
			!flowModule.value.concurrent_limit
	)

	type Summary = { text: string; state: 'configured' | 'default' | 'invalid'; mono?: boolean }
	const def = (text: string): Summary => ({ text, state: 'default' })
	const cfg = (text: string, mono = false): Summary => ({ text, state: 'configured', mono })

	function formatDur(s: number | undefined): string {
		if (s == null) return ''
		if (s < 60) return `${s}s`
		if (s < 3600) return `${Math.round(s / 60)} min`
		return `${Math.round(s / 3600)} h`
	}

	function exprSummary(expr: string | undefined): Summary {
		const e = expr?.trim()
		if (!e) return def('Off')
		return e.length <= 24 ? cfg(e, true) : cfg('Expression set')
	}

	const retriesSummary = $derived.by((): Summary => {
		const r = flowModule.retry
		const c = r?.constant?.attempts ?? 0
		const e = r?.exponential?.attempts ?? 0
		if (!c && !e) return def('None')
		if (validateRetryConfig(r)) return { text: 'Invalid', state: 'invalid' }
		return e
			? cfg(`${e} attempt${e > 1 ? 's' : ''}, exponential`)
			: cfg(`${c} attempt${c > 1 ? 's' : ''}, constant`)
	})

	const errorHandlingSummary = $derived(
		flowModule.continue_on_error ? cfg('Continue on error') : def('Off')
	)

	const timeoutSummary = $derived.by((): Summary => {
		const t = flowModule.timeout
		if (t == null) return def('None')
		if (typeof t === 'number') return cfg(formatDur(t))
		if (t.type === 'static') {
			const v = Number(t.value)
			return Number.isFinite(v) ? cfg(formatDur(v)) : cfg('Dynamic')
		}
		return cfg('Dynamic')
	})

	const concurrencySummary = $derived.by((): Summary => {
		if (flowModule.value.type !== 'rawscript' || !flowModule.value.concurrent_limit)
			return def('None')
		return cfg(
			`Max ${flowModule.value.concurrent_limit}${flowModule.value.custom_concurrency_key ? ' per key' : ''}`
		)
	})

	const prioritySummary = $derived(
		flowModule.priority && flowModule.priority > 0 ? cfg('High priority') : def('Off')
	)

	const cacheSummary = $derived(
		flowModule.cache_ttl ? cfg(formatDur(flowModule.cache_ttl)) : def('Off')
	)

	const debounceSummary = $derived.by((): Summary => {
		const d = flowModule.debouncing?.debounce_delay_s
		return d ? cfg(`${formatDur(d)} debounce`) : def('Off')
	})

	const lifetimeSummary = $derived.by((): Summary => {
		const s = flowModule.delete_after_secs
		if (s == null) return def('Off')
		return s === 0 ? cfg('Delete now') : cfg(`Delete after ${formatDur(s)}`)
	})

	const skipSummary = $derived(exprSummary(flowModule.skip_if?.expr))
	const earlyStopSummary = $derived(
		exprSummary(flowModule.stop_after_if?.expr ?? flowModule.stop_after_all_iters_if?.expr)
	)

	const suspendSummary = $derived.by((): Summary => {
		const su = flowModule.suspend
		if (!su) return def('Off')
		const n = su.required_events ?? 1
		return cfg(`${n} approval${n > 1 ? 's' : ''}`)
	})

	const sleepSummary = $derived.by((): Summary => {
		const s = flowModule.sleep
		if (!s) return def('Off')
		if (s.type === 'static') {
			const v = Number(s.value)
			return Number.isFinite(v) && v > 0 ? cfg(`${formatDur(v)} after`) : def('Off')
		}
		return cfg('Dynamic')
	})
</script>

{#snippet sectionHeader(title: string)}
	<div class="mb-2 text-[11px] font-medium uppercase tracking-[0.04em] text-tertiary">
		{title}
	</div>
{/snippet}

{#snippet rowHeader(key: string, Icon: any, label: string, summary: Summary)}
	<button
		type="button"
		aria-expanded={expanded === key}
		onclick={() => (expanded = expanded === key ? undefined : key)}
		class="flex w-full items-center gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-surface-hover"
	>
		<Icon size={16} class="shrink-0 text-secondary" />
		<div class="flex min-w-0 grow flex-col">
			<span class="text-xs font-normal leading-tight text-emphasis">{label}</span>
			{#if narrow && summary.state !== 'default'}
				<span
					class="mt-0.5 truncate text-xs font-normal leading-tight {summary.mono
						? 'font-mono'
						: ''} {summary.state === 'invalid' ? 'text-red-500' : 'text-accent'}"
				>
					{summary.text}
				</span>
			{/if}
		</div>
		{#if !narrow}
			<span
				class="min-w-0 truncate text-xs font-normal {summary.mono && summary.state === 'configured'
					? 'font-mono'
					: ''} {summary.state === 'configured'
					? 'text-accent'
					: summary.state === 'invalid'
						? 'text-red-500'
						: 'text-emphasis'}"
			>
				{summary.text}
			</span>
		{/if}
		<ChevronRight
			size={14}
			class="shrink-0 text-tertiary transition-transform duration-150 {expanded === key
				? 'rotate-90'
				: ''}"
		/>
	</button>
{/snippet}

<div bind:clientWidth={panelWidth} class="flex-1 min-h-0 overflow-auto p-4 flex flex-col gap-5">
	{#if !isFailure}
		<section>
			{@render sectionHeader('Flow control')}
			<div
				class="divide-y divide-border-light/50 overflow-hidden rounded-md border border-border-light/50 bg-surface-tertiary"
			>
				<div>
					{@render rowHeader('skip', SkipForward, 'Skip if', skipSummary)}
					{#if expanded === 'skip'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleSkip bind:flowModule {parentModule} {previousModule} />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader('early-stop', CircleStop, 'Early stop / break', earlyStopSummary)}
					{#if expanded === 'early-stop'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleEarlyStop bind:flowModule />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader('suspend', Hand, 'Suspend / approval', suspendSummary)}
					{#if expanded === 'suspend'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							<FlowModuleSuspend previousModuleId={previousModule?.id} bind:flowModule />
						</div>
					{/if}
				</div>

				<div>
					{@render rowHeader('sleep', Moon, 'Sleep', sleepSummary)}
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
			<div>
				{@render rowHeader('retries', RefreshCw, 'Retries', retriesSummary)}
				{#if expanded === 'retries'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowRetries bind:flowModuleRetry={flowModule.retry} bind:flowModule />
					</div>
				{/if}
			</div>

			<div>
				{@render rowHeader('error-handling', ShieldAlert, 'Error handling', errorHandlingSummary)}
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
				{@render rowHeader('timeout', Timer, 'Timeout', timeoutSummary)}
				{#if expanded === 'timeout'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowModuleTimeout previousModuleId={previousModule?.id} bind:flowModule />
					</div>
				{/if}
			</div>

			{#if isRawScript}
				<div>
					{@render rowHeader('concurrency', Gauge, 'Concurrency limit', concurrencySummary)}
					{#if expanded === 'concurrency'}
						<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
							{#if flowModule.value.type === 'rawscript'}
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
				{@render rowHeader('priority', ChevronsUp, 'Priority', prioritySummary)}
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
									Setting priority is only available for enterprise edition and not available on the
									cloud.
								</Alert>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<div>
				{@render rowHeader('cache', Database, 'Cache results', cacheSummary)}
				{#if expanded === 'cache'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowModuleCache bind:flowModule />
					</div>
				{/if}
			</div>

			<div>
				{@render rowHeader('debounce', Combine, 'Debounce', debounceSummary)}
				{#if expanded === 'debounce'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowModuleDebounce bind:flowModule {selectedId} />
					</div>
				{/if}
			</div>

			<div>
				{@render rowHeader('lifetime', Trash2, 'Lifetime', lifetimeSummary)}
				{#if expanded === 'lifetime'}
					<div class="px-3 pb-3 pt-1" transition:slide={{ duration: 120 }}>
						<FlowModuleDeleteAfterUse bind:flowModule disabled={!$enterpriseLicense} />
					</div>
				{/if}
			</div>
		</div>
	</section>
</div>
