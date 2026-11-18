<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { resource } from 'runed'
	import { Button } from './common'
	import Alert from './common/alert/Alert.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'
	import TextInput from './text_input/TextInput.svelte'

	let baseUrl = $state('')
	let remoteWorkspaceId = $state('')
	let saving = $state(false)

	const targetResource = resource(
		() => $workspaceStore,
		async (workspace, previous, { signal }) => {
			if (workspace !== previous) {
				baseUrl = ''
				remoteWorkspaceId = ''
			}
			const status = workspace
				? await WorkspaceService.getRemoteDeployTarget({ workspace })
				: undefined
			// runed keeps a superseded fetch's result: filling the form with it would let a save
			// point the workspace switched to at the previous one's instance.
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			baseUrl = status?.target?.base_url ?? ''
			remoteWorkspaceId = status?.target?.workspace_id ?? ''
			return { workspace, target: status?.target }
		}
	)
	let loaded = $derived(
		targetResource.current?.workspace === $workspaceStore ? targetResource.current : undefined
	)
	let saved = $derived(loaded?.target)
	let hasChanges = $derived(
		baseUrl !== (saved?.base_url ?? '') || remoteWorkspaceId !== (saved?.workspace_id ?? '')
	)

	async function save(target: { base_url: string; workspace_id: string } | undefined) {
		const workspace = loaded?.workspace
		if (!workspace) return
		saving = true
		try {
			const message = await WorkspaceService.setRemoteDeployTarget({
				workspace,
				requestBody: { target }
			})
			sendUserToast(message)
			await targetResource.refetch()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			saving = false
		}
	}
</script>

<SettingCard
	label="Workspace on another instance"
	description="Deploy the same items to a workspace on a different Windmill instance. Each person deploying connects with their own token for that instance, from the deploy drawer."
	class="mt-6"
>
	{#if !$enterpriseLicense}
		<Alert type="warning" title="Enterprise license required">
			Deploying to another instance from the web UI is only available with an enterprise license
		</Alert>
	{:else}
		<div class="flex flex-col gap-3 mt-2 max-w-md">
			<label class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Instance URL</span>
				<TextInput
					bind:value={baseUrl}
					inputProps={{ placeholder: 'https://windmill.example.com' }}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Workspace on that instance</span>
				<TextInput bind:value={remoteWorkspaceId} inputProps={{ placeholder: 'prod' }} />
			</label>
			<div class="flex flex-row gap-2">
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!loaded || !hasChanges || !baseUrl || !remoteWorkspaceId || saving}
					onclick={() => save({ base_url: baseUrl, workspace_id: remoteWorkspaceId })}
				>
					Save
				</Button>
				{#if saved}
					<Button
						variant="default"
						unifiedSize="sm"
						destructive
						disabled={saving}
						onclick={() => save(undefined)}
					>
						Remove target
					</Button>
				{/if}
			</div>
			{#if saved}
				<p class="text-xs text-secondary">
					Changing the instance URL or workspace drops every token stored for the current target, so
					everyone connects again.
				</p>
			{/if}
		</div>
	{/if}
</SettingCard>
