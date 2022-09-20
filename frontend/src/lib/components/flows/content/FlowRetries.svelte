<script lang="ts">
	import { flowStore } from '../flowStore'

	function setConstantRetries() {
		$flowStore.value.retry = {
			constant: {
				attempts: 5,
				seconds: 5
			}
		}
	}

	function setExpoentialRetries() {
		$flowStore.value.retry = {
			exponential: {
				attempts: 5,
				multiplier: 2,
				seconds: 5
			}
		}
	}

	function disableRetries() {
		$flowStore.value.retry = undefined
	}
</script>

<div class="flex flex-col items-start space-y-1">
	{#if $flowStore.value.retry?.constant}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => disableRetries()}>Disabled constant retries</button
		>
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={$flowStore.value.retry.constant.attempts} type="number" />
		<span class="text-xs font-bold">Seconds</span>

		<input bind:value={$flowStore.value.retry.constant.seconds} type="number" />
	{:else if $flowStore.value.retry?.exponential}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => disableRetries()}>Disabled exponential retries</button
		>
		<span class="text-xs font-bold">Attempts</span>
		<input bind:value={$flowStore.value.retry.exponential.attempts} type="number" />
		<span class="text-xs font-bold">Mulitplier</span>

		<input bind:value={$flowStore.value.retry.exponential.multiplier} type="number" />
		<span class="text-xs font-bold">Seconds</span>

		<input bind:value={$flowStore.value.retry.exponential.seconds} type="number" />
	{:else}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => setConstantRetries()}>Enable constant retries</button
		>
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => setExpoentialRetries()}>Enable exponential retries</button
		>
	{/if}
</div>
