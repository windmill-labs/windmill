<script lang="ts">
	import type { AutoscalingConfig } from './worker_group'
	import Toggle from './Toggle.svelte'
	import Section from './Section.svelte'
	import Tooltip from './Tooltip.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Alert, Button } from './common'
	import { ExternalLink } from 'lucide-svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Label from './Label.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { ConfigService } from '$lib/gen'
	import Select from './select/Select.svelte'
	import ScriptPicker from './ScriptPicker.svelte'
	import Badge from './common/badge/Badge.svelte'

	interface Props {
		config: AutoscalingConfig | undefined
		worker_tags: string[] | undefined
		disabled: boolean
		eeOnly?: boolean
	}

	let { config = $bindable(), worker_tags, disabled, eeOnly = false }: Props = $props()
	let test_input: number = $state(3)
	let healthCheckLoading: boolean = $state(false)
	let healthCheckResult: { success: boolean; error?: string } | null = $state(null)

	function validateMinMax(): string | undefined {
		if (config?.min_workers && config?.max_workers && config.min_workers > config.max_workers) {
			return 'Minimum cannot be greater than maximum'
		}
		return undefined
	}

	async function checkKubernetesHealth() {
		if (!config?.integration || config.integration.type !== 'kubernetes') return

		healthCheckLoading = true
		healthCheckResult = null

		try {
			await ConfigService.nativeKubernetesAutoscalingHealthcheck()
			healthCheckResult = { success: true }
		} catch (error: any) {
			healthCheckResult = {
				success: false,
				error: error.body || error.message || 'Unknown error'
			}
		} finally {
			healthCheckLoading = false
		}
	}

	let collapsed: boolean = $state(true)
</script>

<Section
	label="Autoscaling"
	collapsable
	class="flex flex-col gap-6"
	bind:collapsed
	description="Autoscaling automatically adjusts the number of workers based on your workload demands."
	{eeOnly}
>
	{#snippet labelExtra()}
		<Badge color="gray">Beta</Badge>
	{/snippet}
	{#snippet header()}
		<div class="ml-2">
			<Toggle
				checked={config?.enabled ?? false}
				options={{ right: 'Enabled' }}
				{disabled}
				on:change={(e) => {
					if (e.detail) {
						collapsed = false
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
		</div>
	{/snippet}

	<div class="flex flex-row gap-4">
		<Label label="Min # of workers" disabled={config === undefined} class="grow min-w-0">
			<span class="text-xs text-secondary">The minimum number of workers to scale down to</span>
			{#if config !== undefined}
				<input
					type="number"
					min="1"
					class="rounded-md border border-border-light text-xs text-primary font-normal bg-surface-input px-2 py-1 focus:border-border-selected hover:border-border-selected/50 disabled:bg-surface-disabled disabled:border-transparent disabled:text-disabled"
					bind:value={config.min_workers}
					{disabled}
				/>
				{#if validateMinMax()}
					<div class="text-2xs text-red-500 font-normal mt-1">
						{validateMinMax()}
					</div>
				{/if}
			{:else}
				<input type="number" {disabled} placeholder="3" />
			{/if}
		</Label>
		<Label label="Max # of workers" disabled={config === undefined} class="grow min-w-0">
			<span class="text-xs text-secondary">The maximum number of workers to scale up to</span>
			{#if config !== undefined}
				<input
					type="number"
					min="1"
					class="rounded-md border border-border-light text-xs text-primary font-normal bg-surface-input px-2 py-1 focus:border-border-selected hover:border-border-selected/50 disabled:bg-surface-disabled disabled:border-transparent disabled:text-disabled"
					bind:value={config.max_workers}
					{disabled}
				/>
			{:else}
				<input type="number" disabled placeholder="10" />
			{/if}
		</Label>
	</div>

	<Label label="Integration">
		<span class="text-xs text-secondary">Choose how to autoscale your worker group</span>
		{#if config?.integration}
			<div class="flex flex-col gap-2">
				<ToggleButtonGroup bind:selected={config.integration.type} {disabled}>
					{#snippet children({ item })}
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
						<ToggleButton value="kubernetes" label="Kubernetes" {item} />
					{/snippet}
				</ToggleButtonGroup>

				{#if config.integration.type === 'script'}
					<div class="flex flex-col gap-6 p-4 rounded-md border border-border-light">
						<Label label="Script path" required>
							<div class="flex flex-row gap-2">
								<ScriptPicker
									itemKind="script"
									bind:scriptPath={
										() => config?.integration?.['path'] ?? undefined,
										(v) => {
											if (!config || !config.integration) return

											if (!v || v === '') {
												delete config.integration['path']
											} else {
												config.integration['path'] = v
											}
										}
									}
									clearable
									{disabled}
								/>

								{#if config?.integration?.['path'] === undefined || config?.integration?.['path'] === ''}
									<Button
										variant="default"
										target="_blank"
										endIcon={{ icon: ExternalLink }}
										href="/scripts/add?hub=hub%2F9204%2Fhelper%2FScale%20a%20worker%20group%20deployed%20as%20a%20kubernetes%20service&workspace=admins"
										>Create from template
										{disabled}
									</Button>
								{:else}
									<Button
										variant="default"
										target="_blank"
										href={`/runs/${config.integration.path}?workspace=admins`}
										endIcon={{ icon: ExternalLink }}
										{disabled}
									>
										See jobs
									</Button>
								{/if}
							</div>
							<span class="text-2xs text-hint">Script must be in the 'admins' workspace</span>
						</Label>

						<Label
							label="Custom tag for executing script"
							tooltip="Optional tag to specify worker capabilities required for this script"
							for="custom_tag_select"
						>
							<Select
								clearable
								id="custom_tag_select"
								disabled={!config || !config.integration || disabled}
								bind:value={
									() => config?.integration?.['tags'] ?? undefined,
									(v) => {
										if (!config || !config.integration) return
										if (!v || v === '') {
											delete config.integration['tags']
										} else {
											config.integration['tags'] = v
										}
									}
								}
								items={safeSelectItems(worker_tags)}
							/>

							<div class="flex flex-row gap-2 justify-end mt-4">
								<Button variant="default" unifiedSize="md">Test scaling</Button>
								<div class="flex text-xs flex-row gap-2 items-center">
									<input class="!w-16" type="number" bind:value={test_input} />
									workers
								</div>
							</div>
						</Label>
					</div>
				{:else if config.integration.type === 'kubernetes'}
					<div class="flex flex-col gap-3 p-4 border border-border-light rounded-md">
						<div class="text-xs text-secondary mb-2">
							Kubernetes configuration is automatically inferred from the cluster environment. The
							worker group name and namespace will be detected automatically.
						</div>

						<div class="flex flex-col gap-2">
							<div class="flex items-center gap-2 justify-between">
								<div class="flex flex-row gap-2">
									<Button
										unifiedSize="md"
										variant="default"
										endIcon={{ icon: ExternalLink }}
										href="https://windmill.dev/docs/core_concepts/autoscaling#kubernetes"
										target="_blank"
									>
										Setup Guide (Roles & Bindings)
									</Button>
									<Button
										unifiedSize="md"
										variant="default"
										onclick={checkKubernetesHealth}
										disabled={healthCheckLoading}
									>
										{healthCheckLoading ? 'Checking...' : 'Check Health'}
									</Button>
								</div>

								<div class="flex flex-row gap-2 justify-end">
									<Button unifiedSize="md" variant="default">Test scaling</Button>
									<div class="flex text-xs flex-row gap-2 items-center">
										<input class="!w-16" type="number" bind:value={test_input} />
										workers
									</div>
								</div>
							</div>

							{#if healthCheckResult !== null}
								<Alert
									type={healthCheckResult.success ? 'success' : 'error'}
									title={healthCheckResult.success ? 'Health check passed' : 'Health check failed'}
								>
									{#if healthCheckResult.success}
										Kubernetes autoscaling is healthy
									{:else}
										{healthCheckResult.error}
										{#if healthCheckResult.error?.includes('permissions') || healthCheckResult.error?.includes('role')}
											<br /><small
												>Please follow the setup guide above to configure proper RBAC permissions.</small
											>
										{/if}
									{/if}
								</Alert>
							{/if}
						</div>
					</div>
				{:else if config.integration.type === 'dryrun'}
					<div class="p-4 border border-border-light rounded-md">
						<span class="text-xs text-secondary">
							In dry run mode, autoscaling will be simulated and events will be logged but no actual
							scaling will be performed.
						</span>
					</div>
				{/if}
			</div>
		{:else}
			<ToggleButtonGroup selected={'script'} disabled class="mb-4 mt-2">
				{#snippet children({ item })}
					<ToggleButton value="dryrun" label="Dry run" {item} />
					<ToggleButton value="script" label="Custom script" {item} />
					<ToggleButton value="ecs" label="ECS (soon)" {item} />
					<ToggleButton value="nomad" label="Nomad (soon)" {item} />
					<ToggleButton value="kubernetes" label="Kubernetes" {item} />
				{/snippet}
			</ToggleButtonGroup>

			<Label label="Script path on the 'admins' workspace" for="script_path">
				<TextInput
					inputProps={{
						disabled: true,
						id: 'script_path',
						placeholder: 'e.g. f/scaling/scale_worker_group'
					}}
				/>
			</Label>
		{/if}
	</Label>

	<Section label="Advanced" small collapsable={true} class="flex flex-col gap-6">
		<Label
			label="Cooldown seconds after incremental scale-in/out"
			disabled={config === undefined || disabled}
			tooltip="Time to wait between incremental scaling operations"
		>
			{#if config !== undefined}
				<input
					type="number"
					step="1"
					min="30"
					placeholder="300"
					class="rounded-md border border-border-light text-xs text-primary font-normal bg-surface-input px-2 py-1 focus:border-border-selected hover:border-border-selected/50"
					bind:value={config.cooldown_seconds}
					{disabled}
				/>
			{:else}
				<input type="number" disabled />
			{/if}
		</Label>
		<Label
			label="Cooldown seconds after full scale out"
			disabled={config === undefined || disabled}
			tooltip="Time to wait after scaling to maximum capacity"
		>
			{#if config !== undefined}
				<input
					type="number"
					step="1"
					min="30"
					placeholder="1500"
					class="rounded-md border border-border-light text-xs text-primary font-normal bg-surface-input px-2 py-1 focus:border-border-selected hover:border-border-selected/50"
					bind:value={config.full_scale_cooldown_seconds}
					{disabled}
				/>
			{:else}
				<input
					type="number"
					disabled
					class="rounded-md border border-border-light text-xs font-normal bg-surface-disabled border-transparent text-disabled px-2 py-1"
				/>
			{/if}
		</Label>
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<Label label="Num jobs waiting to trigger an incremental scale-out">
			{#if config !== undefined}
				<input
					type="number"
					bind:value={config.inc_scale_num_jobs_waiting}
					placeholder="1"
					{disabled}
				/>
			{:else}
				<input type="number" disabled />
			{/if}
		</Label>

		<!-- svelte-ignore a11y_label_has_associated_control -->
		<Label
			label="Num jobs waiting to trigger a full scale out"
			tooltip="Default: max_workers, full scale out = scale out to max workers"
			for="full_scale_jobs_waiting"
		>
			{#if config !== undefined}
				<input
					type="number"
					placeholder="max workers"
					bind:value={config.full_scale_jobs_waiting}
					id="full_scale_jobs_waiting"
					{disabled}
				/>
			{:else}
				<input type="number" disabled />
			{/if}
		</Label>
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<div class="flex flex-row gap-4">
			<Label
				label="Min occupancy rate"
				tooltip="Default: 25%, need to go below average of all of 15s, 5m and 30m occupancy rates"
				for="occupancy_rate_min"
				class="grow min-w-0"
			>
				<span class="text-xs text-secondary"
					>{`Threshold (%) to go below to trigger a scale-in (decrease)`}</span
				>
				{#if config !== undefined}
					<input
						type="number"
						step="1"
						min="0"
						max="100"
						placeholder="25"
						id="occupancy_rate_min"
						bind:value={config.dec_scale_occupancy_rate}
						{disabled}
					/>
				{:else}
					<input type="number" step="0.01" disabled />
				{/if}
			</Label>
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<Label
				label="Max occupancy rate"
				tooltip="Default: 75%, need to exceed average of all of 15s, 5m and 30m occupancy rates"
				for="occupancy_rate_max"
				class="grow min-w-0"
			>
				<span class="text-xs text-secondary"
					>{`Threshold (%) to exceed to trigger a scale-out (increase)`}</span
				>
				{#if config !== undefined}
					<input
						type="number"
						step="1"
						min="0"
						max="100"
						placeholder="75"
						id="occupancy_rate_max"
						bind:value={config.inc_scale_occupancy_rate}
						{disabled}
					/>
				{:else}
					<input type="number" step="0.01" disabled />
				{/if}
			</Label>
		</div>

		<!-- svelte-ignore a11y_label_has_associated_control -->
		<Label
			label="Num workers to scale-in/out by when incremental"
			tooltip="Default: (max_workers - min_workers) / 5"
		>
			{#if config !== undefined}
				<input
					type="number"
					step="1"
					min="1"
					placeholder="(max_workers - min_workers) / 5"
					bind:value={config.inc_num_workers}
					{disabled}
				/>
			{:else}
				<input type="number" disabled />
			{/if}
		</Label>

		<Label label="Custom tags to autoscale on" for="multi_select_custom_tags">
			{#snippet header()}
				<Tooltip>
					By default, autoscaling will apply to the tags the worker group is assigned to but you can
					override this here.
				</Tooltip>
			{/snippet}
			{#if config}
				<MultiSelect
					id="multi_select_custom_tags"
					bind:value={
						() => config?.custom_tags ?? [],
						(v) => {
							config && (config.custom_tags = v.length ? v : undefined)
						}
					}
					items={safeSelectItems(worker_tags)}
					placeholder="Tags"
					{disabled}
				/>
			{/if}
		</Label>
	</Section>
</Section>
