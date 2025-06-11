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
	import Label from './Label.svelte'
	import MultiSelect from 'svelte-multiselect'

	export let config: AutoscalingConfig | undefined
	export let worker_tags: string[] | undefined

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
							integration: { type: 'dryrun' }
						}
					} else {
						config.enabled = true
					}
				} else {
					config = {
						...(config ?? {
							min_workers: 3,
							max_workers: 10,
							integration: { type: 'dryrun' }
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
				<div class="flex flex-col gap-2 text-2xs">
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Cooldown seconds after an incremental scale-in/out
						{#if config !== undefined}
							<input
								on:input={() => dispatch('dirty')}
								type="number"
								step="1"
								min="30"
								placeholder="300"
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
								bind:value={config.full_scale_cooldown_seconds}
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
								placeholder="max workers"
								bind:value={config.full_scale_jobs_waiting}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>
					<!-- svelte-ignore a11y-label-has-associated-control -->
					<label>
						Occupancy rate % threshold to go below to trigger a scale-in (decrease) <Tooltip
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
								placeholder="(max_workers - min_workers) / 5"
								bind:value={config.inc_num_workers}
							/>
						{:else}
							<input type="number" disabled />
						{/if}
					</label>

					<Label label="Custom tags to autoscale on">
						<svelte:fragment slot="header">
							<Tooltip>
								By default, autoscaling will apply to the tags the worker group is assigned to but
								you can override this here.
							</Tooltip>
						</svelte:fragment>
						{#if config}
							{#if config.custom_tags}
								<MultiSelect
									outerDivClass="text-secondary !bg-surface-disabled !border-0"
									selected={config.custom_tags}
									onchange={(e) => {
										console.log(e.type, config?.custom_tags)
										if (e && config?.custom_tags) {
											if (e.type === 'add') {
												config.custom_tags = [
													...config.custom_tags,
													...(e.option ? [e.option.toString()] : [])
												]
											} else if (e.type === 'remove') {
												config.custom_tags = config.custom_tags.filter((t) => t !== e.option)
												if (config?.custom_tags && config.custom_tags.length == 0) {
													config.custom_tags = undefined
												}
											} else if (e.type === 'removeAll') {
												config.custom_tags = undefined
											} else {
												console.error(`Priority tags multiselect - unknown event type: '${e.type}'`)
											}
											dispatch('dirty')
										}
									}}
									options={worker_tags ?? []}
									selectedOptionsDraggable={false}
									ulOptionsClass={'!bg-surface-secondary'}
									placeholder="Tags"
								/>
							{:else}
								<Button
									color="light"
									size="xs"
									variant="contained"
									on:click={() => {
										if (config) {
											config.custom_tags = []
											dispatch('dirty')
										}
									}}>Add custom tags</Button
								>
							{/if}
						{/if}
					</Label>
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
				className="mb-4 mt-2"
				let:item
			>
				<ToggleButton
					value="dryrun"
					label="Dry run"
					tooltip="See autoscaling events but not actual scaling actions will be performed"
					{item}
				/>
				<ToggleButton
					value="script"
					label="Custom script"
					tooltip="Run a custom script to scale your worker group"
					{item}
				/>
				<ToggleButton disabled value="ecs" label="ECS (soon)" {item} />
				<ToggleButton disabled value="nomad" label="Nomad (soon)" {item} />
				<ToggleButton disabled value="kubernetes" label="Kubernetes (soon)" {item} />
			</ToggleButtonGroup>

			{#if config.integration.type === 'script'}
				<label>
					Script path on the 'admins' workspace
					<input
						on:input={() => dispatch('dirty')}
						type="text"
						bind:value={config.integration.path}
					/>
				</label>

				<label>
					Custom tag for executing script (optional)
					{#if config.integration.tag}
						<input
							on:input={() => dispatch('dirty')}
							type="text"
							bind:value={config.integration.tag}
						/>
					{:else}
						<Button
							color="light"
							size="xs"
							variant="contained"
							on:click={() => {
								if (config?.integration?.type === 'script') {
									config.integration.tag = 'bash'
									dispatch('dirty')
								}
							}}>Set tag</Button
						>
					{/if}
				</label>

				<div class="flex mt-6 gap-2">
					<Button
						color="dark"
						target="_blank"
						endIcon={{ icon: ExternalLink }}
						href="/scripts/add?hub=hub%2F9204%2Fhelper%2FScale%20a%20worker%20group%20deployed%20as%20a%20kubernetes%20service&workspace=admins"
						>Create from template</Button
					>
					<Button
						color="dark"
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
			{/if}
		{:else}
			<ToggleButtonGroup selected={'script'} disabled className="mb-4 mt-2" let:item>
				<ToggleButton value="dryrun" label="Dry run" {item} />
				<ToggleButton value="script" label="Custom script" {item} />
				<ToggleButton value="ecs" label="ECS (soon)" {item} />
				<ToggleButton value="nomad" label="Nomad (soon)" {item} />
				<ToggleButton value="kubernetes" label="Kubernetes (soon)" {item} />
			</ToggleButtonGroup>

			<label>
				Script path on the 'admins' workspace
				<input type="text" disabled />
			</label>
		{/if}
	</div>
</div>
