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
		// Optionally returns false to signal that the change was cancelled
		// (e.g. user dismissed a fork-conflict modal). When the parent didn't
		// optimistically update `triggerMode` (list-page rows fall in this
		// bucket), we resync the local toggle state from the prop after the
		// dispatch resolves.
		onToggleMode: (mode: TriggerMode) => void | boolean | Promise<void | boolean>
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

	// Local writable state mirroring `triggerMode`. Both inner controls bind
	// to it: the suspended ToggleButtonGroup directly (it already needed a
	// writable to revert to 'suspended' when there are queued jobs), and the
	// regular Toggle via a function binding that maps 'enabled'/'disabled'
	// to a boolean. Either control can write back here, and the $effect
	// re-syncs us from the prop whenever the parent updates `triggerMode`.
	// This lets the parent reset us in cases where the user click can't go
	// through (e.g. cancelled fork-conflict modal).
	let innerTriggerMode = $state(triggerMode)
	$effect(() => {
		innerTriggerMode = triggerMode
	})
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
			bind:checked={
				() => innerTriggerMode === 'enabled', (v) => (innerTriggerMode = v ? 'enabled' : 'disabled')
			}
			on:change={async (e) => {
				const result = await onToggleMode(e.detail ? 'enabled' : 'disabled')
				if (result === false) {
					// Cancelled: snap the toggle back to the prop. Needed for
					// list-page rows where the parent doesn't optimistically
					// flip `triggerMode` (so $effect won't re-run from a no-op
					// prop change).
					innerTriggerMode = triggerMode
				}
			}}
		/>
		{#if !hideDropdown}
			<DropdownV2
				disabled={!canWrite}
				items={[
					{
						displayName: 'Suspend job execution',
						icon: Pause,
						action: async () => {
							// Optimistically flip the local mirror, not the
							// non-bindable `triggerMode` prop. The parent will
							// echo the new mode back via $effect on success;
							// on cancel (e.g. user dismisses the fork-conflict
							// modal), reset to whatever the prop says — same
							// shape as the Toggle's on:change handler.
							innerTriggerMode = 'suspended'
							const result = await onToggleMode?.('suspended')
							if (result === false) {
								innerTriggerMode = triggerMode
							}
						},
						tooltip:
							'When a trigger is in suspended mode, it will continue to accept payloads and queue jobs, but those jobs will not run automatically. You can review the list of suspended jobs, and resume or cancel them individually.'
					}
				]}
			/>
		{/if}
	{/if}
</div>
