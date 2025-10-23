<script lang="ts">
	import { workspaceStore, superadmin } from '$lib/stores'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService, JobService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen, Loader2 } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { onDestroy, onMount } from 'svelte'
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
	let metadataMigrated = $state(false)
	let oldWorkspaceId = $state('')

	let jobMigrationStarted = $state(false)
	let jobMigrationJobId = $state<string | undefined>(undefined)
	let migratingJobs = $state(false)
	let jobMigrationComplete = $state(false)
	let pollInterval: number | null = null

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
				metadataMigrated = true
			}
		} catch (err: any) {
			console.error('Error checking for incomplete migration:', err)
		} finally {
			checkingIncomplete = false
		}
	}

	onMount(() => {
		checkForIncompleteMigration()
	})

	async function migrateMetadata() {
		try {
			loading = true
			oldWorkspaceId = $workspaceStore!

			await WorkspaceService.migrateWorkspaceTables({
				requestBody: {
					source_workspace_id: oldWorkspaceId,
					target_workspace_id: newId,
					target_workspace_name: newName
				}
			})

			metadataMigrated = true
			// Keep modal open to show step 2

			sendUserToast(`Workspace metadata migrated to ${newId}`)
		} catch (err: any) {
			sendUserToast(`Error migrating workspace metadata: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function checkJobStatus() {
		if (!jobMigrationJobId) return

		try {
			const result = await JobService.getCompletedJob({
				workspace: newId,
				id: jobMigrationJobId
			})
			const jobSuccess = !!result.success
			if (!jobSuccess) {
				throw Error('err')
			}
			jobMigrationComplete = true
			migratingJobs = false
			stopPolling()
			sendUserToast('Job migration completed!')
		} catch (err: any) {
			stopPolling()
			migratingJobs = false
			sendUserToast(`Migration error: ${err.body || err.message}`, true)
		}
	}

	async function startJobMigration() {
		try {
			migratingJobs = true

			jobMigrationJobId = await JobService.runScriptByPath({
				workspace: $workspaceStore!,
				path: hubPaths.workspaceMigrator,
				requestBody: {
					args: {
						source_workspace_id: oldWorkspaceId,
						target_workspace_id: newId
					}
				}
			})

			jobMigrationStarted = true

			startPolling()
		} catch (err: any) {
			sendUserToast(`Migration failed: ${err.body || err.message}`, true)
			migratingJobs = false
		}
	}

	async function cancelJobMigration() {
		if (!jobMigrationJobId) return

		try {
			await JobService.cancelQueuedJob({
				workspace: newId,
				id: jobMigrationJobId,
				requestBody: {
					reason: 'User cancelled migration'
				}
			})

			sendUserToast('Migration cancelled')
			stopPolling()
			migratingJobs = false
		} catch (err: any) {
			sendUserToast(`Failed to cancel: ${err.body || err.message}`, true)
		}
	}

	async function deleteOldWorkspaceAndSwitch() {
		try {
			loading = true

			await WorkspaceService.completeWorkspaceMigration({
				requestBody: {
					source_workspace_id: oldWorkspaceId,
					target_workspace_id: newId
				}
			})

			await WorkspaceService.deleteWorkspace({
				workspace: oldWorkspaceId
			})

			sendUserToast(`Migration completed and old workspace deleted`)

			window.location.href = `/workspace_settings?tab=general&workspace=${newId}`
		} catch (err: any) {
			sendUserToast(`Failed to complete migration: ${err.body || err.message}`, true)
		} finally {
			loading = false
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

	async function completeMigration() {
		if (!incompleteMigration) return

		try {
			loading = true

			await WorkspaceService.completeWorkspaceMigration({
				requestBody: {
					source_workspace_id: incompleteMigration.sourceWorkspaceId,
					target_workspace_id: incompleteMigration.targetWorkspaceId
				}
			})

			sendUserToast('Migration completed successfully!')

			window.location.href = `/workspace_settings?tab=general&workspace=${incompleteMigration.targetWorkspaceId}`
		} catch (err: any) {
			sendUserToast(`Failed to complete migration: ${err.body || err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function revertMigration() {
		// Handle both incomplete migration and post-step-1 revert cases
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

			// Reset all state and form values
			incompleteMigration = null
			metadataMigrated = false
			jobMigrationStarted = false
			jobMigrationComplete = false
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

<Modal bind:open title="Change Workspace ID">
	<div class="flex flex-col gap-6">
		{#if checkingIncomplete}
			<Alert type="info" title="Checking for incomplete migrations...">
				<p class="text-sm">Please wait while we check for any incomplete workspace migrations.</p>
			</Alert>
		{/if}

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

		{#if metadataMigrated || jobMigrationComplete}
			<div class="bg-surface-secondary border rounded-lg p-4">
				<h3 class="text-sm font-medium mb-3">Migration Status</h3>
				<div class="space-y-2">
					<div class="flex items-center gap-3">
						<div class="w-4 h-4 rounded-full bg-green-500 flex items-center justify-center">
							<span class="text-white text-xs">✓</span>
						</div>
						<span class="text-sm">Step 1: Metadata migrated</span>
					</div>
					{#if jobMigrationComplete}
						<div class="flex items-center gap-3">
							<div class="w-4 h-4 rounded-full bg-green-500 flex items-center justify-center">
								<span class="text-white text-xs">✓</span>
							</div>
							<span class="text-sm">Step 2: Jobs migrated</span>
						</div>
					{:else}
						<div class="flex items-center gap-3">
							<div class="w-4 h-4 rounded-full bg-gray-300 dark:bg-gray-600"></div>
							<span class="text-sm text-gray-500">Step 2: Job migration (ready)</span>
						</div>
					{/if}
				</div>
			</div>
		{/if}

		<div
			class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-lg"
		>
			<div class="flex items-center gap-2 mb-2">
				<div class="w-2 h-2 bg-blue-500 rounded-full"></div>
				<span class="text-blue-700 dark:text-blue-300 text-sm font-medium">Current Workspace</span>
			</div>
			<p class="font-mono text-lg font-bold text-blue-900 dark:text-blue-100"
				>{$workspaceStore ?? ''}</p
			>
		</div>

		<div class="space-y-4">
			<label class="block">
				<span class="text-secondary text-sm font-medium">New Workspace Name</span>
				<input
					type="text"
					bind:value={newName}
					placeholder="Enter new workspace name"
					class="mt-1 w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed"
					disabled={incompleteMigration !== null}
				/>
				{#if incompleteMigration}
					<p class="text-xs text-amber-600 dark:text-amber-400 mt-1">
						Input disabled - migration in progress
					</p>
				{/if}
			</label>
			<label class="block">
				<span class="text-secondary text-sm font-medium">New Workspace ID</span>
				<input
					type="text"
					bind:value={newId}
					placeholder="auto-generated from name"
					class="mt-1 w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed"
					disabled={incompleteMigration !== null}
				/>
				{#if errorId}
					<div class="text-red-500 text-xs mt-1 flex items-center gap-1">
						<span class="w-3 h-3 text-red-500">⚠</span>
						{errorId}
					</div>
				{/if}
				{#if incompleteMigration}
					<p class="text-xs text-amber-600 dark:text-amber-400 mt-1">
						Input disabled - migration in progress
					</p>
				{/if}
			</label>
		</div>

		<Alert type="warning" title="Important Notice">
			<p class="text-sm">
				Once finished, please update your webhook calls and adjust your CLI sync configuration
				accordingly.
			</p>
		</Alert>
	</div>

	<svelte:fragment slot="actions">
		{#if incompleteMigration}
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
					onclick={(e) => {
						e.preventDefault()
						completeMigration()
					}}
					{loading}
				>
					Complete Migration
				</Button>
			</div>
		{:else if !metadataMigrated}
			<div class="flex gap-3">
				<Button
					size="sm"
					disabled={checking || errorId.length > 0 || !newName || !newId || loading}
					{loading}
					onclick={(e) => {
						e.preventDefault()
						migrateMetadata()
					}}
					color="blue"
				>
					Step 1: Migrate Metadata
				</Button>
				<Button size="sm" disabled={true} class="opacity-50" color="blue">
					Step 2: Migrate Jobs
				</Button>
			</div>
		{:else if metadataMigrated && !jobMigrationComplete}
			<div class="flex flex-row gap-2">
				<div class="flex gap-3">
					<Button size="sm" disabled={true} color="blue">✓ Step 1 Complete</Button>
					<Button
						size="sm"
						disabled={migratingJobs || loading}
						onclick={(e) => {
							e.preventDefault()
							startJobMigration()
						}}
						color="blue"
					>
						Step 2: Migrate Jobs
					</Button>
				</div>
				<Button
					size="sm"
					color="red"
					onclick={(e) => {
						e.preventDefault()
						revertMigration()
					}}
					{loading}
				>
					Revert
				</Button>
			</div>
		{:else}
			<div class="flex gap-3 w-full">
				<Button size="sm" disabled={true} color="blue">✓ Step 1 Complete</Button>
				<Button size="sm" disabled={true} color="blue">✓ Step 2 Complete</Button>
			</div>
		{/if}
	</svelte:fragment>
</Modal>

{#if migratingJobs}
	<Modal open={true} title="Step 2: Migrating Jobs">
		<div class="flex flex-col gap-6">
			<div
				class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-6 rounded-lg"
			>
				<div class="flex items-center gap-4 mb-4">
					<Loader2 class="w-12 h-12 animate-spin text-blue-500" />
					<div>
						<h3 class="font-semibold text-blue-900 dark:text-blue-100">Migration in Progress</h3>
						<p class="text-sm text-blue-700 dark:text-blue-300"
							>This process may take several minutes depending on job history size</p
						>
					</div>
				</div>
				<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
					<div class="bg-blue-500 h-2 rounded-full animate-pulse" style="width: 60%"></div>
				</div>
			</div>

			<div class="space-y-4">
				<div class="flex items-center gap-3">
					<div class="w-2 h-2 bg-green-500 rounded-full"></div>
					<span class="text-sm text-secondary">Metadata migration completed</span>
				</div>
				<div class="flex items-center gap-3">
					<div class="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
					<span class="text-sm font-medium">Migrating job history and results...</span>
				</div>
				<div class="flex items-center gap-3">
					<div class="w-2 h-2 bg-gray-300 dark:bg-gray-600 rounded-full"></div>
					<span class="text-sm text-gray-400">Authentication tables transfer (pending)</span>
				</div>
			</div>

			<div class="bg-surface-secondary p-4 rounded-lg border">
				<p class="text-sm font-medium mb-2">Migration Details</p>
				<div class="space-y-1 text-sm text-secondary">
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
		</div>
		<svelte:fragment slot="actions">
			<Button
				onclick={(e) => {
					e.preventDefault()
					cancelJobMigration()
				}}
				color="red"
				size="sm"
				class="w-full"
			>
				Cancel Migration
			</Button>
		</svelte:fragment>
	</Modal>
{/if}

{#if jobMigrationComplete}
	<Modal open={true} title="🎉 Migration Complete">
		<div class="space-y-6">
			<div
				class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 p-6 rounded-lg text-center"
			>
				<div
					class="w-16 h-16 bg-green-500 rounded-full flex items-center justify-center mx-auto mb-4"
				>
					<span class="text-2xl text-white">✓</span>
				</div>
				<h3 class="text-lg font-semibold text-green-900 dark:text-green-100 mb-2">
					Migration Successful!
				</h3>
				<p class="text-sm text-green-700 dark:text-green-300">
					All data has been successfully migrated to your new workspace
				</p>
			</div>

			<div class="bg-surface-secondary p-5 rounded-lg border">
				<h4 class="font-medium mb-3">Migration Summary</h4>
				<div class="space-y-2 text-sm">
					<div class="flex items-center gap-3">
						<div class="w-2 h-2 bg-green-500 rounded-full"></div>
						<span>Workspace metadata migrated</span>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-2 h-2 bg-green-500 rounded-full"></div>
						<span>Job history and results migrated</span>
					</div>
					<div class="flex items-center gap-3">
						<div class="w-2 h-2 bg-green-500 rounded-full"></div>
						<span>Ready for final workspace switch</span>
					</div>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div
					class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-4 rounded-lg"
				>
					<p class="text-xs text-red-600 dark:text-red-400 font-medium mb-1">Old Workspace</p>
					<p class="font-mono text-sm text-red-800 dark:text-red-200">{oldWorkspaceId}</p>
					<p class="text-xs text-red-600 dark:text-red-400 mt-1">Will be deleted</p>
				</div>
				<div
					class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 p-4 rounded-lg"
				>
					<p class="text-xs text-green-600 dark:text-green-400 font-medium mb-1">New Workspace</p>
					<p class="font-mono text-sm text-green-800 dark:text-green-200">{newId}</p>
					<p class="text-xs text-green-600 dark:text-green-400 mt-1">Ready to use</p>
				</div>
			</div>

			<Alert type="info" title="Final Step">
				<p class="text-sm">
					Click the button below to complete the migration process. This will transfer
					authentication tables, delete the old workspace, and redirect you to the new workspace.
				</p>
			</Alert>
		</div>
		<svelte:fragment slot="actions">
			<Button
				size="sm"
				{loading}
				onclick={(e) => {
					e.preventDefault()
					deleteOldWorkspaceAndSwitch()
				}}
				class="w-full"
			>
				<span class="flex items-center justify-center gap-2">
					<span>🚀</span>
					Complete Migration & Switch to New Workspace
				</span>
			</Button>
		</svelte:fragment>
	</Modal>
{/if}
