<script lang="ts">
	import { createBubbler } from 'svelte/legacy'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { SecondsInput } from '$lib/components/common'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import DebounceLimit from '$lib/components/flows/DebounceLimit.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import type { Schema } from '$lib/common'
	import type { ScriptAdvancedSettingsFields } from './scriptSettings'

	const bubble = createBubbler()

	interface Props {
		script: ScriptAdvancedSettingsFields
		// Workspace to read worker tags from (defaults to $workspaceStore inside the picker).
		workspaceId?: string | undefined
	}

	let { script = $bindable(), workspaceId = undefined }: Props = $props()
</script>

<div class="flex flex-col gap-8">
	<Section label="Worker group tag (queue)">
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups">
				The script will be executed on a worker configured to listen to this worker group tag
				(queue). For instance, you could setup an "highmem", or "gpu" tag.
			</Tooltip>
		{/snippet}
		<WorkerTagPicker bind:tag={script.tag} {workspaceId} />
	</Section>

	<Section label="Concurrency limits" eeOnly>
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/concurrency_limits">
				Allowed concurrency within a given timeframe
			</Tooltip>
		{/snippet}
		<Toggle
			size="sm"
			checked={Boolean(script.concurrent_limit)}
			on:change={() => {
				if (script.concurrent_limit && script.concurrent_limit != undefined) {
					script.concurrent_limit = undefined
					script.concurrency_time_window_s = undefined
					script.concurrency_key = undefined
				} else {
					script.concurrent_limit = 1
				}
			}}
			options={{ right: 'Concurrency limits' }}
		/>
		{#if Boolean(script.concurrent_limit)}
			<div class="flex flex-col gap-4 mt-2">
				<Label label="Max number of executions within the time window">
					<div class="flex flex-row gap-2 max-w-sm whitespace-nowrap">
						<input
							disabled={!$enterpriseLicense}
							bind:value={script.concurrent_limit}
							type="number"
						/>
					</div>
				</Label>
				<Label label="Time window in seconds">
					<SecondsInput
						disabled={!$enterpriseLicense}
						bind:seconds={script.concurrency_time_window_s}
					/>
				</Label>
				<Label label="Custom concurrency key (optional)">
					{#snippet header()}
						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/concurrency_limits#custom-concurrency-key"
						>
							Concurrency keys are global, you can have them be workspace specific using the
							variable `$workspace`. You can also use an argument's value using `$args[name_of_arg]`</Tooltip
						>
					{/snippet}
					<input
						disabled={!$enterpriseLicense}
						type="text"
						bind:value={script.concurrency_key}
						placeholder={`$workspace/script/${script.path ?? ''}-$args[foo]`}
					/>
				</Label>
			</div>
		{/if}
	</Section>

	<Section label="Cache">
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/caching">
				Cache the results for each possible inputs
			</Tooltip>
		{/snippet}
		<div class="flex gap-2 shrink flex-col">
			<Toggle
				size="sm"
				bind:checked={() => !!script.cache_ttl, (v) => (script.cache_ttl = v ? 300 : undefined)}
				options={{ right: 'Cache the results for each possible inputs' }}
			/>
			{#if script.cache_ttl}
				<div class="text-2xs text-secondary">How long to keep the cache valid</div>
				<SecondsInput bind:seconds={script.cache_ttl} />
				<Toggle
					size="2xs"
					bind:checked={
						() => script.cache_ignore_s3_path, (v) => (script.cache_ignore_s3_path = v || undefined)
					}
					options={{
						right: 'Ignore S3 Object paths for caching purposes',
						rightTooltip:
							'If two S3 objects passed as input have the same content, they will hit the same cache entry, regardless of their path.'
					}}
				/>
			{/if}
		</div>
	</Section>

	<Section label="Timeout">
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/script_editor/settings#timeout">
				Add a custom timeout for this script
			</Tooltip>
		{/snippet}
		<div class="flex gap-2 shrink flex-col">
			<Toggle
				size="sm"
				checked={Boolean(script.timeout)}
				on:change={() => {
					if (script.timeout && script.timeout != undefined) {
						script.timeout = undefined
					} else {
						script.timeout = 300
					}
				}}
				options={{ right: 'Add a custom timeout for this script' }}
			/>
			{#if Boolean(script.timeout)}
				<span class="text-xs font-semibold text-emphasis leading-none mt-2">Timeout duration</span>
				<SecondsInput bind:seconds={script.timeout} />
			{/if}
		</div>
	</Section>

	<Section label="Debouncing">
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/job_debouncing">
				Debounce Jobs
			</Tooltip>
		{/snippet}
		<DebounceLimit
			size="sm"
			bind:debounce_delay_s={script.debounce_delay_s}
			bind:debounce_key={script.debounce_key}
			bind:debounce_args_to_accumulate={script.debounce_args_to_accumulate}
			bind:max_total_debouncing_time={script.max_total_debouncing_time}
			bind:max_total_debounces_amount={script.max_total_debounces_amount}
			schema={script.schema as Schema}
			placeholder={`$workspace/script/${script.path ?? ''}-$args[foo]`}
		/>
	</Section>

	<Section label="Perpetual script">
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/script_editor/perpetual_scripts">
				Restart the script upon ending unless cancelled
			</Tooltip>
		{/snippet}
		<Toggle
			size="sm"
			checked={Boolean(script.restart_unless_cancelled)}
			on:change={() => {
				script.restart_unless_cancelled = script.restart_unless_cancelled ? undefined : true
			}}
			options={{ right: 'Restart upon ending unless cancelled' }}
		/>
	</Section>

	<Section label="Dedicated workers" eeOnly>
		{#snippet header()}
			<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/dedicated_workers">
				In this mode, the script is meant to be run on dedicated workers that run the script at
				native speed. Can reach &gt;1500rps per dedicated worker. Only available on enterprise
				edition and for Python3, Deno, Bun and Bunnative.
			</Tooltip>
		{/snippet}
		<Toggle
			disabled={!$enterpriseLicense ||
				isCloudHosted() ||
				(script.language != 'bun' &&
					script.language != 'bunnative' &&
					script.language != 'python3' &&
					script.language != 'deno')}
			size="sm"
			checked={Boolean(script.dedicated_worker)}
			on:change={() => {
				script.dedicated_worker = script.dedicated_worker ? undefined : true
			}}
			options={{ right: 'Script is run on dedicated workers' }}
		/>
		{#if script.dedicated_worker}
			<div class="py-2">
				<Alert type="info" title="Require dedicated workers">
					A worker group needs to be configured to listen to this script. Select it in the dedicated
					workers section of the worker group configuration.
				</Alert>
			</div>
		{/if}
	</Section>

	<Section label="Delete after completion">
		{#snippet header()}
			<Tooltip
				documentationLink="https://www.windmill.dev/docs/script_editor/settings#delete-after-use"
			>
				The logs, arguments and results of the job will be completely deleted from Windmill after
				the specified delay once it is complete. Set to 0 for immediate deletion. The deletion is
				irreversible. This settings ONLY applies when the script is used within a flow or triggered
				synchronously.
				{#if !$enterpriseLicense}
					This option is only available on Windmill Enterprise Edition.
				{/if}
			</Tooltip>
		{/snippet}
		<div class="flex gap-2 shrink flex-col">
			<Toggle
				disabled={!$enterpriseLicense}
				size="sm"
				checked={script.delete_after_secs != null}
				on:change={() => {
					script.delete_after_secs = script.delete_after_secs != null ? undefined : 0
				}}
				options={{ right: 'Delete logs, arguments and results after completion' }}
			/>
			{#if script.delete_after_secs != null}
				<SecondsInput bind:seconds={script.delete_after_secs} disabled={!$enterpriseLicense} />
			{/if}
		</div>
	</Section>

	{#if !isCloudHosted()}
		<Section label="High priority script" eeOnly>
			{#snippet header()}
				<Tooltip
					documentationLink="https://www.windmill.dev/docs/core_concepts/jobs#high-priority-jobs"
				>
					Jobs from script labeled as high priority take precedence over the other jobs when in the
					jobs queue.
					{#if !$enterpriseLicense}This is a feature only available on enterprise edition.{/if}
				</Tooltip>
			{/snippet}
			<Toggle
				disabled={!$enterpriseLicense || isCloudHosted()}
				size="sm"
				checked={script.priority !== undefined && script.priority > 0}
				on:change={() => {
					script.priority = script.priority ? undefined : 100
				}}
				options={{ right: 'Label as high priority' }}
			>
				{#snippet right()}
					<input
						type="number"
						class="!w-16 ml-4"
						disabled={script.priority === undefined}
						bind:value={script.priority}
						onfocus={bubble('focus')}
						onchange={() => {
							if (script.priority && script.priority > 100) {
								script.priority = 100
							} else if (script.priority && script.priority < 0) {
								script.priority = 0
							}
						}}
					/>
				{/snippet}
			</Toggle>
		</Section>
	{/if}
</div>
