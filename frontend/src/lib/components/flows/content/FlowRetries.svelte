<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { flowStore } from '../flowStore'

	function setConstantRetries() {
		$flowStore.value.retry = {
			...$flowStore.value.retry,
			constant: {
				attempts: 0,
				seconds: 0
			}
		}
	}

	function setExpoentialRetries() {
		$flowStore.value.retry = {
			...$flowStore.value.retry,
			exponential: {
				attempts: 0,
				multiplier: 1,
				seconds: 0
			}
		}
	}

	$: isConstantRetryEnabled = Boolean($flowStore.value.retry?.constant)
	$: isExponentialRetryEnabled = Boolean($flowStore.value.retry?.exponential)
</script>

<div class="flex flex-col items-start space-y-1">
	<Toggle
		checked={isConstantRetryEnabled}
		on:change={() => {
			if (isConstantRetryEnabled && $flowStore.value.retry?.constant) {
				$flowStore.value.retry.constant = undefined
			} else {
				setConstantRetries()
			}
		}}
		options={{
			right: 'Constant retry enabled'
		}}
	/>
	{#if $flowStore.value.retry?.constant}
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={$flowStore.value.retry.constant.attempts} type="number" />
		<span class="text-xs font-bold">Delay</span>
		<input bind:value={$flowStore.value.retry.constant.seconds} type="number" />
	{:else}
		<span class="text-xs font-bold">Attempts</span>
		<input type="number" disabled />
		<span class="text-xs font-bold">Delay</span>
		<input type="number" disabled />
	{/if}

	<Toggle
		checked={isExponentialRetryEnabled}
		on:change={() => {
			if (isExponentialRetryEnabled && $flowStore.value.retry?.exponential) {
				$flowStore.value.retry.exponential = undefined
			} else {
				setExpoentialRetries()
			}
		}}
		options={{
			right: 'Exponential retry enabled'
		}}
	/>
	{#if $flowStore.value.retry?.exponential}
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={$flowStore.value.retry.exponential.attempts} type="number" />
		<span class="text-xs font-bold">Mulitplier</span>
		<input bind:value={$flowStore.value.retry.exponential.multiplier} type="number" />
		<span class="text-xs font-bold">Initial delay</span>
		<input bind:value={$flowStore.value.retry.exponential.seconds} type="number" />
	{:else}
		<span class="text-xs font-bold">Attempts</span>
		<input type="number" disabled />
		<span class="text-xs font-bold">Mulitplier</span>
		<input type="number" disabled />
		<span class="text-xs font-bold">Initial delay</span>
		<input type="number" disabled />
	{/if}
</div>
