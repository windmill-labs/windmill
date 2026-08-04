<script lang="ts">
	import TextInput from '../text_input/TextInput.svelte'
	import { SettingService } from '$lib/gen'
	import { instanceSettingsSaved } from '../instanceSettings'
	import Alert from '../common/alert/Alert.svelte'
	import { resource } from 'runed'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	type StaleRepo = {
		workspace_id: string
		git_repo_resource_path: string
		registered_url?: string
	}

	/**
	 * Changing this setting never moves a webhook GitHub already holds — it only
	 * decides where the next one is registered. It is normally set at instance setup
	 * with no GitHub App connected yet, so this is empty; when it isn't, these are the
	 * workspaces whose git sync settings have to be re-saved to move their hook.
	 */
	// Keyed on the save signal, not on the form value: the list is derived from the
	// *saved* setting, so it has to refresh when a save lands — which is exactly the
	// moment an admin needs to see which workspaces still hold the old receiver.
	const staleRepos = resource(
		() => $instanceSettingsSaved,
		async () => {
			try {
				return (await SettingService.githubAppStaleWebhooks()) as StaleRepo[]
			} catch {
				// Superadmin- and EE-only; a failure here just means nothing to show.
				return [] as StaleRepo[]
			}
		}
	)

	let stale = $derived(staleRepos.current ?? [])
</script>

<div class="flex flex-col gap-1">
	<TextInput
		inputProps={{
			type: 'text',
			placeholder: 'https://windmill-webhooks.company.com',
			disabled
		}}
		bind:value={$values['github_app_webhook_base_url']}
	/>
	{#if stale.length > 0}
		<div class="mt-2">
			<Alert type="warning" title="Existing webhooks still use a previous url" size="xs">
				GitHub keeps delivering to the url a webhook was registered with. Re-save git sync settings
				in {stale.length === 1 ? 'this workspace' : 'these workspaces'} to move
				{stale.length === 1 ? 'it' : 'them'}:
				<ul class="list-disc ml-4 mt-1">
					{#each stale as repo (repo.workspace_id + repo.git_repo_resource_path)}
						<li>
							<span class="font-mono">{repo.workspace_id}</span>
							<span class="text-tertiary"> — {repo.git_repo_resource_path}</span>
						</li>
					{/each}
				</ul>
			</Alert>
		</div>
	{/if}
</div>
