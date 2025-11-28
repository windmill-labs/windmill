<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Save, RotateCcw } from 'lucide-svelte'
	import { type Snippet } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Tooltip } from '../meltComponents'
	import DeleteTriggerButton from './DeleteTriggerButton.svelte'
	import { type Trigger } from './utils'
	import DropdownV2 from '../DropdownV2.svelte'
	import TriggerSuspendedJobsModal from './TriggerSuspendedJobsModal.svelte'
	import type { JobTriggerKind } from '$lib/gen'

	interface Props {
		saveDisabled: any
		enabled: boolean | undefined
		allowDraft: any
		edit: any
		isLoading: any
		permissions: 'write' | 'create' | 'none'
		isDeployed: boolean
		kind: JobTriggerKind
		path: string
		editedAt: string | undefined
		suspendedMode?: boolean
		extra?: Snippet
		onDelete?: () => void
		onReset?: () => void
		onToggleEnabled?: (enabled: boolean) => void
		onToggleSuspendedMode?: (suspendedMode: boolean, enabled?: boolean) => void
		onUpdate?: () => void
		cloudDisabled?: boolean
		trigger?: Trigger
	}

	let {
		saveDisabled,
		enabled,
		allowDraft,
		edit,
		isLoading,
		permissions,
		isDeployed,
		kind,
		path,
		suspendedMode = false,
		editedAt,
		extra,
		onDelete,
		onReset,
		onToggleEnabled,
		onUpdate,
		onToggleSuspendedMode,
		cloudDisabled = false,
		trigger
	}: Props = $props()

	const canSave = $derived((permissions === 'write' && edit) || permissions === 'create')

	let showSuspendedJobsModal = $state(false)
</script>

{#if path && suspendedMode}
	<TriggerSuspendedJobsModal
		bind:shouldShowModal={showSuspendedJobsModal}
		{suspendedMode}
		triggerPath={path}
		jobTriggerKind={kind}
		{onToggleSuspendedMode}
		{editedAt}
	/>
{/if}

{#if !allowDraft}
	{@render extra?.()}
	{#if edit && enabled !== undefined}
		{#if suspendedMode}
			<Button
				on:click={() => {
					showSuspendedJobsModal = true
				}}
				variant="accent"
			>
				See suspended jobs
			</Button>
		{:else}
			<Toggle
				size="sm"
				disabled={permissions === 'none'}
				checked={enabled}
				options={{ right: 'enable', left: 'disable' }}
				on:change={({ detail }) => {
					onToggleEnabled?.(detail)
				}}
			/>

			<DropdownV2
				items={[
					{
						displayName: 'Suspend job executions',
						action: () => {
							onToggleSuspendedMode?.(true)
						},
						tooltip:
							'Suspend job executions for this trigger, allowing you to individually resume or cancel them and reassign them to a different runnable'
					}
				]}
			/>
		{/if}
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
		{#if !trigger?.draftConfig && enabled !== undefined}
			<div class="center-center">
				<Toggle
					disabled={permissions === 'none'}
					checked={enabled}
					options={{ right: 'enable', left: 'disable' }}
					on:change={({ detail }) => {
						onToggleEnabled?.(detail)
					}}
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
