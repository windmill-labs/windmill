<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Save, RotateCcw } from 'lucide-svelte'
	import { type Snippet } from 'svelte'

	import { Tooltip } from '../meltComponents'
	import DeleteTriggerButton from './DeleteTriggerButton.svelte'
	import { type Trigger } from './utils'
	import TriggerSuspendedJobsModal from './TriggerSuspendedJobsModal.svelte'
	import type { TriggerMode } from '$lib/gen'
	import TriggerModeToggle from './TriggerModeToggle.svelte'

	interface Props {
		saveDisabled: any
		mode: TriggerMode
		allowDraft: any
		edit: any
		isLoading: any
		permissions: 'write' | 'create' | 'none'
		isDeployed: boolean
		extra?: Snippet
		onDelete?: () => void
		onReset?: () => void
		onToggleMode: (mode: TriggerMode) => void
		onUpdate?: () => void
		cloudDisabled?: boolean
		trigger?: Trigger
		suspendedJobsModal?: TriggerSuspendedJobsModal | null
		disableSuspendedMode?: boolean
	}

	let {
		saveDisabled,
		mode,
		allowDraft,
		edit,
		isLoading,
		permissions,
		isDeployed,
		extra,
		onDelete,
		onReset,
		onToggleMode,
		onUpdate,
		cloudDisabled = false,
		trigger,
		suspendedJobsModal,
		disableSuspendedMode = false
	}: Props = $props()

	const canSave = $derived((permissions === 'write' && edit) || permissions === 'create')
</script>

{#if !allowDraft}
	{@render extra?.()}
	{#if edit}
		<TriggerModeToggle
			canWrite={canSave}
			triggerMode={mode}
			{onToggleMode}
			{suspendedJobsModal}
			hideDropdown={disableSuspendedMode}
		/>
	{/if}
	{#if canSave}
		<Button
			size="sm"
			variant="accent"
			startIcon={{ icon: Save }}
			disabled={saveDisabled}
			on:click={() => {
				onUpdate?.()
			}}
			loading={isLoading}
		>
			Save
		</Button>
	{/if}
{:else}
	<div class="flex flex-row gap-2 items-center">
		{#if !trigger?.draftConfig}
			<div class="center-center">
				<TriggerModeToggle
					canWrite={permissions !== 'none'}
					triggerMode={mode}
					{onToggleMode}
					{suspendedJobsModal}
					hideDropdown={disableSuspendedMode}
				/>
			</div>
		{/if}
		{#if trigger?.isDraft || permissions === 'create'}
			<DeleteTriggerButton {onDelete} {trigger} />
		{:else if !trigger?.isDraft && trigger?.draftConfig}
			<Button
				unifiedSize="sm"
				startIcon={{ icon: RotateCcw }}
				variant="default"
				on:click={() => {
					onReset?.()
				}}
			>
				Reset changes
			</Button>
		{/if}
		{#if canSave}
			<Tooltip placement="bottom-end" disablePopup={!saveDisabled && !cloudDisabled && isDeployed}>
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					disabled={saveDisabled || cloudDisabled || !isDeployed || !trigger?.draftConfig}
					on:click={() => {
						onUpdate?.()
					}}
					loading={isLoading}
				>
					{trigger?.isDraft ? 'Deploy' : 'Update'}
				</Button>
				<span slot="text">
					{#if !isDeployed}
						Deploy the runnable to enable trigger creation
					{:else if cloudDisabled}
						This trigger is disabled in the multi-tenant cloud
					{:else}
						Enter a valid config to {trigger?.isDraft ? 'deploy' : 'update'} the trigger
					{/if}
				</span>
			</Tooltip>
		{/if}
	</div>
{/if}
