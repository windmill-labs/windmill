<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Save, RotateCcw } from 'lucide-svelte'
	import { type Snippet } from 'svelte'

	import { Tooltip } from '../meltComponents'
	import DeleteTriggerButton from './DeleteTriggerButton.svelte'
	import { type Trigger, type TriggerType } from './utils'
	import { scheduleLock, triggerLock } from '$lib/operatorWriteRights'
	import TriggerSuspendedJobsModal from './TriggerSuspendedJobsModal.svelte'
	import type { TriggerMode } from '$lib/gen'
	import TriggerModeToggle from './TriggerModeToggle.svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { stripBase, TRIGGER_PAGES, SCHEDULES_PATH } from '$lib/components/sessions/previewPaths'
	import { pageDrawerSessionSource } from '../sessions/pageDrawerSession'
	import { page } from '$app/state'
	import { workspaceStore } from '$lib/stores'
	import TriggerHistoryButton from './TriggerHistoryButton.svelte'

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
		onToggleMode: (mode: TriggerMode) => void | boolean | Promise<void | boolean>
		onUpdate?: () => void
		cloudDisabled?: boolean
		trigger?: Trigger
		suspendedJobsModal?: TriggerSuspendedJobsModal | null
		disableSuspendedMode?: boolean
		/** Path of the trigger being edited, used to deep-link "Open in AI session"
		 * at this trigger. Empty while creating one. */
		triggerPath?: string
		/** Kind the modification history is recorded under. Only the schedule
		 * editor has to pass it: every other editor renders with a `trigger`,
		 * whose `type` is the same value. */
		triggerKind?: TriggerType
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
		triggerPath,
		triggerKind
	}: Props = $props()

	const canSave = $derived((permissions === 'write' && edit) || permissions === 'create')

	// Applied here because every schedule and trigger editor renders this toolbar, while each of
	// them derives its own `can_write` from path permissions alone. Anything that is not the
	// schedule editor edits a trigger, the native editor — which passes no kind — included.
	const writeLock = $derived(triggerKind === 'schedule' ? $scheduleLock : $triggerLock)

	// "Open in AI session", on the standalone trigger list pages only: the route
	// gate inside pageDrawerSessionSource is what keeps it off this same toolbar
	// when it renders in a script/flow editor's Triggers panel, which has the
	// editor's own button. A trigger being created has no path to deep-link at.
	const triggerPagePath = $derived.by(() => {
		const route = stripBase(page.url.pathname)
		if (route === SCHEDULES_PATH) return route
		return Object.values(TRIGGER_PAGES).some((p) => p.path === route) ? route : undefined
	})
	const sessionSource = $derived(
		triggerPagePath
			? pageDrawerSessionSource(
					triggerPagePath,
					trigger?.isDraft ? undefined : triggerPath || trigger?.path,
					$workspaceStore ?? undefined
				)
			: undefined
	)

	// Only a deployed trigger has a history: a draft has never been written.
	// `triggerKind` is what opts an editor in, so the kinds `trigger_history`
	// does not record (native triggers) simply never pass it.
	const historyPath = $derived(
		triggerKind && edit && !trigger?.isDraft ? triggerPath || trigger?.path : undefined
	)
</script>

{#if !allowDraft}
	{@render extra?.()}
	{#if triggerKind && historyPath}
		<TriggerHistoryButton {triggerKind} path={historyPath} />
	{/if}
	<OpenInSessionButton source={sessionSource} />
	{#if edit}
		<TriggerModeToggle
			canWrite={canSave && !writeLock}
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
			disabled={saveDisabled || !!writeLock}
			title={writeLock}
			on:click={() => {
				onUpdate?.()
			}}
			loading={isLoading}
		>
			Save
		</Button>
	{/if}
{:else}
	<!-- This toolbar inside a script or flow editor's Triggers panel: another path to the same
		 trigger writes, so it takes the same lock. -->
	<div class="flex flex-row gap-2 items-center">
		{#if triggerKind && historyPath}
			<TriggerHistoryButton {triggerKind} path={historyPath} />
		{/if}
		<OpenInSessionButton source={sessionSource} />
		{#if !trigger?.draftConfig}
			<div class="center-center">
				<TriggerModeToggle
					canWrite={permissions !== 'none' && !writeLock}
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
			<Tooltip
				placement="bottom-end"
				disablePopup={!saveDisabled && !cloudDisabled && isDeployed && !writeLock}
			>
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: Save }}
					disabled={saveDisabled ||
						cloudDisabled ||
						!isDeployed ||
						!trigger?.draftConfig ||
						!!writeLock}
					on:click={() => {
						onUpdate?.()
					}}
					loading={isLoading}
				>
					{trigger?.isDraft ? 'Deploy' : 'Update'}
				</Button>
				{#snippet text()}
					<span>
						{#if writeLock}
							{writeLock}
						{:else if !isDeployed}
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
