<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule

	function setConstantRetries() {
		flowModule.retry = {
			...flowModule.retry,
			constant: {
				attempts: 0,
				seconds: 0
			}
		}
	}

	function setExpoentialRetries() {
		flowModule.retry = {
			...flowModule.retry,
			exponential: {
				attempts: 0,
				multiplier: 1,
				seconds: 0
			}
		}
	}

	$: isConstantRetryEnabled = Boolean(flowModule.retry?.constant)
	$: isExponentialRetryEnabled = Boolean(flowModule.retry?.exponential)
</script>

<div class="flex flex-col items-start space-y-1 {$$props.class}">
	<h2 class="mt-2"
		>Retries <Tooltip>
			If defined, upon error this step will be retried with a delay and a maximum number of attempts
			as defined below. If both static and exponential delay is defined, the static delay attempts
			are tried before the exponential ones.</Tooltip
		></h2
	>

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
	{#if flowModule.retry?.constant}
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={flowModule.retry.constant.attempts} type="number" />
		<span class="text-xs font-bold">Delay (seconds)</span>
		<input bind:value={flowModule.retry.constant.seconds} type="number" />
	{:else}
		<span class="text-xs font-bold">Attempts</span>
		<input type="number" disabled />
		<span class="text-xs font-bold">Delay (seconds)</span>
		<input type="number" disabled />
	{/if}

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
	{#if flowModule.retry?.exponential}
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={flowModule.retry.exponential.attempts} type="number" />
		<span class="text-xs font-bold">Mulitplier</span>
		<span class="text-xs text-gray-500">additional delay = multiplier * base ^ (# attempts)</span>
		<input bind:value={flowModule.retry.exponential.multiplier} type="number" />
		<span class="text-xs font-bold">Base</span>
		<input bind:value={flowModule.retry.exponential.seconds} type="number" />
	{:else}
		<span class="text-xs font-bold">Attempts</span>
		<input type="number" disabled />
		<span class="text-xs font-bold">Mulitplier</span>
		<span class="text-xs text-gray-500">additional delay = multiplier * seconds ^ (# attempts)</span
		>
		<input type="number" disabled />
		<span class="text-xs font-bold">Base</span>
		<input type="number" disabled />
	{/if}
</div>
