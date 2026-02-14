<script lang="ts">
	import type { Retry, FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { untrack, getContext } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import PropPickerWrapper from '$lib/components/flows/propPicker/PropPickerWrapper.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Section from '$lib/components/Section.svelte'
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

	let editor: SimpleEditor | undefined = $state(undefined)
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

<div class="h-full flex flex-col gap-4">
	<ToggleButtonGroup
		bind:selected={delayType}
		class={`${disabled ? 'disabled' : ''}`}
		on:selected={(e) => {
			flowModuleRetry = undefined
			if (e.detail === 'constant') {
				flowModuleRetry = undefined
				setConstantRetries()
			} else if (e.detail === 'exponential') {
				flowModuleRetry = undefined
				setExponentialRetries()
			}
		}}
	>
		{#snippet children({ item })}
			<ToggleButton value="disabled" label="Disabled" {item} />
			<ToggleButton value="constant" label="Constant" {item} />
			<ToggleButton value="exponential" label="Exponential" {item} />
		{/snippet}
	</ToggleButtonGroup>

	{#if delayType === 'constant' || delayType === 'exponential'}
		<Section label="Retry Condition" class="w-full">
			{#snippet header()}
				<Tooltip>
					Optional condition to determine when to retry. If not specified, will retry on any failure
					within the configured attempt limits.
				</Tooltip>
			{/snippet}

			<Toggle
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
					right: 'Only retry if condition is met'
				}}
			/>

			<div
				class="w-full border rounded-md p-2 mt-2 flex flex-col {flowModuleRetry?.retry_if
					? ''
					: 'bg-surface-secondary'}"
			>
				{#if flowModuleRetry?.retry_if}
					<span class="mt-2 text-xs font-bold">Retry condition expression</span>
					<span class="text-xs text-primary mb-2"
						>Expression should return true to retry, false to skip retry</span
					>
					<div class="border rounded-md overflow-auto w-full">
						{#if stepPropPicker}
							<PropPickerWrapper
								noPadding
								notSelectable
								pickableProperties={stepPropPicker.pickableProperties}
								{result}
								on:select={({ detail }) => {
									editor?.insertAtCursor(detail)
									editor?.focus()
								}}
							>
								<SimpleEditor
									bind:this={editor}
									lang="javascript"
									bind:code={flowModuleRetry.retry_if.expr}
									class="h-full"
									extraLib={`declare const result = ${JSON.stringify(result)};` +
										`\ndeclare const flow_input = ${JSON.stringify(stepPropPicker.pickableProperties.flow_input || {})};`}
								/>
							</PropPickerWrapper>
						{:else}
							<SimpleEditor
								bind:this={editor}
								lang="javascript"
								bind:code={flowModuleRetry.retry_if.expr}
								class="few-lines-editor"
								extraLib={`declare const result = ${JSON.stringify(result)};`}
							/>
						{/if}
					</div>
				{:else}
					<span class="mt-2 text-xs font-bold">Retry condition expression</span>
					<span class="text-xs text-primary mb-2"
						>Expression should return true to retry, false to skip retry</span
					>
					<textarea disabled rows="3" class="min-h-[80px]"></textarea>
				{/if}
			</div>
		</Section>
	{/if}

	<div class="flex h-[calc(100%-22px)]">
		<div class="w-1/2 h-full overflow-auto pr-2">
			{#if delayType === 'constant'}
				{#if flowModuleRetry?.constant}
					<div class="text-xs font-bold !mt-2">Attempts</div>
					<div class="flex gap-1">
						<input
							max={u32Max.toString()}
							bind:value={flowModuleRetry.constant.attempts}
							type="number"
						/>
						<button
							class="text-xs"
							onclick={() =>
								flowModuleRetry?.constant && (flowModuleRetry.constant.attempts = u32Max)}
							>max</button
						>
					</div>
					<div class="text-xs font-bold !mt-4">Delay</div>
					<SecondsInput bind:seconds={flowModuleRetry.constant.seconds} />
				{/if}
			{:else if delayType === 'exponential'}
				{#if flowModuleRetry?.exponential}
					<div class="text-xs font-bold !mt-2">Attempts</div>
					<div class="flex gap-1">
						<input max="100" bind:value={flowModuleRetry.exponential.attempts} type="number" />
						<button
							class="text-xs"
							onclick={() =>
								flowModuleRetry?.exponential && (flowModuleRetry.exponential.attempts = 100)}
							>max</button
						>
					</div>
					<div class="text-xs font-bold !mt-2">Multiplier</div>
					<span class="text-xs text-primary">delay = multiplier * base ^ (number of attempt)</span>
					<input bind:value={flowModuleRetry.exponential.multiplier} type="number" />
					<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
					<input
						bind:value={flowModuleRetry.exponential.seconds}
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
							disabled={!$enterpriseLicense}
							type="range"
							min={0}
							max={100}
							step={5}
							bind:value={flowModuleRetry.exponential.random_factor}
						/>
						<div class="w-20">
							<input
								disabled={true}
								bind:value={flowModuleRetry.exponential.random_factor}
								type="number"
								step={5}
								min={0}
								max={100}
							/>
						</div>
					</div>
				{/if}
			{/if}
		</div>
		<div class="w-1/2 h-full overflow-auto pl-2">
			{#if true}
				{@const { attempts: cAttempts, seconds: cSeconds } = flowModuleRetry?.constant || {}}
				{@const {
					attempts: eAttempts,
					seconds: eSeconds,
					multiplier,
					random_factor
				} = flowModuleRetry?.exponential || {}}
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
					{:else}
						<div class="text-xs">No retries</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
