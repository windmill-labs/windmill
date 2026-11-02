<script lang="ts">
	import { Alert, Button, SecondsInput } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import DebounceLimit from '$lib/components/flows/DebounceLimit.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import type { Schema } from '$lib/common'
	import type { ScriptAdvancedSettingsFields } from '$lib/components/scriptSettings'
	import { Plus, X } from 'lucide-svelte'
	import SettingGroup from '$lib/components/itemSettings/SettingGroup.svelte'
	import SettingToggle from './SettingToggle.svelte'
	import WorkerTagSetting from './WorkerTagSetting.svelte'
	import ConcurrencyLimitSetting from './ConcurrencyLimitSetting.svelte'
	import CacheSetting from './CacheSetting.svelte'
	import PrioritySetting from './PrioritySetting.svelte'
	import DedicatedWorkersSetting from './DedicatedWorkersSetting.svelte'
	import DeleteAfterUseSetting from './DeleteAfterUseSetting.svelte'

	interface Props {
		script: ScriptAdvancedSettingsFields
		/** Workspace to read worker tags from; the picker defaults to the operating one. */
		workspaceId?: string
		tagPlaceholder?: string
		/** Seeded when the custom timeout is turned on. */
		defaultTimeout?: number
	}

	let { script = $bindable(), workspaceId, tagPlaceholder, defaultTimeout = 300 }: Props = $props()

	const DEDICATED_WORKER_LANGUAGES = ['bun', 'bunnative', 'python3', 'deno']

	// The list opens before a variable is typed, so the toggle cannot read only `envs`.
	let addingEnvs = $state(false)
</script>

<div class="flex flex-col gap-8">
	<SettingGroup label="Execution policy" class="flex flex-col gap-6">
		<SettingToggle
			setting="timeout"
			label="Custom timeout"
			tooltip="Used instead of the instance timeout, which it cannot exceed."
			documentationLink="https://www.windmill.dev/docs/script_editor/settings#timeout"
			checked={!!script.timeout}
			onchange={(on) => (script.timeout = on ? defaultTimeout : undefined)}
		>
			<SecondsInput bind:seconds={script.timeout} />
		</SettingToggle>
		<ConcurrencyLimitSetting
			kind="script"
			bind:limit={script.concurrent_limit}
			bind:timeWindowS={script.concurrency_time_window_s}
			bind:concurrencyKey={script.concurrency_key}
			keyPlaceholder={`$workspace/script/${script.path ?? ''}-$args[foo]`}
		/>
		<DebounceLimit
			bind:debounce_delay_s={script.debounce_delay_s}
			bind:debounce_key={script.debounce_key}
			bind:debounce_args_to_accumulate={script.debounce_args_to_accumulate}
			bind:max_total_debouncing_time={script.max_total_debouncing_time}
			bind:max_total_debounces_amount={script.max_total_debounces_amount}
			schema={script.schema as Schema}
			placeholder={`$workspace/script/${script.path ?? ''}-$args[foo]`}
		/>
		<CacheSetting
			kind="script"
			bind:ttl={script.cache_ttl}
			bind:ignoreS3Path={script.cache_ignore_s3_path}
		/>
		<PrioritySetting kind="script" bind:priority={script.priority} />
		<SettingToggle
			setting="perpetual"
			label="Perpetual script"
			tooltip="The script restarts as soon as it ends, unless it was cancelled."
			documentationLink="https://www.windmill.dev/docs/script_editor/perpetual_scripts"
			checked={!!script.restart_unless_cancelled}
			onchange={(on) => (script.restart_unless_cancelled = on || undefined)}
		/>
		<DeleteAfterUseSetting kind="script" bind:deleteAfterSecs={script.delete_after_secs} />
	</SettingGroup>

	<SettingGroup label="Workers" class="flex flex-col gap-6">
		<WorkerTagSetting
			kind="script"
			bind:tag={script.tag}
			placeholder={tagPlaceholder}
			{workspaceId}
		/>
		<DedicatedWorkersSetting
			kind="script"
			bind:dedicatedWorker={script.dedicated_worker}
			unsupportedReason={script.language && !DEDICATED_WORKER_LANGUAGES.includes(script.language)
				? 'Only for Python, Bun and Deno scripts. Other languages start no full runtime, so they already run about as fast.'
				: undefined}
		/>
		{#if !isCloudHosted()}
			<SettingToggle
				setting="envs"
				label="Custom env variables"
				tooltip="Static environment variables passed to the deployed script."
				documentationLink="https://www.windmill.dev/docs/script_editor/custom_environment_variables"
				checked={addingEnvs || (script.envs?.length ?? 0) > 0}
				onchange={(on) => {
					addingEnvs = on
					script.envs = on ? [''] : []
				}}
			>
				<Alert type="info" title="Not passed in previews" size="xs">
					Only the deployed script gets these variables.
				</Alert>
				{#each script.envs ?? [] as _, i (i)}
					<div class="flex flex-row items-center gap-1 max-w-md">
						{#if script.envs}
							<TextInput
								bind:value={script.envs[i]}
								inputProps={{ type: 'text', placeholder: 'KEY=VALUE' }}
							/>
						{/if}
						<Button
							variant="subtle"
							unifiedSize="md"
							iconOnly
							startIcon={{ icon: X }}
							title="Remove variable"
							onclick={() => (script.envs = script.envs?.filter((_, j) => j !== i))}
						/>
					</div>
				{/each}
				<div class="flex">
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						onclick={() => (script.envs = [...(script.envs ?? []), ''])}
					>
						Add variable
					</Button>
				</div>
			</SettingToggle>
		{/if}
	</SettingGroup>
</div>
