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
	import { loadProtectionRules } from '$lib/workspaceProtectionRules.svelte'
	import { GitFork, ExternalLink } from 'lucide-svelte'
	import { resource } from 'runed'

	let currentWs = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let isDev = $derived(currentWs?.is_dev_workspace ?? false)
	let parentId = $derived(currentWs?.parent_workspace_id ?? undefined)
	let canonicalDev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))

	// A prod admin who isn't a member of the dev can't see it in their workspace list, so ask the
	// server (only when the client list doesn't already have it) — otherwise the tab would show the
	// attach form instead of the existing pairing and detach control.
	const devWorkspaceResource = resource(
		() => (!isDev && !parentId && !canonicalDev ? $workspaceStore : undefined),
		async (ws) => (ws ? await WorkspaceService.getDevWorkspace({ workspace: ws }) : undefined)
	)
	// The paired dev to display: the client entry when we're a member (so "Go to dev workspace" works),
	// else the server result (pairing + detach still available to a prod admin).
	let pairedDev = $derived(
		canonicalDev
			? { id: canonicalDev.id, name: canonicalDev.name, isMember: true }
			: devWorkspaceResource.current
				? {
						id: devWorkspaceResource.current.id,
						name: devWorkspaceResource.current.name,
						isMember: false
					}
				: undefined
	)

	let selectedDevId = $state<string | undefined>(undefined)
	let lockProdDeploy = $state(true)
	let lockProdForking = $state(true)
	let busy = $state(false)

	// A standalone root workspace, or an existing fork of this prod (same family), can be attached.
	// A fork parented to a different workspace can't (the backend rejects a parent that isn't this
	// prod), so it's excluded here.
	let attachCandidates = $derived(
		$userWorkspaces
			.filter(
				(w) =>
					w.id !== $workspaceStore &&
					w.id !== 'admins' &&
					(!w.parent_workspace_id || w.parent_workspace_id === $workspaceStore)
			)
			.map((w) => ({
				label:
					w.parent_workspace_id === $workspaceStore
						? `${w.name} (${w.id}), fork of this workspace`
						: `${w.name} (${w.id})`,
				value: w.id
			}))
	)

	async function refresh() {
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		// Attach/detach changes this (root) workspace's protection rules; reload them so the
		// direct-deploy / forking lock UI reflects the change without a workspace switch or reload.
		if ($workspaceStore) {
			await loadProtectionRules($workspaceStore)
		}
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
			This is a <b>dev workspace</b> paired with root workspace <b>{parentId}</b>. Promote changes
			from the home page banner or the Compare &amp; Deploy page.
		</p>
		<div>
			<Button
				variant="default"
				startIcon={{ icon: ExternalLink }}
				onclick={() => switchWorkspace(parentId)}
			>
				Go to root workspace
			</Button>
		</div>
	</div>
{:else if pairedDev}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm">
			This workspace's dev workspace is <b>{pairedDev.name}</b> ({pairedDev.id}). Edits to this
			workspace are redirected there.
		</p>
		<div class="flex gap-2">
			{#if pairedDev.isMember}
				<Button
					variant="default"
					startIcon={{ icon: GitFork }}
					onclick={() => switchWorkspace(pairedDev.id)}
				>
					Go to dev workspace
				</Button>
			{/if}
			<Button color="red" disabled={busy} onclick={() => detach(pairedDev.id)}>Detach</Button>
		</div>
	</div>
{:else if parentId}
	<p class="text-sm text-secondary max-w-2xl">
		Dev workspace pairing is only available for root workspaces. This workspace is a fork of
		<b>{parentId}</b>.
	</p>
{:else}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm text-secondary">
			Pair this workspace with a dev workspace: the same code with a different environment (resource
			and variable values). Edits are made in the dev workspace and promoted here.
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
				right: 'Block direct edits in this workspace (deploy via the dev workspace)'
			}}
		/>
		<Toggle bind:checked={lockProdForking} options={{ right: 'Prevent forking this workspace' }} />
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
