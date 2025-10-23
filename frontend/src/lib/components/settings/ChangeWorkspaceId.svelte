<script lang="ts">
	import { workspaceStore, superadmin } from '$lib/stores'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService, JobService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { onDestroy } from 'svelte'
	import { hubPaths } from '$lib/hub'

	let { open = $bindable(false) } = $props()

	let newName = $state('')
	let newId = $derived(newName.toLowerCase().replace(/\s/gi, '-'))
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
			open = false

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
					source_workspace_id: oldWorkspaceId,
					target_workspace_id: newId,
					target_workspace_name: newName
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

			await WorkspaceService.deleteWorkspace({
				workspace: oldWorkspaceId
			})

			sendUserToast(`Old workspace ${oldWorkspaceId} deleted`)

			window.location.href = `/workspace_settings?tab=general&workspace=${newId}`
		} catch (err: any) {
			sendUserToast(`Failed to delete workspace: ${err.body || err.message}`, true)
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

<Modal bind:open title="Change Workspace">
	<div class="flex flex-col gap-6">
		<Alert type="info" title="Two-Step Process">
			<div class="text-sm space-y-2">
				<p class="font-semibold">This process involves two steps:</p>
				<ol class="list-decimal list-inside space-y-1 ml-2">
					<li>
						<strong>Migrate Metadata</strong> - Migrates workspace configuration, scripts, flows, apps,
						resources, schedules, and all metadata
					</li>
					<li>
						<strong>Migrate Jobs</strong>
					</li>
				</ol>
				<p class="text-xs text-secondary mt-2">
					After completing step 1, you'll be prompted to proceed with step 2.
				</p>
			</div>
		</Alert>

		<Alert type="warning" title="Important Notice">
			<p class="text-sm">
				Once finished, please update your webhook calls and adjust your CLI sync configuration
				accordingly.
			</p>
		</Alert>

		<div class="bg-surface-secondary p-4 rounded-md border border-gray-200 dark:border-gray-700">
			<p class="text-secondary text-sm mb-2">Current Workspace ID</p>
			<p class="font-bold text-lg">{$workspaceStore ?? ''}</p>
		</div>

		<label class="block">
			<span class="text-secondary text-sm font-medium">New Workspace Name</span>
			<input type="text" bind:value={newName} placeholder="Enter new workspace name" class="mt-1" />
		</label>
		<label class="block">
			<span class="text-secondary text-sm font-medium">New Workspace ID</span>
			<input type="text" bind:value={newId} placeholder="auto-generated from name" class="mt-1" />
			{#if errorId}
				<div class="text-red-500 text-xs mt-1">{errorId}</div>
			{/if}
		</label>
	</div>

	<svelte:fragment slot="actions">
		<Button
			size="sm"
			disabled={checking || errorId.length > 0 || !newName || !newId}
			{loading}
			on:click={() => {
				migrateMetadata()
			}}
		>
			Step 1: Migrate Metadata
		</Button>
	</svelte:fragment>
</Modal>

{#if metadataMigrated}
	<Alert type="success" title="Step 1 Complete: Metadata Migrated" class="mt-4">
		<div class="flex flex-col gap-4">
			<div class="space-y-2">
				<p class="font-semibold">
					✓ Workspace metadata successfully migrated to <strong class="text-primary">{newId}</strong
					>
				</p>
			</div>

			{#if !jobMigrationStarted && !jobMigrationComplete}
				<Alert type="warning" title="Step 2: Migrate Job History">
					<p class="text-sm mb-3">
						Job history has not been migrated yet. Click below to start migrating completed jobs
						from the old workspace.
					</p>
					<Button size="sm" onclick={startJobMigration}>Start Job Migration</Button>
				</Alert>
			{/if}

			{#if migratingJobs}
				<div
					class="bg-surface-secondary p-4 rounded-md border border-gray-200 dark:border-gray-700"
				>
					<div class="space-y-4">
						<div
							class="flex items-center gap-2 text-sm text-secondary p-3 bg-blue-50 dark:bg-blue-900/20 rounded border border-blue-200 dark:border-blue-800"
						>
							<div
								class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
							></div>
							<span class="font-medium">Migrating jobs... This may take a while.</span>
						</div>

						<Button onclick={cancelJobMigration} color="red" size="sm" class="w-full">
							Cancel Migration
						</Button>
					</div>
				</div>
			{/if}

			{#if jobMigrationComplete}
				<Alert type="success" title="Migration Complete">
					<div class="space-y-3">
						<p class="font-semibold"> ✓ All jobs have been successfully migrated! </p>
						<p class="text-sm text-secondary">
							Your workspace migration is complete. You can now delete the old workspace and switch
							to the new one.
						</p>
						<Button size="sm" {loading} onclick={deleteOldWorkspaceAndSwitch}>
							Delete Old Workspace & Switch
						</Button>
					</div>
				</Alert>
			{/if}
		</div>
	</Alert>
{/if}
