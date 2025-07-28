<script lang="ts">
	import { Save, Plus, ExternalLink } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { setGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncRepositoryList from './GitSyncRepositoryList.svelte'
	import GitSyncModalManager from './GitSyncModalManager.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'

	let { workspace } = $props<{ workspace: string }>()

	// Create and set context
	const gitSyncContext = setGitSyncContext(workspace)

	// Load settings on mount
	onMount(async () => {
		try {
			await gitSyncContext.loadSettings()
		} catch (error) {
			console.error('Failed to load git sync settings:', error)
			sendUserToast('Failed to load git sync settings', true)
		}
	})

	async function handleSaveAll() {
		try {
			await gitSyncContext.saveAllRepositories()
			sendUserToast('Git sync settings saved successfully')
		} catch (error: any) {
			console.error('Failed to save git sync settings:', error)
			sendUserToast('Failed to save git sync settings: ' + error.message, true)
		}
	}
</script>

{#if gitSyncContext.loading}
	<div class="flex items-center justify-center p-8">
		<div class="text-sm text-secondary">Loading git sync settings...</div>
	</div>
{:else}
	<div class="flex flex-col gap-4 my-8">
		<div class="flex flex-col gap-1">
			<div class="text-primary text-lg font-semibold">Git Sync</div>
			<Description link="https://www.windmill.dev/docs/advanced/git_sync">
				Connect the Windmill workspace to a Git repository to automatically commit and push
				scripts, flows, and apps to the repository on each deploy.
			</Description>
		</div>
		<Alert type="info" title="Only new updates trigger git sync">
			Only new changes matching the filters will trigger a git sync. You still need to initialize
			the repo to the desired state first.
		</Alert>
	</div>
	{#if !$enterpriseLicense}
		<div class="mb-2"></div>

		<Alert type="warning" title="Syncing workspace to Git is an EE feature">
			Automatically saving scripts to a Git repository on each deploy is a Windmill EE feature.
		</Alert>
		<div class="mb-2"></div>
	{/if}
	{#if gitSyncContext.repositories != undefined}
		<div class="flex mt-5 mb-5 gap-8">
			<Button
				startIcon={{ icon: Save }}
				disabled={!$enterpriseLicense ||
					!gitSyncContext.hasAnyChanges ||
					!gitSyncContext.allRepositoriesValid ||
					gitSyncContext.hasUnsavedConnections}
				onclick={handleSaveAll}
			>
				Save all git sync settings {!$enterpriseLicense ? '(ee only)' : ''}
			</Button>

			<Button
				color="dark"
				target="_blank"
				endIcon={{ icon: ExternalLink }}
				href={`/runs?job_kinds=deploymentcallbacks&workspace=${workspace}`}
			>
				See sync jobs
			</Button>
		</div>
		<div class="pt-2"></div>

		<!-- Repository list -->
		<GitSyncRepositoryList />

		<!-- Add repository button -->
		<div class="flex mt-5 mb-5">
			<Button
				startIcon={{ icon: Plus }}
				color="dark"
				variant="border"
				onclick={() => gitSyncContext.addRepository()}
			>
				Add connection
			</Button>
		</div>

		<!-- Modals -->
		<GitSyncModalManager />
	{/if}
{/if}
