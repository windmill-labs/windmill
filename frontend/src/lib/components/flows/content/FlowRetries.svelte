<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'

	export let flowModule: FlowModule

	function setConstantRetries() {
		flowModule.retry = {
			...flowModule.retry,
			constant: {
				attempts: 1,
				seconds: 5
			}
		}
	}

	function setExpoentialRetries() {
		flowModule.retry = {
			...flowModule.retry,
			exponential: {
				attempts: 1,
				multiplier: 1,
				seconds: 5
			}
		}
	}

	$: isConstantRetryEnabled = Boolean(flowModule.retry?.constant)
	$: isExponentialRetryEnabled = Boolean(flowModule.retry?.exponential)
</script>

<div class="h-full flex flex-col {$$props.class ?? ''}">
	<h2>
		Retries
		<Tooltip>
			If defined, upon error this step will be retried with a delay and a maximum number of attempts
			as defined below. If both static and exponential delay is defined, the static delay attempts
			are tried before the exponential ones.
		</Tooltip>
	</h2>

	<div class="flex h-[calc(100%-22px)]">
		<div class="w-1/2 h-full overflow-auto pr-2">
			<div class="pt-4">
				<Toggle
					checked={isConstantRetryEnabled}
					on:change={() => {
						if (isConstantRetryEnabled && flowModule.retry?.constant) {
							flowModule.retry.constant = undefined
						} else {
							setConstantRetries()
						}
					}}
					options={{
						right: 'Constant retry enabled'
					}}
				/>
			</div>
			{#if flowModule.retry?.constant}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<input bind:value={flowModule.retry.constant.attempts} type="number" />
				<div class="text-xs font-bold !mt-2">Delay</div>
				<SecondsInput bind:seconds={flowModule.retry.constant.seconds} />
			{:else}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<input type="number" disabled />
				<div class="text-xs font-bold !mt-2">Delay</div>
				<SecondsInput disabled />
			{/if}
			<div class="pt-6">
				<Toggle
					checked={isExponentialRetryEnabled}
					on:change={() => {
						if (isExponentialRetryEnabled && flowModule.retry?.exponential) {
							flowModule.retry.exponential = undefined
						} else {
							setExpoentialRetries()
						}
					}}
					options={{
						right: 'Exponential backoff enabled'
					}}
				/>
			</div>
			{#if flowModule.retry?.exponential}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<input bind:value={flowModule.retry.exponential.attempts} type="number" />
				<div class="text-xs font-bold !mt-2">Mulitplier</div>
				<span class="text-xs text-gray-500">delay = multiplier * base ^ (number of attempt)</span>
				<input bind:value={flowModule.retry.exponential.multiplier} type="number" />
				<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
				<input bind:value={flowModule.retry.exponential.seconds} type="number" />
			{:else}
				<div class="text-xs font-bold !mt-2">Attempts</div>
				<input type="number" disabled />
				<div class="text-xs font-bold !mt-2">Mulitplier</div>
				<span class="text-xs text-gray-500">delay = multiplier * base ^ (number of attempt)</span>
				<input type="number" disabled />
				<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
				<input type="number" disabled />
			{/if}
		</div>
		<div class="w-1/2 h-full overflow-auto pl-2">
			{#if true}
				{@const { attempts: cAttempts, seconds: cSeconds } = flowModule.retry?.constant || {}}
				{@const {
					attempts: eAttempts,
					seconds: eSeconds,
					multiplier
				} = flowModule.retry?.exponential || {}}
				{@const cArray = Array.from({ length: cAttempts || 0 }, () => cSeconds)}
				{@const eArray = Array.from(
					{ length: eAttempts || 0 },
					(_, i) => (multiplier || 0) * (eSeconds || 0) ** (i + cArray.length + 1)
				)}
				{@const array = [...cArray, ...eArray]}
				<div class="bg-gray-50 border border-gray-300 rounded px-4 py-2">
					<div class="text-xs font-medium mb-2">Retry attempts</div>
					{#if array.length > 0}
						<table class="text-xs">
							<tr>
								<td class="font-semibold pr-1 pb-1">1:</td>
								<td class="pb-1">After {array[0]} second{array[0] === 1 ? '' : 's'}</td>
							</tr>
							{#each array.slice(1) as delay, i}
								{@const index = i + 2}
								<tr>
									<td class="font-semibold pr-1 align-top">{index}:</td>
									<td class="pb-1 whitespace-nowrap">
										{delay} second{delay === 1 ? '' : 's'} after attempt #{index - 1}
										{#if i > cArray.length - 2}
											<span class="text-gray-400 pl-2">
												({multiplier} * {eSeconds}<sup>{index}</sup>)
											</span>
										{/if}
									</td>
								</tr>
							{/each}
						</table>
					{:else}
						<div class="text-xs">No retries</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
