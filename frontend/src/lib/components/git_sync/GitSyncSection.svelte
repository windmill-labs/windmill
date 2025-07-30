<script lang="ts">
	import { Plus, ExternalLink } from 'lucide-svelte'
	import { Button, Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { setGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncRepositoryList from './GitSyncRepositoryList.svelte'
	import GitSyncModalManager from './GitSyncModalManager.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'


	// Create context reactively based on workspaceStore
	const gitSyncContext = $derived($workspaceStore ? setGitSyncContext($workspaceStore) : null)

	// Load settings when workspace context changes
	$effect(() => {
		if (gitSyncContext) {
			untrack(async () => {
				try {
					await gitSyncContext.loadSettings()
				} catch (error) {
					console.error('Failed to load git sync settings:', error)
					sendUserToast('Failed to load git sync settings', true)
				}
			})
		}
	})

</script>

{#if !gitSyncContext}
	<div class="flex items-center justify-center p-8">
		<div class="text-sm text-secondary">Loading workspace...</div>
	</div>
{:else if gitSyncContext.loading}
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
	{#if $enterpriseLicense && gitSyncContext.repositories != undefined}
		<div class="flex mt-5 mb-5 gap-8">
			<Button
				color="dark"
				target="_blank"
				endIcon={{ icon: ExternalLink }}
				href={`/runs?job_kinds=deploymentcallbacks&workspace=${$workspaceStore}`}
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
