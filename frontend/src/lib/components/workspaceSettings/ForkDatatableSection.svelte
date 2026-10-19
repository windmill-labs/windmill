<script module lang="ts">
	export type DatatableCloneJob = {
		name: string
		resourceType: string
		behavior: 'schema_only' | 'schema_and_data'
		_newDbName: string
	}
</script>

<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore, userStore, enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import { TriangleAlert } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'

	interface Props {
		// Workspace whose datatables are cloned into the fork (the fork's base). Falls back to the
		// current workspace so existing callers keep working.
		sourceWorkspace?: string
		// A protection rule on the base forbids `keep_original` for the base's own datatables.
		requireIsolation?: boolean
		itemCount?: number
		onAllDone?: () => void
		onCanceled?: () => void
	}

	let {
		sourceWorkspace,
		requireIsolation = false,
		itemCount = $bindable(),
		onAllDone,
		onCanceled
	}: Props = $props()

	let effectiveSource = $derived(sourceWorkspace ?? $workspaceStore ?? undefined)

	let allDatatables = resource(
		() => effectiveSource,
		async (ws) => (ws ? WorkspaceService.listDataTables({ workspace: ws }) : undefined)
	)

	$effect(() => {
		itemCount = allDatatables.current?.length ?? 0
	})

	type Behavior = 'schema_only' | 'schema_and_data' | 'keep_original'
	let datatableBehaviors: Record<string, Behavior> = $state({})

	// An entry whose database another workspace owns is not the base's data: the rule leaves it
	// alone, as the server does.
	function mustClone(dt: { database_workspace_id?: string }): boolean {
		return requireIsolation && !dt.database_workspace_id
	}

	function behaviorOf(dt: { name: string; database_workspace_id?: string }): Behavior {
		const chosen = datatableBehaviors[dt.name] ?? 'keep_original'
		return mustClone(dt) && chosen === 'keep_original' ? 'schema_only' : chosen
	}

	let anyShared = $derived(
		(allDatatables.current ?? []).some((dt) => behaviorOf(dt) === 'keep_original')
	)

	let cloneModalOpen = $state(false)
	let currentCloneJob: DatatableCloneJob | undefined = $state(undefined)
	let cloneQueue: DatatableCloneJob[] = $state([])

	export function hasDatatables(): boolean {
		return (allDatatables.current?.length ?? 0) > 0
	}

	export function buildCloneQueue(targetWorkspaceId: string): DatatableCloneJob[] {
		// A fork attempt starts here: what an earlier attempt confirmed is not this one's to send.
		confirmedJobs = []
		return (allDatatables.current ?? [])
			.filter((dt) => behaviorOf(dt) !== 'keep_original')
			.map((dt) => ({
				name: dt.name,
				resourceType: dt.resource_type,
				behavior: behaviorOf(dt) as 'schema_only' | 'schema_and_data',
				// A dev workspace's id has no `wm-fork-` prefix; its copies are named like a fork's
				_newDbName: `wm_fork_${targetWorkspaceId.replace(/^wm-fork-/, '').replace(/-/g, '_')}__${dt.name}`
			}))
	}

	let confirmedJobs: DatatableCloneJob[] = $state([])

	// Each clone is only confirmed here: the fork request makes the copies, and drops them if the
	// fork is not created.
	export function startCloning(queue: DatatableCloneJob[]) {
		confirmedJobs = []
		cloneQueue = queue
		currentCloneJob = cloneQueue[0]
		cloneModalOpen = true
	}

	export function getConfirmedCloneJobs(): DatatableCloneJob[] {
		return confirmedJobs
	}

	function advanceCloneQueue() {
		if (currentCloneJob) {
			confirmedJobs.push(currentCloneJob)
		}
		const idx = cloneQueue.indexOf(currentCloneJob!)
		if (idx < cloneQueue.length - 1) {
			currentCloneJob = cloneQueue[idx + 1]
		} else {
			cloneModalOpen = false
			currentCloneJob = undefined
			cloneQueue = []
			onAllDone?.()
		}
	}
</script>

{#if allDatatables.current && allDatatables.current.length > 0}
	<div class="flex flex-col gap-1">
		<span class="text-xs font-medium text-secondary">Data tables</span>
		<div class="border rounded-md divide-y">
			{#each allDatatables.current as dt (dt.name)}
				<div class="flex items-center gap-2 justify-between px-4 py-1.5">
					<div class="flex flex-col">
						<span class="text-xs font-medium">{dt.name}</span>
						<span class="text-2xs text-tertiary"
							>{dt.resource_type === 'instance' ? 'Instance DB' : 'Resource DB'}</span
						>
					</div>
					<Select
						dropdownClass="max-w-96"
						bind:value={() => behaviorOf(dt), (v) => (datatableBehaviors[dt.name] = v)}
						items={[
							...(mustClone(dt)
								? []
								: [
										{
											value: 'keep_original' as const,
											label: dt.permissioned
												? 'Shared with parent (under roles)'
												: 'Shared with parent'
										}
									]),
							// A copy of a data table under roles carries the source's owners and grants,
							// which only the Enterprise Edition replays: elsewhere the server refuses it.
							...(dt.permissioned && !$enterpriseLicense
								? []
								: [
										{ value: 'schema_only', label: 'Clone schema only' },
										...(!isCloudHosted() && $userStore?.is_admin
											? [{ value: 'schema_and_data', label: 'Clone schema and data' }]
											: [])
									])
						]}
					/>
				</div>
			{/each}
		</div>
		{#if anyShared}
			<span class="flex items-center gap-1 text-2xs text-amber-700 dark:text-amber-300">
				<TriangleAlert size={12} class="shrink-0" />
				Scripts in the fork will read and write this workspace's rows.
			</span>
		{/if}
	</div>
{/if}

{#if cloneModalOpen && currentCloneJob}
	<ConfirmationModal
		title="Clone datatable: {currentCloneJob.name}"
		confirmationText="Confirm"
		open={cloneModalOpen}
		onConfirmed={advanceCloneQueue}
		onCanceled={() => {
			cloneModalOpen = false
			currentCloneJob = undefined
			cloneQueue = []
			onCanceled?.()
		}}
	>
		{#if currentCloneJob.behavior === 'schema_and_data'}
			<Alert type="error" title="Heavy operation">
				This will copy the <b>entire database</b> including all data. The pg_dump output is temporarily
				stored on disk and may consume significant server disk space during the operation.
			</Alert>
		{:else}
			<Alert type="info" title="Schema only">
				This will copy the database schema only. All tables will be empty. This is a lightweight
				operation.
			</Alert>
		{/if}

		{#if currentCloneJob.resourceType === 'instance'}
			<p class="text-xs text-secondary mt-2">
				Creating the fork will run <code>CREATE DATABASE {currentCloneJob._newDbName}</code> on the Windmill
				PostgreSQL instance. A data table under roles keeps its owners and grants in the copy, and its
				roles stay decided where they are decided today.
			</p>
		{:else}
			<p class="text-xs text-secondary mt-2">
				Creating the fork will run <code>CREATE DATABASE {currentCloneJob._newDbName}</code> on the resource's
				PostgreSQL server.
			</p>
		{/if}
		<p class="text-xs text-secondary mt-2"> If the fork cannot be created, the copy is dropped. </p>
	</ConfirmationModal>
{/if}
