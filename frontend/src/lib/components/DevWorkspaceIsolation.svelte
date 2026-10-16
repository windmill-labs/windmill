<script lang="ts">
	import { Alert, Badge, Button, Section } from './common'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Toggle from './Toggle.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { ArrowRight, ChevronDown, ChevronRight, Save, Settings } from 'lucide-svelte'
	import {
		SettingService,
		WorkerService,
		WorkspaceService,
		type GetQueueStatusResponse,
		type Workspace
	} from '$lib/gen'
	import { DEV_WORKSPACE_TAG_SUFFIX_SETTING } from '$lib/consts'
	import {
		DEFAULT_DEV_WORKSPACE_TAG_SUFFIX,
		devWorkspaceTagSuffix,
		isDevOrForkWorkspace,
		loadDevWorkspaceTagSuffix,
		withDevSuffix
	} from '$lib/devWorkspaceTags'
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { pluralize } from '$lib/utils'
	import { onMount } from 'svelte'

	interface Props {
		/** Worker group configs, by group name. */
		workerGroups: Record<string, any> | undefined
		/** Workers of each group that pinged in the last 15 seconds. */
		liveWorkers: Record<string, number>
		canEdit: boolean
		onConfigureGroup: (name: string) => void
	}

	let { workerGroups, liveWorkers, canEdit, onConfigureGroup }: Props = $props()

	let suffix = $derived($devWorkspaceTagSuffix)
	let enabled = $derived(suffix != null)
	let effectiveSuffix = $derived(suffix ?? DEFAULT_DEV_WORKSPACE_TAG_SUFFIX)

	// Overridden while a confirmation is pending, reset to the saved state otherwise.
	let toggleChecked = $derived(enabled)
	let suffixDraft = $derived(effectiveSuffix)
	let suffixError = $derived(
		!suffixDraft.trim()
			? 'The suffix cannot be empty'
			: /[\s,]/.test(suffixDraft.trim())
				? 'The suffix cannot contain spaces or commas'
				: undefined
	)

	let workspaces = $state<Workspace[] | undefined>(undefined)
	let devWorkspaces = $derived(
		(workspaces ?? []).filter((w) => !w.deleted && isDevOrForkWorkspace(w))
	)
	let workspaceNames = $derived(Object.fromEntries((workspaces ?? []).map((w) => [w.id, w.name])))

	async function loadWorkspaces() {
		const all: Workspace[] = []
		for (let page = 1; ; page++) {
			const batch = await WorkspaceService.listWorkspacesAsSuperAdmin({ page, perPage: 100 })
			all.push(...batch)
			if (batch.length < 100) break
		}
		workspaces = all
	}

	let queueStatus = $state<GetQueueStatusResponse | undefined>(undefined)
	async function loadQueueStatus() {
		try {
			queueStatus = await WorkerService.getQueueStatus()
		} catch {
			queueStatus = undefined
		}
	}

	onMount(() => {
		loadDevWorkspaceTagSuffix(true)
		loadWorkspaces().catch((e) => console.error('Could not list workspaces', e))
		loadQueueStatus()
		const interval = setInterval(loadQueueStatus, 15_000)
		return () => clearInterval(interval)
	})

	type GroupEntry = { name: string; live: number; tags: string[] }

	function isDedicated(config: any): boolean {
		return config?.dedicated_worker != undefined || (config?.dedicated_workers?.length ?? 0) > 0
	}

	let flaggedGroups: GroupEntry[] = $derived(
		Object.entries(workerGroups ?? {})
			.filter(([_, c]) => c?.serves_dev_workspaces === true && !isDedicated(c))
			.map(([name, c]) => ({ name, live: liveWorkers[name] ?? 0, tags: c?.worker_tags ?? [] }))
	)
	let liveFlaggedWorkers = $derived(flaggedGroups.reduce((acc, g) => acc + g.live, 0))
	let unflaggedGroupNames = $derived(
		Object.entries(workerGroups ?? {})
			.filter(([_, c]) => c?.serves_dev_workspaces !== true && !isDedicated(c))
			.map(([name]) => name)
	)
	// The only way to break isolation: an unflagged group listing a suffixed tag takes dev jobs.
	let leakingGroups = $derived(
		Object.entries(workerGroups ?? {})
			.filter(([_, c]) => c?.serves_dev_workspaces !== true)
			.map(([name, c]) => ({
				name,
				tags: ((c?.worker_tags ?? []) as string[]).filter((t) => t.endsWith(effectiveSuffix))
			}))
			.filter((g) => g.tags.length > 0)
	)
	let unservedDevTags = $derived(
		enabled
			? (queueStatus ?? []).filter(
					(s) => s.tag.endsWith(effectiveSuffix) && s.waiting > 0 && s.workers === 0
				)
			: []
	)

	let groupToFlag = $state<string | undefined>(undefined)
	let showWorkspaces = $state(false)
	let showAdvanced = $state(false)

	let confirmEnable = $state(false)
	let confirmDisable = $state(false)
	let saving = $state(false)

	async function saveSuffix(value: string | undefined) {
		saving = true
		try {
			await SettingService.setGlobal({
				key: DEV_WORKSPACE_TAG_SUFFIX_SETTING,
				requestBody: { value }
			})
			devWorkspaceTagSuffix.set(value ?? null)
			sendUserToast(
				value
					? `Dev workspace and fork jobs now run on tags suffixed with ${value}`
					: 'Dev workspace isolation turned off'
			)
		} catch (e) {
			sendUserToast(`Could not save dev workspace isolation: ${e}`, true)
			toggleChecked = enabled
		} finally {
			saving = false
		}
	}

	function onToggle(checked: boolean) {
		if (checked === enabled) return
		if (checked) {
			if (liveFlaggedWorkers === 0) {
				confirmEnable = true
			} else {
				saveSuffix(suffixDraft.trim() || DEFAULT_DEV_WORKSPACE_TAG_SUFFIX)
			}
		} else {
			confirmDisable = true
		}
	}

	function workspaceKind(w: Workspace): string {
		if (w.parent_workspace_id) {
			const parent = workspaceNames[w.parent_workspace_id] ?? w.parent_workspace_id
			return w.is_dev_workspace ? `dev workspace of ${parent}` : `fork of ${parent}`
		}
		return 'fork whose parent was deleted'
	}
</script>

<ConfirmationModal
	open={confirmEnable}
	title="Turn on dev workspace isolation"
	confirmationText="Turn on"
	type="info"
	onCanceled={() => {
		confirmEnable = false
		toggleChecked = enabled
	}}
	onConfirmed={async () => {
		confirmEnable = false
		await saveSuffix(suffixDraft.trim() || DEFAULT_DEV_WORKSPACE_TAG_SUFFIX)
	}}
>
	<div class="flex flex-col gap-2 text-sm">
		<p>
			{#if workspaces === undefined}
				Every dev workspace and fork
			{:else}
				<b>{pluralize(devWorkspaces.length, 'dev workspace or fork', 'dev workspaces and forks')}</b
				>
			{/if}
			will have their jobs queued until a worker group serves them.
		</p>
		<p class="text-secondary text-xs">
			{#if flaggedGroups.length === 0}
				No worker group is flagged as serving dev workspaces yet.
			{:else}
				The worker groups serving dev workspaces ({flaggedGroups.map((g) => g.name).join(', ')})
				have no live worker.
			{/if}
			Jobs never fall back to the other worker groups: they wait in the queue and run as soon as a serving
			worker is up.
		</p>
		{#if unflaggedGroupNames.length > 0 && flaggedGroups.length === 0}
			<div>
				<Button
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: Settings }}
					onClick={() => {
						confirmEnable = false
						toggleChecked = enabled
						onConfigureGroup(unflaggedGroupNames[0])
					}}
				>
					Flag a worker group first
				</Button>
			</div>
		{/if}
	</div>
</ConfirmationModal>

<ConfirmationModal
	open={confirmDisable}
	title="Turn off dev workspace isolation"
	confirmationText="Turn off"
	type="danger"
	onCanceled={() => {
		confirmDisable = false
		toggleChecked = enabled
	}}
	onConfirmed={async () => {
		confirmDisable = false
		await saveSuffix(undefined)
	}}
>
	<p class="text-sm">
		Jobs of dev workspaces and forks will run on the same worker groups as root workspaces again.
		Jobs already queued on a <code>{effectiveSuffix}</code> tag stay on it and still need a group serving
		dev workspaces.
	</p>
</ConfirmationModal>

<Section
	label="Dev workspace isolation"
	tooltip="Dev workspaces and forks run code that has not been reviewed or deployed to a root workspace. Route their jobs to worker groups of their own, so they never run on the workers of your root workspaces."
	eeOnly={!$enterpriseLicense}
	class="mt-4 mb-6 rounded-md border border-light p-4 bg-surface-tertiary"
>
	{#snippet labelExtra()}
		{#if suffix !== undefined}
			<Badge color={enabled ? 'green' : 'gray'} small>{enabled ? 'On' : 'Off'}</Badge>
		{/if}
	{/snippet}

	<div class="flex flex-col gap-4">
		<Toggle
			bind:checked={toggleChecked}
			on:change={(e) => onToggle(e.detail)}
			disabled={!canEdit || !$enterpriseLicense || saving || suffix === undefined}
			options={{ right: 'Run dev workspaces and forks on separate workers' }}
			class="w-fit"
		/>

		<div class="grid grid-cols-[auto_auto_1fr] gap-x-3 gap-y-2 items-center text-xs max-w-2xl">
			<span class="font-semibold text-emphasis">Root workspaces</span>
			<ArrowRight size={14} class="text-secondary" />
			<span class="text-primary">
				Worker groups not serving dev workspaces, on their usual tags:
				<code>python3</code>, <code>gpu</code>
			</span>
			<span class="font-semibold text-emphasis">Dev workspaces and forks</span>
			<ArrowRight size={14} class="text-secondary" />
			<span class="text-primary">
				{#if enabled}
					Worker groups serving dev workspaces, on tags suffixed with
					<code>{effectiveSuffix}</code>:
				{:else}
					Once on, worker groups serving dev workspaces, on suffixed tags:
				{/if}
				<code>{withDevSuffix('python3', effectiveSuffix)}</code>,
				<code>{withDevSuffix('gpu', effectiveSuffix)}</code>
			</span>
		</div>
		<p class="text-2xs text-secondary max-w-2xl">
			A workspace with a parent, and every fork (including a fork whose parent was deleted), counts
			as a dev workspace. Every job is covered: runs, previews, schedules, triggers, flow steps, AI
			agent tools and dependency jobs. A job whose tag no serving worker listens to stays queued: it
			never falls back to the other worker groups. Dedicated workers keep their own tags.
		</p>

		<!-- Worker groups serving dev workspaces -->
		<div class="flex flex-col gap-2">
			<span class="text-xs font-semibold text-emphasis">Worker groups serving dev workspaces</span>
			{#if flaggedGroups.length > 0}
				<div class="flex flex-wrap gap-2">
					{#each flaggedGroups as g (g.name)}
						<Button
							variant="default"
							unifiedSize="sm"
							startIcon={{ icon: Settings }}
							onClick={() => onConfigureGroup(g.name)}
						>
							{g.name}
							<span class={g.live > 0 ? 'text-secondary' : 'text-red-500'}>
								· {pluralize(g.live, 'live worker')}
							</span>
						</Button>
					{/each}
				</div>
			{:else}
				<span class="text-xs text-secondary">None yet.</span>
			{/if}
			{#if canEdit && unflaggedGroupNames.length > 0}
				<div class="flex gap-2 items-center">
					<Select
						size="sm"
						class="w-56"
						placeholder="Pick a worker group"
						items={unflaggedGroupNames.map((n) => ({ value: n }))}
						bind:value={groupToFlag}
					/>
					<Button
						variant="default"
						unifiedSize="sm"
						disabled={!groupToFlag}
						onClick={() => groupToFlag && onConfigureGroup(groupToFlag)}
					>
						Flag as serving dev workspaces
					</Button>
					<Tooltip>
						{#snippet text()}
							Opens the group's config, where "Serves dev workspaces and forks" sits under its tags.
							A dedicated group to start from: create a new group config, flag it, and start workers
							with its name as <code>WORKER_GROUP</code>.
						{/snippet}
					</Tooltip>
				</div>
			{/if}
		</div>

		{#if enabled && liveFlaggedWorkers === 0}
			<Alert type="warning" size="xs" title="No worker serves dev workspaces">
				Jobs of dev workspaces and forks stay queued until a worker group flagged as serving dev
				workspaces has live workers{flaggedGroups.length === 0
					? ': flag a worker group as serving dev workspaces'
					: ''}.
			</Alert>
		{/if}

		{#if unservedDevTags.length > 0}
			<Alert type="warning" size="xs" title="Dev jobs waiting with no worker">
				No live worker listens to
				{#each unservedDevTags as s, i (s.tag)}
					{i > 0 ? ', ' : ''}<code>{s.tag}</code> ({s.waiting} waiting)
				{/each}. These tags carry the <code>{effectiveSuffix}</code> suffix of dev workspace jobs: add
				the unsuffixed tag to a worker group flagged as serving dev workspaces, or flag a group that
				already lists it.
			</Alert>
		{/if}

		{#if leakingGroups.length > 0}
			<Alert
				type="warning"
				size="xs"
				title="A worker group not serving dev workspaces takes dev jobs"
			>
				{#each leakingGroups as g, i (g.name)}
					{i > 0 ? '; ' : ''}<b>{g.name}</b> lists
					{#each g.tags as t, j (t)}{j > 0 ? ', ' : ''}<code>{t}</code>{/each}
				{/each}. Jobs of dev workspaces and forks run on these tags, so this group runs them next to
				root workspace jobs. Remove the tags, or flag the group as serving dev workspaces.
			</Alert>
		{/if}

		<!-- Affected workspaces -->
		<div class="flex flex-col gap-1">
			<div class="w-fit">
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: showWorkspaces ? ChevronDown : ChevronRight }}
					onClick={() => (showWorkspaces = !showWorkspaces)}
				>
					{#if workspaces === undefined}
						Workspaces routed to dev workers
					{:else}
						{pluralize(devWorkspaces.length, 'workspace')}
						{enabled ? 'routed' : 'that would be routed'} to dev workers
					{/if}
				</Button>
			</div>
			{#if showWorkspaces}
				{#if workspaces === undefined}
					<span class="text-xs text-secondary">Loading…</span>
				{:else if devWorkspaces.length === 0}
					<span class="text-xs text-secondary"
						>No dev workspace or fork yet. Only root workspaces exist.</span
					>
				{:else}
					<ul class="flex flex-col gap-1 text-xs max-h-60 overflow-auto">
						{#each devWorkspaces as w (w.id)}
							<li class="flex gap-2">
								<span class="font-mono text-primary">{w.id}</span>
								<span class="text-secondary">{workspaceKind(w)}</span>
							</li>
						{/each}
					</ul>
				{/if}
			{/if}
		</div>

		<!-- Advanced -->
		<div class="flex flex-col gap-2">
			<div class="w-fit">
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: showAdvanced ? ChevronDown : ChevronRight }}
					onClick={() => (showAdvanced = !showAdvanced)}
				>
					Advanced
				</Button>
			</div>
			{#if showAdvanced}
				<div class="flex flex-col gap-1 max-w-md">
					<span class="text-xs font-semibold text-emphasis">Tag suffix</span>
					<span class="text-2xs text-secondary">
						Appended last, after every other tag setting: with workspace-specific default tags a
						fork's <code>python3-$workspace-fork</code> becomes
						<code>{withDevSuffix('python3-$workspace-fork', effectiveSuffix)}</code>. Changing it
						re-tags worker groups serving dev workspaces; jobs already queued keep their tag.
					</span>
					<div class="flex gap-2 items-start">
						<TextInput
							bind:value={suffixDraft}
							size="sm"
							error={suffixError}
							inputProps={{
								placeholder: DEFAULT_DEV_WORKSPACE_TAG_SUFFIX,
								disabled: !canEdit || !$enterpriseLicense
							}}
						/>
						{#if enabled}
							<Button
								variant="accent"
								unifiedSize="sm"
								startIcon={{ icon: Save }}
								disabled={!canEdit ||
									!!suffixError ||
									suffixDraft.trim() === effectiveSuffix ||
									saving}
								onClick={() => saveSuffix(suffixDraft.trim())}
							>
								Save
							</Button>
						{/if}
					</div>
					{#if suffixError}
						<span class="text-2xs text-red-500">{suffixError}</span>
					{:else if !enabled}
						<span class="text-2xs text-secondary">Used when you turn isolation on.</span>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</Section>
