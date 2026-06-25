<script lang="ts">
	import { workspaceStore, userWorkspaces, usersWorkspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import { GitFork, ExternalLink } from 'lucide-svelte'

	let currentWs = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let isDev = $derived(currentWs?.is_dev_workspace ?? false)
	let parentId = $derived(currentWs?.parent_workspace_id ?? undefined)
	let canonicalDev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))

	let selectedDevId = $state<string | undefined>(undefined)
	let lockProdDeploy = $state(true)
	let lockProdForking = $state(true)
	let busy = $state(false)

	// Only standalone root workspaces (not already a fork/dev of something) can be attached.
	let attachCandidates = $derived(
		$userWorkspaces
			.filter((w) => !w.parent_workspace_id && w.id !== $workspaceStore && w.id !== 'admins')
			.map((w) => ({ label: `${w.name} (${w.id})`, value: w.id }))
	)

	async function refresh() {
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
	}

	async function attach() {
		if (!selectedDevId || !$workspaceStore) return
		busy = true
		try {
			await WorkspaceService.attachDevWorkspace({
				workspace: $workspaceStore,
				requestBody: {
					dev_workspace_id: selectedDevId,
					lock_prod_deploy: lockProdDeploy,
					lock_prod_forking: lockProdForking
				}
			})
			sendUserToast(`Attached ${selectedDevId} as dev workspace`)
			selectedDevId = undefined
			await refresh()
		} catch (e: any) {
			sendUserToast(`Failed to attach dev workspace: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function detach(devId: string) {
		if (!$workspaceStore) return
		busy = true
		try {
			await WorkspaceService.detachDevWorkspace({
				workspace: $workspaceStore,
				requestBody: { dev_workspace_id: devId }
			})
			sendUserToast(`Detached dev workspace ${devId}`)
			await refresh()
		} catch (e: any) {
			sendUserToast(`Failed to detach dev workspace: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}
</script>

{#if isDev && parentId}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm">
			This is a <b>dev workspace</b> paired with prod workspace <b>{parentId}</b>. Promote changes
			from the home page banner or the Compare &amp; Deploy page.
		</p>
		<div>
			<Button
				variant="default"
				startIcon={{ icon: ExternalLink }}
				onclick={() => switchWorkspace(parentId)}
			>
				Go to prod workspace
			</Button>
		</div>
	</div>
{:else if canonicalDev}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm">
			This workspace's dev workspace is <b>{canonicalDev.name}</b> ({canonicalDev.id}). Edits to
			this (prod) workspace are redirected there.
		</p>
		<div class="flex gap-2">
			<Button
				variant="default"
				startIcon={{ icon: GitFork }}
				onclick={() => switchWorkspace(canonicalDev.id)}
			>
				Go to dev workspace
			</Button>
			<Button color="red" disabled={busy} onclick={() => detach(canonicalDev.id)}>Detach</Button>
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm text-secondary">
			Pair this workspace (as prod) with a dev workspace: the same code with a different environment
			(resource and variable values). Edits are made in the dev workspace and promoted here.
		</p>
		<div class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Attach an existing workspace as dev</span>
			<Select
				items={attachCandidates}
				bind:value={selectedDevId}
				placeholder="Select a workspace"
				clearable
			/>
		</div>
		<Toggle
			bind:checked={lockProdDeploy}
			options={{
				right: 'Block direct edits in this (prod) workspace (deploy via the dev workspace)'
			}}
		/>
		<Toggle
			bind:checked={lockProdForking}
			options={{ right: 'Prevent forking this (prod) workspace' }}
		/>
		<div class="flex gap-2">
			<Button variant="accent" disabled={busy || !selectedDevId} onclick={attach}>
				Attach dev workspace
			</Button>
			<Button
				variant="default"
				startIcon={{ icon: GitFork }}
				onclick={() => goto(`${base}/user/fork_workspace`)}
			>
				Create a new dev workspace
			</Button>
		</div>
	</div>
{/if}
