<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	export let flowModule: FlowModule

	let delayType: 'disabled' | 'constant' | 'exponential'
	let loaded = false

	function setConstantRetries() {
		flowModule.retry = {
			...flowModule.retry,
			constant: {
				attempts: 1,
				seconds: 5
			}
		}
	}

	function setExponentialRetries() {
		flowModule.retry = {
			...flowModule.retry,
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
			(flowModule.retry?.constant?.attempts ?? 0) > 0
				? 'constant'
				: (flowModule.retry?.exponential?.attempts ?? 0) > 0
				? 'exponential'
				: 'disabled'
		loaded = true
	}

	$: !loaded && initialLoad()
</script>

<div class="h-full flex flex-col {$$props.class ?? ''}">
	<Section label="Retries">
		<svelte:fragment slot="header">
			<Tooltip documentationLink="https://www.windmill.dev/docs/flows/retries">
				If defined, upon error this step will be retried with a delay and a maximum number of
				attempts as defined below.
			</Tooltip>
		</svelte:fragment>

		<ToggleButtonGroup
			bind:selected={delayType}
			class="h-10"
			on:selected={(e) => {
				flowModule.retry = undefined
				if (e.detail === 'constant') {
					flowModule.retry = undefined
					setConstantRetries()
				} else if (e.detail === 'exponential') {
					flowModule.retry = undefined
					setExponentialRetries()
				}
			}}
		>
			<ToggleButton light value="disabled" label="Disabled" />
			<ToggleButton light value="constant" label="Constant" />
			<ToggleButton light value="exponential" label="Exponential" />
		</ToggleButtonGroup>
		<div class="flex h-[calc(100%-22px)]">
			<div class="w-1/2 h-full overflow-auto pr-2">
				{#if delayType === 'constant'}
					{#if flowModule.retry?.constant}
						<div class="text-xs font-bold !mt-2">Attempts</div>
						<input bind:value={flowModule.retry.constant.attempts} type="number" />
						<div class="text-xs font-bold !mt-2">Delay</div>
						<SecondsInput bind:seconds={flowModule.retry.constant.seconds} />
					{/if}
				{:else if delayType === 'exponential'}
					{#if flowModule.retry?.exponential}
						<div class="text-xs font-bold !mt-2">Attempts</div>
						<input bind:value={flowModule.retry.exponential.attempts} type="number" />
						<div class="text-xs font-bold !mt-2">Multiplier</div>
						<span class="text-xs text-tertiary"
							>delay = multiplier * base ^ (number of attempt)</span
						>
						<input bind:value={flowModule.retry.exponential.multiplier} type="number" />
						<div class="text-xs font-bold !mt-2">Base (in seconds)</div>
						<input bind:value={flowModule.retry.exponential.seconds} type="number" step="1" />
						<div class="text-xs font-bold !mt-2">Randomization factor (percentage)</div>
						<div class="flex w-full gap-4">
							<input
								type="range"
								min={0}
								max={100}
								step={5}
								bind:value={flowModule.retry.exponential.random_factor}
							/>
							<div class="w-20">
								<input
									disabled={true}
									bind:value={flowModule.retry.exponential.random_factor}
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
					{@const { attempts: cAttempts, seconds: cSeconds } = flowModule.retry?.constant || {}}
					{@const {
						attempts: eAttempts,
						seconds: eSeconds,
						multiplier,
						random_factor
					} = flowModule.retry?.exponential || {}}
					{@const cArray = Array.from({ length: cAttempts || 0 }, () => cSeconds)}
					{@const eArray = Array.from(
						{ length: eAttempts || 0 },
						(_, i) => (multiplier || 0) * (eSeconds || 0) ** (i + cArray.length + 1)
					)}
					{@const array = [...cArray, ...eArray]}
					<div class="bg-surface-secondary border rounded px-4 py-2">
						<div class="text-xs font-medium mb-2">Retry attempts</div>
						{#if array.length > 0}
							<table class="text-xs">
								<tr>
									<td class="font-semibold pr-1 pb-1">1:</td>
									<td class="pb-1"
										>After {array[0]} second{array[0] === 1 ? '' : 's'}
										{#if (random_factor ?? 0) > 0}(+/- {((array[0] ?? 0) * (random_factor ?? 0)) /
												100}
											seconds){/if}</td
									>
								</tr>
								{#each array.slice(1) as delay, i}
									{@const index = i + 2}
									<tr>
										<td class="font-semibold pr-1 align-top">{index}:</td>
										<td class="pb-1 whitespace-nowrap">
											{delay} second{delay === 1 ? '' : 's'}
											{#if (random_factor ?? 0) > 0}(+/- {((delay ?? 0) * (random_factor ?? 0)) /
													100} seconds){/if}
											after attempt #{index - 1}
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
	</Section>
</div>
