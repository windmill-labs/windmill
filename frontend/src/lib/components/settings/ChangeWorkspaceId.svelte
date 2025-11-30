<script lang="ts">
	import { workspaceStore, superadmin } from '$lib/stores'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService, JobService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen, Loader2 } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { onDestroy } from 'svelte'
	import { hubPaths } from '$lib/hub'

	let { open = $bindable(false) } = $props()

	let newName = $state('')
	let newId = $state('')

	$effect(() => {
		if (!incompleteMigration && newName) {
			newId = newName.toLowerCase().replace(/\s/gi, '-')
		}
	})
	let checking = $state(false)
	let errorId = $state('')

	$effect(() => {
		validateName(newId)
	})

	async function validateName(id: string): Promise<void> {
		checking = true
		let exists = await WorkspaceService.existsWorkspace({ requestBody: { id } })
		if (exists) {
			errorId = 'ID already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		} else {
			errorId = ''
		}
		checking = false
	}

	let loading = $state(false)
	let workspaceDataMigrated = $state(false)
	let oldWorkspaceId = $state('')

	let jobMigrationJobId = $state<string | undefined>(undefined)
	let migratingJobs = $state(false)
	let jobMigrationComplete = $state(false)
	let pollInterval: number | null = null

	let migrationInProgress = $state(false)
	let migrationError = $state<string | null>(null)

	const isWorkspaceDataStep = $derived(migrationInProgress && !workspaceDataMigrated)
	const isJobsStep = $derived(migratingJobs)
	const isComplete = $derived(jobMigrationComplete)
	const isError = $derived(!!migrationError)

	let incompleteMigration = $state<{
		targetWorkspaceId: string
		sourceWorkspaceId: string
	} | null>(null)
	let checkingIncomplete = $state(false)

	async function checkForIncompleteMigration() {
		if (!$workspaceStore) return

		console.log('Checking for incomplete migration for workspace:', $workspaceStore)
		try {
			checkingIncomplete = true
			const targetWorkspaceId = await WorkspaceService.getIncompleteMigration({
				workspace: $workspaceStore
			})

			console.log('Incomplete migration check result:', targetWorkspaceId)
			if (targetWorkspaceId) {
				incompleteMigration = {
					targetWorkspaceId,
					sourceWorkspaceId: $workspaceStore
				}
				oldWorkspaceId = $workspaceStore
				newId = targetWorkspaceId
				newName = targetWorkspaceId.replace(/-/g, ' ')
				workspaceDataMigrated = true
			}
		} catch (err: any) {
			console.error('Error checking for incomplete migration:', err)
		} finally {
			checkingIncomplete = false
		}
	}

	async function checkJobStatus() {
		if (!jobMigrationJobId) return

		try {
			const jobResult = await JobService.getCompletedJobResultMaybe({
				workspace: $workspaceStore!,
				id: jobMigrationJobId
			})
			if (jobResult.completed) {
				migratingJobs = false
				stopPolling()
				if (jobResult.success) {
					jobMigrationComplete = true
					sendUserToast('Job migration completed!')
					try {
						await WorkspaceService.completeWorkspaceMigration({
							requestBody: {
								source_workspace_id: $workspaceStore!,
								target_workspace_id: newId
							}
						})
						sendUserToast(`Migration completed and old workspace deleted`)
						window.location.href = `/workspace_settings?tab=general&workspace=${newId}`
					} catch (error) {
						throw error
					}
				} else {
					throw Error(JSON.stringify(jobResult.result))
				}
			}
		} catch (err: any) {
			stopPolling()
			migratingJobs = false
			migrationInProgress = false
			migrationError = err.body || err.message
			sendUserToast(`Migration error: ${err.body || err.message}`, true)
		}
	}

	async function cancelJobMigration() {
		if (!jobMigrationJobId) return

		try {
			await JobService.cancelQueuedJob({
				workspace: $workspaceStore!,
				id: jobMigrationJobId,
				requestBody: {
					reason: 'User cancelled migration'
				}
			})

			sendUserToast('Migration cancelled')
			stopPolling()
			migratingJobs = false
			migrationInProgress = false
			incompleteMigration = {
				sourceWorkspaceId: $workspaceStore!,
				targetWorkspaceId: newId
			}
		} catch (err: any) {
			sendUserToast(`Failed to cancel: ${err.body || err.message}`, true)
		}
	}

	function startPolling() {
		if (!pollInterval) {
			pollInterval = setInterval(checkJobStatus, 1000) as any
		}
	}

	function stopPolling() {
		if (pollInterval) {
			clearInterval(pollInterval)
			pollInterval = null
		}
	}

	async function performFullMigration(ignore_workspace_data?: boolean) {
		try {
			migrationInProgress = true
			migrationError = null
			loading = true
			oldWorkspaceId = $workspaceStore!

			if (!ignore_workspace_data) {
				await WorkspaceService.migrateWorkspaceTables({
					requestBody: {
						source_workspace_id: oldWorkspaceId,
						target_workspace_id: newId,
						target_workspace_name: newName
					}
				})
				workspaceDataMigrated = true
				sendUserToast(`Workspace data migrated to ${newId}`)
			}

			migratingJobs = true

			jobMigrationJobId = await JobService.runScriptByPath({
				workspace: $workspaceStore!,
				path: hubPaths.workspaceMigrator,
				requestBody: {
					source_workspace_id: oldWorkspaceId,
					target_workspace_id: newId
				},
				skipPreprocessor: true
			})

			startPolling()
			loading = false
		} catch (err: any) {
			migrationError = err.body || err.message
			sendUserToast(`Migration failed: ${migrationError}`, true)
		} finally {
			if (!migratingJobs) {
				loading = false
				migrationInProgress = false
			}
		}
	}

	async function revertMigration() {
		const sourceWorkspaceId = incompleteMigration?.sourceWorkspaceId || oldWorkspaceId
		const targetWorkspaceId = incompleteMigration?.targetWorkspaceId || newId

		if (!sourceWorkspaceId || !targetWorkspaceId) return

		try {
			loading = true

			await WorkspaceService.revertWorkspaceMigration({
				workspace: sourceWorkspaceId,
				requestBody: {
					target_workspace_id: targetWorkspaceId
				}
			})

			sendUserToast('Migration reverted successfully!')

			incompleteMigration = null
			workspaceDataMigrated = false
			migratingJobs = false
			jobMigrationComplete = false
			migrationInProgress = false
			migrationError = null
			newName = ''
			newId = ''
			oldWorkspaceId = ''
			errorId = ''
		} catch (err: any) {
			sendUserToast(`Failed to revert migration: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	onDestroy(() => {
		stopPolling()
	})
</script>

<div class="flex flex-col gap-1">
	<p class="font-medium text-xs text-emphasis">Workspace ID</p>
	<div class="flex flex-row gap-0.5 items-center">
		<p class="text-xs font-normal text-primary">{$workspaceStore ?? ''}</p>
		{#if !isCloudHosted() || $superadmin}
			<Button
				on:click={() => {
					checkForIncompleteMigration()
					open = true
				}}
				size="xs"
				spacingSize="xs2"
				color="light"
				iconOnly
				startIcon={{
					icon: Pen
				}}
			/>
		{/if}
	</div>
	<p class="text-xs text-secondary font-normal">Slug to uniquely identify your workspace</p>
</div>

<Modal
	bind:open
	title={isComplete
		? 'Migration Complete'
		: migrationInProgress
			? isWorkspaceDataStep
				? 'Step 1: Migrating workspace data'
				: 'Step 2: Migrating Jobs'
			: 'Change Workspace ID'}
>
	<div class="flex flex-col gap-6">
		{#if checkingIncomplete}
			<Alert type="info" title="Checking for incomplete migrations...">
				<p class="text-sm">Please wait while we check for any incomplete workspace migrations.</p>
			</Alert>
		{:else if migrationInProgress || migratingJobs}
			<div
				class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-6 rounded-lg"
			>
				<div class="flex items-center gap-4 mb-4">
					<Loader2 class="w-12 h-12 animate-spin text-blue-500" />
					<div>
						<h3 class="font-semibold text-blue-900 dark:text-blue-100">Migration in Progress</h3>
						<p class="text-sm text-blue-700 dark:text-blue-300">
							{#if isWorkspaceDataStep}
								Migrating workspace data (scripts, flows, apps, settings)...
							{:else if isJobsStep}
								Migrating job history - this may take several minutes
							{:else}
								This process may take several minutes depending on job history size
							{/if}
						</p>
					</div>
				</div>
				<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
					<div
						class="bg-blue-500 h-2 rounded-full transition-all duration-500"
						style="width: {isWorkspaceDataStep ? '30%' : isJobsStep ? '70%' : '100%'}"
					></div>
				</div>
			</div>

			<div class="bg-surface-secondary p-4 rounded-lg border">
				<p class="text-sm font-medium mb-3">Migration Progress</p>
				<div class="space-y-2">
					<div class="flex items-center gap-3">
						<div
							class="w-4 h-4 rounded-full {!isWorkspaceDataStep && workspaceDataMigrated
								? 'bg-green-500'
								: 'bg-blue-500 animate-pulse'} flex items-center justify-center"
						>
							{#if !isWorkspaceDataStep && workspaceDataMigrated}
								<span class="text-white text-xs">✓</span>
							{/if}
						</div>
						<span class="text-sm">Step 1: Migrate workspace data</span>
					</div>
					<div class="flex items-center gap-3">
						<div
							class="w-4 h-4 rounded-full {isComplete
								? 'bg-green-500'
								: isJobsStep
									? 'bg-blue-500 animate-pulse'
									: 'bg-gray-300 dark:bg-gray-600'} flex items-center justify-center"
						>
							{#if isComplete}
								<span class="text-white text-xs">✓</span>
							{/if}
						</div>
						<span class="text-sm {isJobsStep || isComplete ? '' : 'text-gray-500'}"
							>Step 2: Migrate job</span
						>
					</div>
				</div>
				<div
					class="space-y-1 text-sm text-secondary mt-3 pt-3 border-t border-gray-200 dark:border-gray-700"
				>
					<p><span class="font-medium">From:</span> {oldWorkspaceId}</p>
					<p><span class="font-medium">To:</span> {newId}</p>
				</div>
			</div>

			<Alert type="warning" title="Important">
				<p class="text-sm">
					Please do not close this window or navigate away during the migration process. This
					ensures data integrity and allows proper completion tracking.
				</p>
			</Alert>
		{:else if isError && migrationError}
			<Alert type="error" title="Migration Failed">
				<div class="space-y-3">
					<p class="text-sm"> The change workspace ID process failed. </p>
					<details class="text-sm">
						<summary class="cursor-pointer font-medium">Error Details</summary>
						<pre class="mt-2 text-xs bg-gray-100 dark:bg-gray-800 p-2 rounded overflow-auto"
							>{migrationError}</pre
						>
					</details>
				</div>
			</Alert>
		{:else}
			{#if incompleteMigration}
				<Alert type="warning" title="Incomplete Migration Detected">
					<div class="space-y-3">
						<p class="text-sm">
							We found an incomplete migration from workspace <strong
								>{incompleteMigration.sourceWorkspaceId}</strong
							>
							to <strong>{incompleteMigration.targetWorkspaceId}</strong>.
						</p>
						<p class="text-sm"> You can either complete the migration or revert to start over. </p>
					</div>
				</Alert>
			{/if}

			<div
				class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-lg"
			>
				<div class="flex items-center gap-2 mb-2">
					<div class="w-2 h-2 bg-blue-500 rounded-full"></div>
					<span class="text-blue-700 dark:text-blue-300 text-sm font-medium">Current Workspace</span
					>
				</div>
				<p class="font-mono text-lg font-bold text-blue-900 dark:text-blue-100">
					{$workspaceStore ?? ''}
				</p>
			</div>

			<div class="space-y-4">
				<label class="block">
					<span class="text-secondary text-sm font-medium">New Workspace Name</span>
					<input
						type="text"
						bind:value={newName}
						placeholder="Enter new workspace name"
						class="mt-1 w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed"
						disabled={incompleteMigration !== null || migrationInProgress || workspaceDataMigrated}
					/>
				</label>
				<label class="block">
					<span class="text-secondary text-sm font-medium">New Workspace ID</span>
					<input
						type="text"
						bind:value={newId}
						placeholder="auto-generated from name"
						class="mt-1 w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed"
						disabled={incompleteMigration !== null || migrationInProgress || workspaceDataMigrated}
					/>
					{#if errorId && incompleteMigration == null}
						<div class="text-red-500 text-xs mt-1 flex items-center gap-1">
							<span class="w-3 h-3 text-red-500">⚠</span>
							{errorId}
						</div>
					{/if}
				</label>
			</div>

			<Alert type="warning" title="Important Notice">
				<p class="text-sm">
					Once finished, please update your webhook calls and adjust your CLI sync configuration
					accordingly.
				</p>
			</Alert>
		{/if}
	</div>

	<svelte:fragment slot="actions">
		{#if migrationInProgress && migratingJobs}
			<Button
				onclick={(e) => {
					e.preventDefault()
					cancelJobMigration()
				}}
				color="red"
				size="sm"
			>
				Cancel Migration
			</Button>
		{:else if incompleteMigration}
			<div class="flex gap-3 w-full">
				<Button
					size="sm"
					color="red"
					onclick={(e) => {
						e.preventDefault()
						revertMigration()
					}}
					{loading}
				>
					Revert Migration
				</Button>
				<Button
					size="sm"
					disabled={loading}
					onclick={async (e) => {
						e.preventDefault()
						incompleteMigration = null
						workspaceDataMigrated = true
						await performFullMigration(true)
					}}
					color="blue"
				>
					Complete Migration
				</Button>
			</div>
		{:else if isError}
			<div class="flex gap-3">
				<Button
					size="sm"
					color="red"
					onclick={(e) => {
						e.preventDefault()
						revertMigration()
					}}
					{loading}
				>
					Revert Migration
				</Button>
				<Button
					size="sm"
					color="blue"
					onclick={async (e) => {
						e.preventDefault()
						migrationError = null
						migrationInProgress = false
						await performFullMigration(!isWorkspaceDataStep)
					}}
				>
					Try Again
				</Button>
			</div>
		{:else}
			<Button
				size="sm"
				disabled={checking ||
					errorId.length > 0 ||
					!newName ||
					!newId ||
					loading ||
					migrationInProgress}
				{loading}
				onclick={async (e) => {
					e.preventDefault()
					await performFullMigration()
				}}
				color="blue"
			>
				Change Workspace
			</Button>
		{/if}
	</svelte:fragment>
</Modal>
