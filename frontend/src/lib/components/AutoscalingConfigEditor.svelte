<script lang="ts">
	import type { AutoscalingConfig } from './worker_group'
	import Toggle from './Toggle.svelte'
	import Section from './Section.svelte'
	import Tooltip from './Tooltip.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Button } from './common'
	import { ExternalLink } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let config: AutoscalingConfig | undefined

	const dispatch = createEventDispatcher()
	let test_input: number = 3
</script>

<div class="flex flex-row gap-16 pt-2">
	<div class="space-y-4 flex flex-col gap-1 max-w-xs text-sm">
		<h5>Rules</h5>
		<Toggle
			checked={config?.enabled ?? false}
			options={{ right: 'Enabled' }}
			on:change={(e) => {
				dispatch('dirty')
				if (e.detail) {
					if (!config) {
						config = {
							enabled: true,
							min_workers: 3,
							max_workers: 10,
							integration: { type: 'script', path: '' }
						}
					} else {
						config.enabled = true
					}
				} else {
					config = {
						...(config ?? {
							min_workers: 3,
							max_workers: 10,
							integration: { type: 'script', path: '' }
						}),
						enabled: false
					}
				}
			}}
		/>
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label>
			Min # of Workers
			{#if config !== undefined}
				<input on:input={() => dispatch('dirty')} type="number" bind:value={config.min_workers} />
				{#if config.min_workers !== undefined && config.min_workers != undefined && config.min_workers > config.max_workers}
					<div class="text-red-600 text-xs whitespace-nowrap"
						>Minimum cannot be {'>'} to Maximum</div
					>
				{/if}
			{:else}
				<input type="number" disabled />
			{/if}
		</label>
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label>
			Max # of Workers
			{#if config !== undefined}
				<input on:input={() => dispatch('dirty')} type="number" bind:value={config.max_workers} />
			{:else}
				<input type="number" disabled />
			{/if}
		</label>
		<div class="p-2">
			<Section label="Advanced" small collapsable={true}>
				<div class="flex flex-col gap-1 text-xs">
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Cooldown seconds after an incremental scale-in/out
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="30"
								placeholder="60"
								bind:value={config.cooldown_seconds}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Cooldown seconds after a full scale out
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="30"
								placeholder="1500"
								bind:value={config.cooldown_seconds}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Num jobs waiting to trigger an incremental scale-out
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								bind:value={config.inc_scale_num_jobs_waiting}
								placeholder="1"
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>

					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Num jobs waiting to trigger a full scale out <Tooltip
							>Default: max_workers, full scale out = scale out to max workers</Tooltip
						>
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								bind:value={config.full_scale_jobs_waiting}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Occupancy rate % threhsold to go below to trigger a scale-in (decrease) <Tooltip
							>Default: 25%, need to go below average of all of 15s, 5m and 30m occupancy rates</Tooltip
						>
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="0"
								max="100"
								placeholder="25"
								bind:value={config.dec_scale_occupancy_rate}
							/>
						{:else}
							<input type="number" step="0.01" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Occupancy rate threshold to exceed to trigger an incremental scale-out (increase) <Tooltip
							>Default: 75%, need to exceed average of all of 15s, 5m and 30m occupancy rates</Tooltip
						>
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="0"
								max="100"
								placeholder="75"
								bind:value={config.inc_scale_occupancy_rate}
							/>
						{:else}
							<input type="number" step="0.01" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Num workers to scale-in/out by when incremental <Tooltip
							>Default: (max_workers - min_workers) / 5</Tooltip
						>
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="1"
								bind:value={config.inc_percent}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>
				</div>
			</Section>
		</div>
	</div>
	<div class="flex flex-col gap-1 max-w-xs text-sm">
		<h5>Integration</h5>
		{#if config?.integration}
			<ToggleButtonGroup
				on:selected={(e) => dispatch('dirty')}
				bind:selected={config.integration.type}
				class="mb-4 mt-2"
			>
				<ToggleButton
					value="script"
					size="sm"
					label="Custom Script"
					tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
				/>
				<ToggleButton position="center" disabled value="ecs" size="sm" label="ECS (soon)" />
				<ToggleButton position="right" disabled value="nomad" size="sm" label="Nomad (soon)" />
				<ToggleButton
					position="right"
					disabled
					value="kubernetes"
					size="sm"
					label="Kubernetes (soon)"
				/>
			</ToggleButtonGroup>

			<label>
				Script path on the 'admins' workspace
				{#if config.integration.type === 'script'}
					<input
						on:input={() => dispatch('dirty')}
						type="text"
						bind:value={config.integration.path}
					/>
				{/if}

				<div class="flex mt-6 gap-2">
					<Button
						endIcon={{ icon: ExternalLink }}
						href="/scripts/add?hub=hub%2F9088%2Fwindmill%2FHTTP%20route%20script%20with%20preprocessor%20template&workspace=admins"
						>Create from template</Button
					>
					<Button
						target="_blank"
						href={`/runs/${config.integration.path}?workspace=admins`}
						endIcon={{ icon: ExternalLink }}
					>
						See jobs
					</Button>
				</div>
				<div class="flex flex-row gap-2 mt-4">
					<Button color="light" size="xs" variant="contained">Test scaling</Button>
					<div class="flex text-xs flex-row gap-2 items-center">
						<input class="!w-16" type="number" bind:value={test_input} />
						workers
					</div>
				</div>
			</label>
		{:else}
			<ToggleButtonGroup selected={'script'} disabled class="mb-4 mt-2">
				<ToggleButton
					value="script"
					size="sm"
					label="Custom Script"
					tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
				/>
				<ToggleButton position="center" value="ecs" size="sm" label="ECS (soon)" />
				<ToggleButton position="right" value="nomad" size="sm" label="Nomad (soon)" />
				<ToggleButton position="right" value="kubernetes" size="sm" label="Kubernetes (soon)" />
			</ToggleButtonGroup>

			<label>
				Script path on the 'admins' workspace
				<input type="text" disabled />
			</label>
		{/if}
	</div>
</div>
