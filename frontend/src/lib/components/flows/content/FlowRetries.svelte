<script lang="ts">
	import type { Retry } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle } from 'lucide-svelte'
	import { untrack } from 'svelte'

	interface Props {
		flowModuleRetry: Retry | undefined
		disabled?: boolean
	}

	let { flowModuleRetry = $bindable(), disabled = false }: Props = $props()

	let delayType = $state() as 'disabled' | 'constant' | 'exponential' | undefined
	let loaded = $state(false)

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

<div class="h-full flex flex-col">
	<ToggleButtonGroup
		bind:selected={delayType}
		class={`h-10 ${disabled ? 'disabled' : ''}`}
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
			<ToggleButton light value="disabled" label="Disabled" {item} />
			<ToggleButton light value="constant" label="Constant" {item} />
			<ToggleButton light value="exponential" label="Exponential" {item} />
		{/snippet}
	</ToggleButtonGroup>
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
					<div class="text-xs font-bold !mt-2">Delay</div>
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
					<span class="text-xs text-tertiary">delay = multiplier * base ^ (number of attempt)</span>
					<input bind:value={flowModuleRetry.exponential.multiplier} type="number" />
					<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
					<input bind:value={flowModuleRetry.exponential.seconds} type="number" step="1" />
					<div class="text-xs font-bold !mt-2">Randomization factor (percentage)</div>
					<div class="flex w-full gap-4">
						{#if !$enterpriseLicense}
							<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap">
								<AlertTriangle size={16} />
								EE only
							</div>{/if}
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
