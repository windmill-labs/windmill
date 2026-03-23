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
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { userStore, workspaceStore } from '$lib/stores'

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
		/** Current permissioned_as value from the trigger (e.g., 'u/admin') */
		permissionedAs?: string
		/** Callback when user changes the permissioned_as selection */
		onPermissionedAsChange?: (permissionedAs: string | undefined, preserve: boolean) => void
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
		disableSuspendedMode = false,
		permissionedAs,
		onPermissionedAsChange
	}: Props = $props()

	const canSave = $derived((permissions === 'write' && edit) || permissions === 'create')

	const canPreserve = $derived(
		$userStore?.is_admin || ($userStore?.groups ?? []).includes('wm_deployers')
	)

	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customPermissionedAs = $state<string | undefined>(undefined)

	function handleOnBehalfOfSelect(choice: OnBehalfOfChoice, details?: OnBehalfOfDetails) {
		onBehalfOfChoice = choice
		if (choice === 'target') {
			customPermissionedAs = undefined
			onPermissionedAsChange?.(permissionedAs, true)
		} else if (choice === 'custom' && details) {
			customPermissionedAs = details.permissionedAs
			onPermissionedAsChange?.(details.permissionedAs, true)
		} else {
			customPermissionedAs = undefined
			onPermissionedAsChange?.(undefined, false)
		}
	}
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
	{#if edit && permissionedAs && $workspaceStore}
		<OnBehalfOfSelector
			targetWorkspace={$workspaceStore}
			targetValue={permissionedAs}
			selected={onBehalfOfChoice}
			onSelect={handleOnBehalfOfSelect}
			kind="trigger"
			{canPreserve}
			customValue={customPermissionedAs}
			isDeployment={false}
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
		{#if edit && permissionedAs && $workspaceStore}
			<OnBehalfOfSelector
				targetWorkspace={$workspaceStore}
				targetValue={permissionedAs}
				selected={onBehalfOfChoice}
				onSelect={handleOnBehalfOfSelect}
				kind="trigger"
				{canPreserve}
				customValue={customPermissionedAs}
				isDeployment={false}
			/>
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
				{#snippet text()}
								<span >
						{#if !isDeployed}
							Deploy the runnable to enable trigger creation
						{:else if cloudDisabled}
							This trigger is disabled in the multi-tenant cloud
						{:else}
							Enter a valid config to {trigger?.isDraft ? 'deploy' : 'update'} the trigger
						{/if}
					</span>
							{/snippet}
			</Tooltip>
		{/if}
	</div>
{/if}
