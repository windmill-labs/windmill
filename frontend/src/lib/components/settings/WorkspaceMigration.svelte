<script lang="ts">
	import { WorkspaceService, type MigrateJobResponse } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import Alert from '../common/alert/Alert.svelte'
	import { onMount } from 'svelte'

	let { sourceWorkspace }: { sourceWorkspace: string } = $props()

	let migrating = $state(false)
	let migrationComplete = $state(false)
	let status = $state<MigrateJobResponse | undefined>(undefined)

	let pollInterval: number | null = null

	const progress = $derived(status ? status.migration_progress : 0)

	async function checkStatus() {
		try {
			status = await WorkspaceService.getMigrationStatus({
				sourceWorkspace
			})

			if (status && status.remaining_jobs === 0 && status.total_jobs > 0) {
				migrationComplete = true
				if (pollInterval) {
					clearInterval(pollInterval)
					pollInterval = null
				}
			}
		} catch (err) {
			console.error('Failed to check migration status:', err)
		}
	}

	async function startMigration() {
		try {
			migrating = true

			await WorkspaceService.migrateWorkspace({
				requestBody: {
					source_workspace: sourceWorkspace,
					target_workspace: $workspaceStore!,
					migration_type: 'jobs',
					disable_workspace: false
				}
			})

			sendUserToast('Job migration completed!')
			await checkStatus()
			migrationComplete = true
		} catch (err) {
			sendUserToast(`Migration failed: ${err}`, true)
		} finally {
			migrating = false
		}
	}

	function startPolling() {
		if (!pollInterval) {
			pollInterval = setInterval(checkStatus, 2000) as any
		}
	}

	function stopPolling() {
		if (pollInterval) {
			clearInterval(pollInterval)
			pollInterval = null
		}
	}

	onMount(() => {
		checkStatus()
		return () => {
			stopPolling()
		}
	})

	$effect(() => {
		if (migrating) {
			startPolling()
		}
	})
</script>

<div class="flex flex-col gap-6">
	<div>
		<h2 class="text-xl font-semibold mb-2">Workspace Job Migration</h2>
		<p class="text-sm text-secondary">
			Migrate job history from <strong class="text-primary">{sourceWorkspace}</strong> to
			<strong class="text-primary">{$workspaceStore}</strong>
		</p>
	</div>

	{#if status}
		<div class="bg-surface-secondary p-6 rounded-md border border-gray-200 dark:border-gray-700">
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<span class="text-sm font-medium">Jobs Remaining:</span>
					<span class="text-lg font-semibold"
						>{status.remaining_jobs.toLocaleString()} / {status.total_jobs.toLocaleString()}</span
					>
				</div>

				<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-4">
					<div
						class="bg-blue-500 h-4 rounded-full transition-all duration-300 flex items-center justify-center"
						style="width: {progress}%"
					>
						{#if progress > 10}
							<span class="text-xs text-white font-medium">{progress.toFixed(1)}%</span>
						{/if}
					</div>
				</div>

				{#if migrating}
					<div class="flex items-center gap-2 text-sm text-secondary animate-pulse">
						<div
							class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"
						></div>
						<span>Migration in progress...</span>
					</div>
				{/if}

				{#if !migrationComplete && !migrating && status.total_jobs > 0}
					<Button on:click={startMigration} size="sm" class="w-full">Start Job Migration</Button>
				{/if}

				{#if migrationComplete}
					<Alert type="success" title="Migration Complete">
						<p class="mb-2">All jobs have been successfully migrated!</p>
						<Button
							size="sm"
							on:click={() => {
								window.location.href = `/workspace_settings?tab=general&workspace=${$workspaceStore}`
							}}
						>
							Back to Workspace Settings
						</Button>
					</Alert>
				{/if}

				{#if status.total_jobs === 0}
					<Alert type="info" title="No Jobs to Migrate">
						<p class="mb-2">The source workspace has no job history to migrate.</p>
						<Button
							size="sm"
							on:click={() => {
								window.location.href = `/workspace_settings?tab=general&workspace=${$workspaceStore}`
							}}
						>
							Back to Workspace Settings
						</Button>
					</Alert>
				{/if}
			</div>
		</div>
	{:else}
		<div class="flex items-center justify-center p-8">
			<div class="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"
			></div>
		</div>
	{/if}
</div>
