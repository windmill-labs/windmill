<script lang="ts">
	import { workspaceStore, userWorkspaces, usersWorkspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { Badge, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import { devBadgeText, devLabelKey, devLabelNoun } from '$lib/utils/devWorkspaceLabel'
	import {
		loadProtectionRules,
		fetchProtectionRulesForWorkspace,
		isRuleUnconditionallyActiveInRulesets
	} from '$lib/workspaceProtectionRules.svelte'
	import { GitFork, ExternalLink } from 'lucide-svelte'
	import { resource } from 'runed'

	let currentWs = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let isDev = $derived(currentWs?.is_dev_workspace ?? false)
	let currentLabel = $derived(devLabelKey(currentWs?.dev_workspace_label))
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
			? {
					id: canonicalDev.id,
					name: canonicalDev.name,
					isMember: true,
					label: canonicalDev.dev_workspace_label
				}
			: devWorkspaceResource.current
				? {
						id: devWorkspaceResource.current.id,
						name: devWorkspaceResource.current.name,
						isMember: false,
						label: devWorkspaceResource.current.dev_workspace_label
					}
				: undefined
	)

	let selectedDevId = $state<string | undefined>(undefined)
	let lockProdDeploy = $state(true)
	let lockProdForking = $state(true)
	// Cosmetic display label chosen when attaching an existing workspace as dev.
	let attachLabel = $state<'dev' | 'staging'>('dev')
	let busy = $state(false)
	let labelBusy = $state(false)

	// If this workspace already blocks direct deploy / forking through an existing protection rule, keep
	// the matching lock toggle on but locked: attaching only manages its own reserved dev-workspace rule,
	// so turning it "off" here couldn't lift a separately-defined block. Fetched only while the attach form
	// is on screen; a failed fetch falls back to the editable default-on toggle (real rules still enforce).
	const rootProtectionRules = resource(
		() => (!parentId && !pairedDev ? $workspaceStore : undefined),
		async (ws, _prev, { signal }) => {
			if (!ws) return undefined
			const rules = await fetchProtectionRulesForWorkspace(ws)
			// The generated client can't take an abort signal, so drop a superseded response here: a late
			// result for a previously selected workspace must not overwrite the current one's rules.
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			return { ws, rules }
		}
	)
	// Only trust a result that belongs to the current workspace (guards the in-flight window and any
	// out-of-order response); undefined means "not known yet" and is treated as locked below.
	let rootRules = $derived.by(() => {
		const current = rootProtectionRules.current
		return current && current.ws === $workspaceStore ? current.rules : undefined
	})
	// Only a rule with no bypass users/groups matches the empty-bypass reserved lock we would create; a
	// bypassable rule stays editable, otherwise forcing the lock on would revoke the bypassed users'
	// direct-deploy / forking access.
	let alreadyBlocksDeploy = $derived(
		isRuleUnconditionallyActiveInRulesets(rootRules ?? [], 'DisableDirectDeployment')
	)
	let alreadyBlocksForking = $derived(
		isRuleUnconditionallyActiveInRulesets(rootRules ?? [], 'DisableWorkspaceForking')
	)
	// Until the fetch resolves for the current workspace its rules are unknown. Treat each lock as
	// engaged during that window so the toggle is locked on and the effective value stays true:
	// otherwise a user could turn a lock off and attach before an existing rule is detected, sending
	// false and omitting the reserved rule — leaving prod unprotected if that rule is later removed.
	let rulesUnknown = $derived(rootProtectionRules.loading || rootRules === undefined)
	let deployLocked = $derived(alreadyBlocksDeploy || rulesUnknown)
	let forkingLocked = $derived(alreadyBlocksForking || rulesUnknown)
	// Sent to the backend: a locked restriction (enforced or not-yet-known) stays on regardless of the
	// toggle's raw state, keeping the request consistent with what the locked toggle shows.
	let effectiveLockProdDeploy = $derived(deployLocked || lockProdDeploy)
	let effectiveLockProdForking = $derived(forkingLocked || lockProdForking)

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
					lock_prod_deploy: effectiveLockProdDeploy,
					lock_prod_forking: effectiveLockProdForking,
					dev_workspace_label: attachLabel
				}
			})
			sendUserToast(`Attached ${selectedDevId} as ${attachLabel} workspace`)
			selectedDevId = undefined
			await refresh()
		} catch (e: any) {
			sendUserToast(`Failed to attach dev workspace: ${e?.body ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function setLabel(label: 'dev' | 'staging') {
		if (!$workspaceStore || label === devLabelKey(currentWs?.dev_workspace_label)) return
		labelBusy = true
		try {
			await WorkspaceService.setDevWorkspaceLabel({
				workspace: $workspaceStore,
				requestBody: { dev_workspace_label: label }
			})
			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		} catch (e: any) {
			sendUserToast(`Failed to update display label: ${e?.body ?? e}`, true)
		} finally {
			labelBusy = false
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
			This is a <b>{devLabelNoun(currentWs?.dev_workspace_label)}</b> paired with root workspace
			<b>{parentId}</b>. Promote changes from the home page banner or the Compare &amp; Deploy page.
		</p>
		<div class="text-2xs text-secondary">
			Cosmetic label: <Badge color="indigo" small>{devBadgeText(currentLabel)}</Badge>
			<button
				type="button"
				disabled={labelBusy}
				class="text-secondary hover:text-primary hover:underline disabled:opacity-50"
				onclick={() => setLabel(currentLabel === 'staging' ? 'dev' : 'staging')}
			>
				Change to {currentLabel === 'staging' ? 'dev' : 'staging'}
			</button>
		</div>
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
			This workspace's {devLabelNoun(pairedDev.label)} is <b>{pairedDev.name}</b> ({pairedDev.id}).
			Edits to this workspace are redirected there.
		</p>
		<div class="flex gap-2">
			{#if pairedDev.isMember}
				<Button
					variant="default"
					startIcon={{ icon: GitFork }}
					onclick={() => switchWorkspace(pairedDev.id)}
				>
					Go to {devLabelNoun(pairedDev.label)}
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
		<div class="text-2xs text-secondary">
			Cosmetic label: <Badge color="indigo" small>{devBadgeText(attachLabel)}</Badge>
			<button
				type="button"
				class="text-secondary hover:text-primary hover:underline"
				onclick={() => (attachLabel = attachLabel === 'staging' ? 'dev' : 'staging')}
			>
				Change to {attachLabel === 'staging' ? 'dev' : 'staging'}
			</button>
		</div>
		{#if deployLocked}
			<div class="flex flex-col gap-0.5">
				<Toggle
					checked
					disabled
					options={{
						right: 'Block direct edits in this workspace (deploy via the dev workspace)'
					}}
				/>
				{#if alreadyBlocksDeploy}
					<span class="text-2xs text-secondary ml-8"
						>Already enforced by an existing protection rule</span
					>
				{/if}
			</div>
		{:else}
			<Toggle
				bind:checked={lockProdDeploy}
				options={{
					right: 'Block direct edits in this workspace (deploy via the dev workspace)'
				}}
			/>
		{/if}
		{#if forkingLocked}
			<div class="flex flex-col gap-0.5">
				<Toggle checked disabled options={{ right: 'Prevent forking this workspace' }} />
				{#if alreadyBlocksForking}
					<span class="text-2xs text-secondary ml-8"
						>Already enforced by an existing protection rule</span
					>
				{/if}
			</div>
		{:else}
			<Toggle
				bind:checked={lockProdForking}
				options={{ right: 'Prevent forking this workspace' }}
			/>
		{/if}
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
