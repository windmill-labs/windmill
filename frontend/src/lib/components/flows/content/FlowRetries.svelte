<script lang="ts">
	import type { Retry, FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { untrack, getContext } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import FlowExpressionEditor from './FlowExpressionEditor.svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'
	import { NEVER_TESTED_THIS_FAR } from '../models'
	import { validateRetryConfig } from '$lib/utils'
	import EEOnly from '$lib/components/EEOnly.svelte'

	interface Props {
		flowModuleRetry: Retry | undefined
		disabled?: boolean
		flowModule?: FlowModule
	}

	let {
		flowModule = $bindable(),
		flowModuleRetry = $bindable(),
		disabled = false
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	const { flowStateStore, flowStore, previewArgs } = flowEditorContext || {
		flowStateStore: null,
		flowStore: null,
		previewArgs: null
	}

	let delayType = $state() as 'disabled' | 'constant' | 'exponential' | undefined
	let loaded = $state(false)

	let stepPropPicker = $derived(
		flowModule && flowStateStore?.val && flowStore?.val && previewArgs?.val
			? getStepPropPicker(
					flowStateStore.val,
					undefined,
					undefined,
					flowModule.id,
					flowStore.val,
					previewArgs.val,
					false
				)
			: null
	)

	let isRetryConditionEnabled = $derived(Boolean(flowModuleRetry?.retry_if))
	// When retries are toggled off the config is still shown (read-only) so the row
	// isn't empty: fall back to sensible defaults and force the "constant" tab.
	let retriesOff = $derived(!(delayType === 'constant' || delayType === 'exponential'))
	let cfgDisabled = $derived(disabled || retriesOff)
	let displayDelayType = $derived(delayType === 'exponential' ? 'exponential' : 'constant')
	let displayRetry = $derived(
		flowModuleRetry ?? {
			constant: { attempts: 1, seconds: 5 },
			exponential: { attempts: 1, multiplier: 1, seconds: 5, random_factor: 0 }
		}
	)
	// Always-defined sub-configs so the read-only off-state can bind without
	// undefined checks; when the real config exists these are the same refs.
	let displayConstant = $derived(displayRetry.constant ?? { attempts: 1, seconds: 5 })
	let displayExponential = $derived(
		displayRetry.exponential ?? { attempts: 1, multiplier: 1, seconds: 5, random_factor: 0 }
	)
	// Only feed the preview the branch that's actually shown, so the off-state
	// defaults don't render a bogus combined schedule.
	let previewRetry = $derived(
		retriesOff
			? displayDelayType === 'constant'
				? { constant: displayRetry.constant }
				: { exponential: displayRetry.exponential }
			: flowModuleRetry
	)
	let result = $derived(
		flowModule && flowStateStore?.val
			? (flowStateStore.val[flowModule.id]?.previewResult ?? NEVER_TESTED_THIS_FAR)
			: NEVER_TESTED_THIS_FAR
	)

	let validationError = $derived.by(() => {
		return validateRetryConfig(flowModuleRetry)
	})

	function setConstantRetries() {
		flowModuleRetry = {
			...flowModuleRetry,
			constant: {
				attempts: 1,
				seconds: 5
			}
		}
	}

	function setExponentialRetries() {
		flowModuleRetry = {
			...flowModuleRetry,
			exponential: {
				attempts: 1,
				multiplier: 1,
				seconds: 5,
				random_factor: 0
			}
		}
	}

	function initialLoad() {
		delayType =
			(flowModuleRetry?.constant?.attempts ?? 0) > 0
				? 'constant'
				: (flowModuleRetry?.exponential?.attempts ?? 0) > 0
					? 'exponential'
					: 'disabled'
		loaded = true
	}

	function resetDelayType() {
		delayType = 'disabled'
	}

	$effect(() => {
		flowModuleRetry === undefined && resetDelayType()
	})
	$effect(() => {
		!loaded && untrack(() => initialLoad())
	})

	const u32Max = 4294967295
</script>

<div class="flex flex-col gap-3">
	<Toggle
		size="xs"
		textClass="text-xs font-normal text-primary"
		{disabled}
		checked={delayType === 'constant' || delayType === 'exponential'}
		on:change={() => {
			if (delayType === 'constant' || delayType === 'exponential') {
				flowModuleRetry = undefined
				delayType = 'disabled'
			} else {
				setConstantRetries()
				delayType = 'constant'
			}
		}}
		options={{
			right: 'Retry this step on failure',
			rightTooltip:
				'Upon error this step is retried with a delay and a maximum number of attempts as defined below.',
			rightDocumentationLink: 'https://www.windmill.dev/docs/flows/retries'
		}}
	/>

	<ToggleButtonGroup
		selected={retriesOff ? displayDelayType : delayType}
		disabled={cfgDisabled}
		class={`${cfgDisabled ? 'disabled' : ''}`}
		on:selected={(e) => {
			flowModuleRetry = undefined
			if (e.detail === 'constant') {
				setConstantRetries()
				delayType = 'constant'
			} else if (e.detail === 'exponential') {
				setExponentialRetries()
				delayType = 'exponential'
			}
		}}
	>
		{#snippet children({ item })}
			<ToggleButton value="constant" label="Constant" {item} />
			<ToggleButton value="exponential" label="Exponential" {item} />
		{/snippet}
	</ToggleButtonGroup>

	<div class="flex flex-col gap-2">
		<Toggle
			size="xs"
			textClass="text-xs font-normal text-primary"
			disabled={cfgDisabled}
			checked={isRetryConditionEnabled}
			on:change={() => {
				if (!flowModuleRetry) {
					return
				}
				if (isRetryConditionEnabled && flowModuleRetry.retry_if) {
					const { retry_if, ...rest } = flowModuleRetry
					flowModuleRetry = rest
				} else {
					flowModuleRetry = {
						...flowModuleRetry,
						retry_if: {
							expr: 'error && error.name !== "PERMANENT_FAILURE"'
						}
					}
				}
			}}
			options={{
				right: 'Only retry if condition is met',
				rightTooltip:
					'Optional condition to determine when to retry. If not specified, retries on any failure within the configured attempt limits.'
			}}
		/>

		<div>
			{#if stepPropPicker}
				<FlowExpressionEditor
					forceCollapsePicker
					disabled={cfgDisabled || !isRetryConditionEnabled}
					label="Retry condition expression"
					bind:code={
						() => flowModuleRetry?.retry_if?.expr ?? '',
						(v) => {
							if (flowModuleRetry?.retry_if) flowModuleRetry.retry_if.expr = v
						}
					}
					pickableProperties={stepPropPicker.pickableProperties}
					{result}
					extraLib={`declare const result = ${JSON.stringify(result)};` +
						`\ndeclare const flow_input = ${JSON.stringify(stepPropPicker.pickableProperties.flow_input || {})};`}
				>
					{#snippet tooltip()}
						Expression should return true to retry, false to skip retry.
					{/snippet}
				</FlowExpressionEditor>
			{:else if flowModuleRetry?.retry_if}
				<div class="border rounded-md overflow-auto w-full">
					<SimpleEditor
						lang="javascript"
						bind:code={flowModuleRetry.retry_if.expr}
						class="few-lines-editor"
						extraLib={`declare const result = ${JSON.stringify(result)};`}
					/>
				</div>
			{/if}
		</div>
	</div>

	<div class="flex h-[calc(100%-22px)]">
		<div class="w-1/2 h-full overflow-auto pr-2">
			{#if displayDelayType === 'constant'}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<div class="flex gap-1">
					<input
						max={u32Max.toString()}
						disabled={cfgDisabled}
						bind:value={displayConstant.attempts}
						type="number"
					/>
					<button
						class="text-xs"
						disabled={cfgDisabled}
						onclick={() =>
							flowModuleRetry?.constant && (flowModuleRetry.constant.attempts = u32Max)}>max</button
					>
				</div>
				<div class="text-xs font-bold !mt-4">Delay</div>
				<SecondsInput disabled={cfgDisabled} bind:seconds={displayConstant.seconds} />
			{:else if displayDelayType === 'exponential'}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<div class="flex gap-1">
					<input
						max="100"
						disabled={cfgDisabled}
						bind:value={displayExponential.attempts}
						type="number"
					/>
					<button
						class="text-xs"
						disabled={cfgDisabled}
						onclick={() =>
							flowModuleRetry?.exponential && (flowModuleRetry.exponential.attempts = 100)}
						>max</button
					>
				</div>
				<div class="text-xs font-bold !mt-2">Multiplier</div>
				<span class="text-xs text-primary">delay = multiplier * base ^ (number of attempt)</span>
				<input
					disabled={cfgDisabled}
					bind:value={displayExponential.multiplier}
					type="number"
				/>
				<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
				<input
					disabled={cfgDisabled}
					bind:value={displayExponential.seconds}
					type="number"
					step="1"
					min="0"
					class={validationError ? 'border-red-500' : ''}
				/>
				{#if validationError}
					<span class="text-xs text-red-500">{validationError}</span>
				{:else}
					<span class="text-xs text-tertiary"
						>Must be ≥ 1. A base of 0 would cause immediate retries.</span
					>
				{/if}
				<div class="text-xs font-bold !mt-2">Randomization factor (percentage)</div>
				<div class="flex w-full gap-4">
					{#if !$enterpriseLicense}
						<EEOnly />
					{/if}
					<input
						disabled={cfgDisabled || !$enterpriseLicense}
						type="range"
						min={0}
						max={100}
						step={5}
						bind:value={displayExponential.random_factor}
					/>
					<div class="w-20">
						<input
							disabled={true}
							bind:value={displayExponential.random_factor}
							type="number"
							step={5}
							min={0}
							max={100}
						/>
					</div>
				</div>
			{/if}
		</div>
		<div class="w-1/2 h-full overflow-auto pl-2">
			{#if true}
				{@const { attempts: cAttempts, seconds: cSeconds } = previewRetry?.constant || {}}
				{@const {
					attempts: eAttempts,
					seconds: eSeconds,
					multiplier,
					random_factor
				} = previewRetry?.exponential || {}}
				{@const cArray = Array.from({ length: Math.min(cAttempts || 0, 100) }, () => cSeconds)}
				{@const eArray = Array.from(
					{ length: Math.min(eAttempts || 0, 100) },
					(_, i) => (multiplier || 0) * (eSeconds || 0) ** (i + cArray.length + 1)
				)}
				{@const array = [...cArray, ...eArray]}
				<div class="bg-surface-secondary border rounded px-4 py-2">
					<div class="text-xs font-medium mb-2">Retry attempts</div>
					{#if array.length > 0}
						<table class="text-xs">
							<thead>
								<tr>
									<td class="font-semibold pr-1 pb-1">1:</td>
									<td class="pb-1"
										>After {array[0]} second{array[0] === 1 ? '' : 's'}
										{#if (random_factor ?? 0) > 0}(+/- {((array[0] ?? 0) * (random_factor ?? 0)) /
												100}
											seconds){/if}</td
									>
								</tr>
							</thead>
							<tbody>
								{#each array.slice(1, 100) as delay, i}
									{@const index = i + 2}
									<tr>
										<td class="font-semibold pr-1 align-top">{index}:</td>
										<td class="pb-1 whitespace-nowrap">
											{delay} second{delay === 1 ? '' : 's'}
											{#if (random_factor ?? 0) > 0}(+/- {((delay ?? 0) * (random_factor ?? 0)) /
													100}
												seconds){/if}
											after attempt #{index - 1}
											{#if i > cArray.length - 2}
												<span class="text-gray-400 pl-2">
													({multiplier} * {eSeconds}<sup>{index}</sup>)
												</span>
											{/if}
										</td>
									</tr>
								{/each}
								{#if (cAttempts ?? 0) > 100 || (eAttempts ?? 0) > 100}
									<tr>
										<td class="font-semibold pr-1 align-top">...</td>
										<td class="pb-1">...</td>
									</tr>
								{/if}
							</tbody>
						</table>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
