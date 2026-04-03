<script module lang="ts">
	export type ForkStep = {
		label: string
		status: 'pending' | 'running' | 'done' | 'error'
		error?: string
	}

	export type DatatableCloneJob = {
		name: string
		resourceType: string
		behavior: 'schema_only' | 'schema_and_data'
		steps: ForkStep[]
		_newDbName: string
		_isInstance: boolean
		_sourceWorkspace: string
		_targetWorkspace: string
		_resourcePath: string
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
	import { Check, X, Loader2 } from 'lucide-svelte'

	interface Props {
		onAllDone?: () => void
		onCanceled?: () => void
	}

	let { onAllDone, onCanceled }: Props = $props()

	let allDatatables = resource([], async () =>
		$workspaceStore
			? WorkspaceService.listDataTables({ workspace: $workspaceStore ?? '' })
			: undefined
	)

	let datatableBehaviors: Record<string, 'schema_only' | 'schema_and_data' | 'keep_original'> =
		$state({})

	let cloneModalOpen = $state(false)
	let currentCloneJob: DatatableCloneJob | undefined = $state(undefined)
	let cloneQueue: DatatableCloneJob[] = $state([])
	let cloneRunning = $state(false)

	export function hasDatatables(): boolean {
		return (allDatatables.current?.length ?? 0) > 0
	}

	export function buildCloneQueue(targetWorkspaceId: string): DatatableCloneJob[] {
		return (allDatatables.current ?? [])
			.filter((dt) => {
				const behavior = datatableBehaviors[dt.name] ?? 'keep_original'
				return behavior !== 'keep_original'
			})
			.map((dt) => {
				const behavior = datatableBehaviors[dt.name] as 'schema_only' | 'schema_and_data'
				const isInstance = dt.resource_type === 'instance'
				const newDbName = `${targetWorkspaceId.replace(/-/g, '_')}__${dt.name}`

				const steps: ForkStep[] = [
					{
						label: `CREATE DATABASE "${newDbName}"`,
						status: 'pending'
					},
					{
						label: `pg_dump → pg_import (${behavior === 'schema_only' ? 'schema only' : 'schema + data'})`,
						status: 'pending'
					}
				]

				return {
					name: dt.name,
					resourceType: dt.resource_type,
					behavior,
					steps,
					_newDbName: newDbName,
					_isInstance: isInstance,
					_sourceWorkspace: $workspaceStore!,
					_targetWorkspace: targetWorkspaceId,
					_resourcePath: dt.resource_path
				}
			})
	}

	let completedJobs: DatatableCloneJob[] = $state([])

	export function startCloning(queue: DatatableCloneJob[]) {
		completedJobs = []
		cloneQueue = queue
		currentCloneJob = cloneQueue[0]
		cloneModalOpen = true
	}

	export function getCompletedCloneJobs(): DatatableCloneJob[] {
		return completedJobs
	}

	async function executeCloneJob(job: DatatableCloneJob) {
		cloneRunning = true
		let stepIdx = 0

		// Step 1: Create the database
		job.steps[stepIdx].status = 'running'
		try {
			await WorkspaceService.createPgDatabase({
				workspace: job._sourceWorkspace,
				requestBody: {
					source: `datatable://${job.name}`,
					target_dbname: job._newDbName
				}
			})
			job.steps[stepIdx].status = 'done'
		} catch (e: any) {
			job.steps[stepIdx].status = 'error'
			job.steps[stepIdx].error = e?.body ?? e?.message ?? String(e)
			cloneRunning = false
			return
		}
		stepIdx++

		// Step 2: Import data
		job.steps[stepIdx].status = 'running'
		try {
			await WorkspaceService.importPgDatabase({
				workspace: job._sourceWorkspace,
				requestBody: {
					source: `datatable://${job.name}`,
					target: `datatable://${job.name}`,
					target_dbname_override: job._newDbName,
					fork_behavior: job.behavior
				}
			})
			job.steps[stepIdx].status = 'done'
		} catch (e: any) {
			job.steps[stepIdx].status = 'error'
			job.steps[stepIdx].error = e?.body ?? e?.message ?? String(e)
			cloneRunning = false
			return
		}
		stepIdx++

		cloneRunning = false
	}

	function advanceCloneQueue() {
		if (currentCloneJob) {
			completedJobs.push(currentCloneJob)
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
		confirmationText={cloneRunning ? 'Running...' : 'Start'}
		open={cloneModalOpen}
		loading={cloneRunning}
		onConfirmed={async () => {
			await executeCloneJob(currentCloneJob!)
			advanceCloneQueue()
		}}
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
				This will run <code
					>CREATE DATABASE {currentCloneJob.steps[0]?.label.match(/"([^"]+)"/)?.[1] ?? ''}</code
				> on the Windmill PostgreSQL instance.
			</p>
		{:else}
			<p class="text-xs text-secondary mt-2">
				This will run <code>CREATE DATABASE</code> on the resource's PostgreSQL server.
			</p>
		{/if}

		<div class="mt-4 flex flex-col gap-2">
			{#each currentCloneJob.steps as step}
				<div class="flex items-center gap-2 text-xs">
					{#if step.status === 'done'}
						<Check class="w-4 h-4 shrink-0 text-green-500" />
					{:else if step.status === 'running'}
						<Loader2 class="w-4 h-4 shrink-0 animate-spin text-blue-500" />
					{:else if step.status === 'error'}
						<X class="w-4 h-4 shrink-0 text-red-500" />
					{:else}
						<div class="w-4 h-4 shrink-0 rounded-full border border-gray-300"></div>
					{/if}
					<span
						class:text-tertiary={step.status === 'pending'}
						class:font-medium={step.status === 'running'}
					>
						{step.label}
					</span>
				</div>
				{#if step.error}
					<p class="text-2xs text-red-500 ml-6">{step.error}</p>
				{/if}
			{/each}
		</div>
	</ConfirmationModal>
{/if}
