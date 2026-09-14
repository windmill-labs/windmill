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
	import { workspaceStore, userStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import Label from '../Label.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'

	interface Props {
		// Workspace whose datatables are cloned into the fork (the fork's base). Falls back to the
		// current workspace so existing callers keep working.
		sourceWorkspace?: string
		onAllDone?: () => void
		onCanceled?: () => void
	}

	let { sourceWorkspace, onAllDone, onCanceled }: Props = $props()

	let effectiveSource = $derived(sourceWorkspace ?? $workspaceStore ?? undefined)

	let allDatatables = resource(
		() => effectiveSource,
		async (ws) => (ws ? WorkspaceService.listDataTables({ workspace: ws }) : undefined)
	)

	let datatableBehaviors: Record<string, 'schema_only' | 'schema_and_data' | 'keep_original'> =
		$state({})

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
			.filter((dt) => {
				const behavior = datatableBehaviors[dt.name] ?? 'keep_original'
				return behavior !== 'keep_original'
			})
			.map((dt) => ({
				name: dt.name,
				resourceType: dt.resource_type,
				behavior: datatableBehaviors[dt.name] as 'schema_only' | 'schema_and_data',
				_newDbName: `${targetWorkspaceId.replace(/-/g, '_')}__${dt.name}`
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
	<Label label="Data table behavior">
		<span class="text-xs text-secondary"> Choose how to handle each datatable when forking </span>
		<div class="border rounded-md divide-y">
			{#each allDatatables.current as dt}
				<div class="flex items-center gap-2 justify-between px-4 py-1.5">
					<div class="flex flex-col">
						<span class="text-xs font-medium">{dt.name}</span>
						<span class="text-2xs text-tertiary"
							>{dt.resource_type === 'instance' ? 'Instance DB' : 'Resource DB'}</span
						>
					</div>
					<Select
						dropdownClass="max-w-96"
						bind:value={
							() => datatableBehaviors[dt.name] ?? 'keep_original',
							(v) => (datatableBehaviors[dt.name] = v)
						}
						items={[
							{ value: 'keep_original', label: 'Keep original' },
							{ value: 'schema_only', label: 'Clone schema only' },
							...(!isCloudHosted() && $userStore?.is_admin
								? [{ value: 'schema_and_data', label: 'Clone schema and data' }]
								: [])
						]}
					/>
				</div>
			{/each}
		</div>
	</Label>
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
