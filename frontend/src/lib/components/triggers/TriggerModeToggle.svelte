<script lang="ts">
	import type { JobTriggerKind, TriggerMode } from '$lib/gen'
	import { Pause } from 'lucide-svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import Toggle from '../Toggle.svelte'
	import TriggerSuspendedJobsModal, {
		type TriggerRunnableConfig
	} from './TriggerSuspendedJobsModal.svelte'

	interface Props {
		triggerMode: TriggerMode
		onToggleMode: (mode: TriggerMode) => void
		canWrite: boolean
		hideToggleLabels?: boolean
		hideDropdown?: boolean
		suspendedJobsModal?: TriggerSuspendedJobsModal | null
		includeModalConfig?: {
			runnableConfig: TriggerRunnableConfig
			triggerPath: string
			triggerKind: JobTriggerKind
		}
	}

	let {
		triggerMode,
		onToggleMode,
		canWrite,
		hideToggleLabels = false,
		hideDropdown = false,
		suspendedJobsModal: passedSuspendedJobsModal,
		includeModalConfig
	}: Props = $props()

	let innerTriggerMode = $derived(triggerMode)
	let suspendedJobsModal = $derived(passedSuspendedJobsModal ?? null)
</script>

{#if includeModalConfig && triggerMode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={includeModalConfig.triggerPath}
		triggerKind={includeModalConfig.triggerKind}
		runnableConfig={includeModalConfig.runnableConfig}
		{onToggleMode}
		hasChanged={false}
	/>
{/if}

<div class="flex flex-row gap-2 items-center">
	{#if triggerMode === 'suspended'}
		<ToggleButtonGroup
			disabled={!canWrite}
			onSelected={async (value) => {
				if (value === 'disabled' || value === 'enabled') {
					const hasJobs = await suspendedJobsModal?.hasJobs()
					if (hasJobs) {
						innerTriggerMode = 'suspended'
						suspendedJobsModal?.openModal(value === 'disabled' ? 'disable' : 'enable')
					} else {
						onToggleMode(value)
					}
				}
			}}
			bind:selected={innerTriggerMode}
		>
			{#snippet children({ item })}
				<ToggleButton value="disabled" label="Disable" {item} />
				<ToggleButton value="suspended" label="Suspended" {item} />
				<ToggleButton value="enabled" label="Enable" {item} />
			{/snippet}
		</ToggleButtonGroup>
	{:else}
		<Toggle
			disabled={!canWrite}
			options={hideToggleLabels ? undefined : { right: 'enable', left: 'disable' }}
			checked={triggerMode === 'enabled'}
			on:change={(e) => {
				onToggleMode(e.detail ? 'enabled' : 'disabled')
			}}
		/>
		{#if !hideDropdown}
			<DropdownV2
				disabled={!canWrite}
				items={[
					{
						displayName: 'Suspend job execution',
						icon: Pause,
						action: () => {
							triggerMode = 'suspended'
							onToggleMode?.('suspended')
						},
						tooltip:
							'When a trigger is in suspended mode, it will continue to accept payloads and queue jobs, but those jobs will not run automatically. You can review the list of suspended jobs, and resume or cancel them individually.'
					}
				]}
			/>
		{/if}
	{/if}
</div>
