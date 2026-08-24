<script lang="ts">
	import { workspaceStore, userWorkspaces, usersWorkspaceStore, superadmin } from '$lib/stores'
	import { WorkspaceService, type ProtectionRuleset } from '$lib/gen'
	import { Badge, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import {
		findCanonicalDevWorkspace,
		findWorkspaceAncestors,
		findWorkspaceDescendants,
		devWorkspacesInChainAbove
	} from '$lib/utils/workspaceHierarchy'
	import { getUserExt } from '$lib/user'
	import {
		DEV_WORKSPACE_LABELS,
		devBadgeText,
		devLabelKey,
		devLabelNoun,
		type DevWorkspaceLabelKey
	} from '$lib/utils/devWorkspaceLabel'
	import DevWorkspaceLabelPicker from '$lib/components/workspaceSettings/DevWorkspaceLabelPicker.svelte'
	import {
		loadProtectionRules,
		isRuleActiveInRulesets,
		isRuleUnconditionallyActiveInRulesets,
		DEV_WORKSPACE_LOCK_RULE_NAME
	} from '$lib/workspaceProtectionRules.svelte'
	import { GitFork, ExternalLink, Check, Minus, Pen } from 'lucide-svelte'
	import { resource } from 'runed'
	import type { Snippet } from 'svelte'

	let {
		// What this workspace promotes into its parent (deploy target + item filters). Rendered with
		// the pairing statement it restates, ahead of the parent's protections and of the nested
		// pairing below — hence handed over rather than placed by the page, which cannot see where
		// that boundary falls.
		deployTarget
	}: { deployTarget?: Snippet } = $props()

	let currentWs = $derived($userWorkspaces.find((w) => w.id === $workspaceStore))
	let isDev = $derived(currentWs?.is_dev_workspace ?? false)
	let currentLabel = $derived(devLabelKey(currentWs?.dev_workspace_label))
	let parentId = $derived(currentWs?.parent_workspace_id ?? undefined)
	let parentWs = $derived(parentId ? $userWorkspaces.find((w) => w.id === parentId) : undefined)
	// A throwaway fork sits on neither side of a pairing: it is not its parent's dev workspace, and a
	// dev of its own would be discarded along with it.
	let isThrowawayFork = $derived(!!parentId && !isDev)
	let canonicalDev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))

	// A prod admin who isn't a member of the dev can't see it in their workspace list, so ask the
	// server (only when the client list doesn't already have it) — otherwise the tab would show the
	// attach form instead of the existing pairing and detach control.
	const devWorkspaceResource = resource(
		() => (!isThrowawayFork && !canonicalDev ? $workspaceStore : undefined),
		async (ws) => (ws ? await WorkspaceService.getDevWorkspace({ workspace: ws }) : undefined)
	)
	// The paired dev to display: the client entry when we're a member, else the server result (pairing
	// + detach still available to a prod admin). `isMember` decides whether switching into it would
	// land somewhere the caller can use — a superadmin can enter any workspace, member or not.
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
	// The label names the deploy branch, and dev workspaces in a chain share their git-sync
	// repositories: two carrying the same label deploy to the same branch. So the picker steers away
	// from a label the dev workspaces above already hold. Computed without the selected candidate, so
	// picking one can never take the form away mid-selection.
	let chainTakenLabels = $derived(
		new Set(
			devWorkspacesInChainAbove($workspaceStore, $userWorkspaces).map((w) =>
				devLabelKey(w.dev_workspace_label)
			)
		)
	)
	let availableAttachLabels = $derived(DEV_WORKSPACE_LABELS.filter((l) => !chainTakenLabels.has(l)))
	let attachLabel = $state<DevWorkspaceLabelKey>('dev')
	// A candidate keeps its own dev workspaces through the attach, labels included: the first one
	// whose label is already spoken for further up the resulting chain blocks the pairing, whatever
	// label the candidate itself is given.
	let candidateClash = $derived.by(() => {
		if (!selectedDevId) return undefined
		const taken = new Set<string>([...chainTakenLabels, attachLabel])
		for (const w of findWorkspaceDescendants(selectedDevId, $userWorkspaces)) {
			if (!w.is_dev_workspace) continue
			const label = devLabelKey(w.dev_workspace_label)
			if (taken.has(label)) return w.id
			taken.add(label)
		}
		return undefined
	})
	let busy = $state(false)

	// Protection-rule state for one workspace. Instantiated twice because a dev workspace shows two
	// panels at once: the locks its own prod carries, and the locks it would carry as the prod of a
	// nested dev.
	function useProtectionRules(getWs: () => string | undefined) {
		const rules = resource(getWs, async (ws, _prev, { signal }) => {
			if (!ws) return undefined
			// `fetchProtectionRulesForWorkspace` fails open with an empty list, which the toggles below
			// want but the status panel must not read as "nothing is enforced" — so keep the failure.
			let rules: ProtectionRuleset[] | undefined
			try {
				rules = await WorkspaceService.listProtectionRules({ workspace: ws })
			} catch (e) {
				console.error(`Failed to fetch protection rules for workspace ${ws}:`, e)
			}
			// The generated client can't take an abort signal, so drop a superseded response here: a
			// late result for a previously selected workspace must not overwrite the current one's.
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			return { ws, rules: rules ?? [], failed: rules === undefined }
		})
		// Only trust a result that belongs to the workspace we're currently reading rules for (guards
		// the in-flight window and any out-of-order response); undefined means "not known yet" and is
		// treated as locked below.
		const result = $derived.by(() => {
			const current = rules.current
			return current && current.ws === getWs() ? current : undefined
		})
		const list = $derived(result?.rules)
		// Until the fetch resolves for the current workspace its rules are unknown. Treat each lock as
		// engaged during that window so the toggle is locked on and the effective value stays true:
		// otherwise a user could turn a lock off and attach before an existing rule is detected,
		// sending false and omitting the reserved rule — leaving prod unprotected if that rule is
		// later removed.
		const unknown = $derived(rules.loading || list === undefined)
		// Only a rule with no bypass users/groups matches the empty-bypass reserved lock we would
		// create; a bypassable rule stays editable, otherwise forcing the lock on would revoke the
		// bypassed users' direct-deploy / forking access.
		const alreadyBlocksDeploy = $derived(
			isRuleUnconditionallyActiveInRulesets(list ?? [], 'DisableDirectDeployment')
		)
		const alreadyBlocksForking = $derived(
			isRuleUnconditionallyActiveInRulesets(list ?? [], 'DisableWorkspaceForking')
		)
		// What the paired view reports. Unlike the toggles above, this asks whether the rule is
		// enforced at all: a ruleset with bypass users still blocks everyone outside that list, so it
		// is in force here even though the attach form leaves its toggle editable.
		const enforcesDeployBlock = $derived(
			isRuleActiveInRulesets(list ?? [], 'DisableDirectDeployment')
		)
		const enforcesForkingBlock = $derived(
			isRuleActiveInRulesets(list ?? [], 'DisableWorkspaceForking')
		)
		// The named rulesets actually carrying either lock. Reporting only "blocked / allowed" left no
		// way to reach the rule that decides it, which is the one thing a reader here wants to change.
		const enforcingRulesets = $derived(
			(list ?? []).filter(
				(r) =>
					r.rules.includes('DisableDirectDeployment') || r.rules.includes('DisableWorkspaceForking')
			)
		)
		// A failed read is not "nothing is enforced": report it as unknown rather than claiming allowed.
		const readFailed = $derived(result?.failed ?? false)
		return {
			get deployLocked() {
				return alreadyBlocksDeploy || unknown
			},
			get forkingLocked() {
				return alreadyBlocksForking || unknown
			},
			get alreadyBlocksDeploy() {
				return alreadyBlocksDeploy
			},
			get alreadyBlocksForking() {
				return alreadyBlocksForking
			},
			get enforcesDeployBlock() {
				return enforcesDeployBlock
			},
			get enforcesForkingBlock() {
				return enforcesForkingBlock
			},
			get enforcingRulesets() {
				return enforcingRulesets
			},
			get readFailed() {
				return readFailed
			},
			get enforcementUnknown() {
				return unknown || readFailed
			},
			refetch: () => rules.refetch()
		}
	}

	// The locks a pairing applies always sit on the prod side, so each panel reads its own workspace:
	// the parent's rules describe what this dev workspace is promoted into, this workspace's own
	// describe what its (possibly nested) dev workspace is promoted into.
	const parentRules = useProtectionRules(() => (isDev ? parentId : undefined))
	const ownRules = useProtectionRules(() => (isThrowawayFork ? undefined : $workspaceStore))

	// Sent to the backend: a locked restriction (enforced or not-yet-known) stays on regardless of the
	// toggle's raw state, keeping the request consistent with what the locked toggle shows.
	let effectiveLockProdDeploy = $derived(ownRules.deployLocked || lockProdDeploy)
	let effectiveLockProdForking = $derived(ownRules.forkingLocked || lockProdForking)

	// Editing the parent's rules from the dev side needs admin IN THE PARENT, which membership does
	// not imply and this workspace's own admin rights say nothing about: the rulesets tab is
	// admin-only, so a link offered to anyone else lands them on a tab they cannot open. Asked of the
	// parent directly, as `is_admin` is per-workspace. A superadmin is admin everywhere and has no
	// `usr` row to find. Tagged with its workspace and guarded against a superseded response, like the
	// rules resource above: runed keeps the previous `current` while a new source loads, so switching
	// between dev workspaces would otherwise offer Edit based on the previous parent's role.
	const parentUser = resource(
		() => (isDev && parentId ? parentId : undefined),
		async (ws, _prev, { signal }) => {
			if (!ws) return undefined
			const user = await getUserExt(ws)
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			return { ws, isAdmin: user?.is_admin === true }
		}
	)
	let canEditParentRules = $derived(
		$superadmin || (parentUser.current?.ws === parentId && parentUser.current?.isAdmin === true)
	)

	// With a name, deep-links into that rule's drawer; without one, the rulesets list. Built from
	// scratch rather than from the current query so no stale `?workspace=<dev>` survives a switch.
	function rulesetsHref(name?: string): string {
		const rule = name ? `&rule=${encodeURIComponent(name)}` : ''
		return `${base}/workspace_settings?tab=rulesets${rule}`
	}

	function openRulesets(name?: string) {
		goto(rulesetsHref(name))
	}

	function openRulesetsInParent(name?: string) {
		if (!parentId) return
		switchWorkspace(parentId)
		goto(rulesetsHref(name))
	}

	// Reparenting an ancestor below this workspace would close a parent<->child cycle, which the
	// backend rejects: with a dev workspace allowed as prod, the family root is such an ancestor and
	// would otherwise show up here as a standalone candidate.
	let ancestorIds = $derived(
		new Set(findWorkspaceAncestors($workspaceStore, $userWorkspaces).map((w) => w.id))
	)
	// A standalone root workspace, or an existing fork of this workspace (same family), can be
	// attached. A fork parented to a different workspace can't (the backend rejects a parent that
	// isn't this one), so it's excluded here.
	let attachCandidates = $derived(
		$userWorkspaces
			.filter(
				(w) =>
					w.id !== $workspaceStore &&
					w.id !== 'admins' &&
					!ancestorIds.has(w.id) &&
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
		// Both lookups have to be refetched explicitly: neither resource's source value changes when
		// this workspace gains or loses its dev workspace, so without this the caller who is not a
		// member of the dev workspace (the one that reads the pairing from the server rather than
		// from the workspace list) keeps seeing the pre-attach/detach state until the tab remounts.
		devWorkspaceResource.refetch()
		// Attach/detach changes this workspace's protection rules; reload them so the direct-deploy /
		// forking lock UI reflects the change without a workspace switch or reload.
		// Refetching duplicates the request `loadProtectionRules` just made, which is the price of it
		// being the only way to supersede whatever this resource already has in flight: `mutate` just
		// assigns, so an earlier fetch lands afterwards and puts the pre-attach rules back on screen.
		// No guard makes seeding safe either — runed shares one `loading` flag, which a superseded
		// request clears while its replacement is still running. One extra request on an admin-only
		// action is cheaper than a panel that misreports what is enforced.
		if ($workspaceStore) {
			await loadProtectionRules($workspaceStore)
			ownRules.refetch()
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

<!-- The locks are protection rules, so being paired does not imply them: a pairing that came from the
     deploy_to migration rather than from an attach carries neither. `onOpen` navigates to the rules
     of the workspace this panel describes, which is not the active one on the dev side; it is
     undefined when the reader is not an admin there. The labels name that workspace in that case,
     so the button does not switch workspaces without saying so. -->
{#snippet protectionsPanel(opts: {
	title: string
	rules: ReturnType<typeof useProtectionRules>
	onOpen?: (name?: string) => void
	editLabel: string
	manageLabel: string
})}
	{@const onOpen = opts.onOpen}
	{@const rules = opts.rules}
	<div class="flex flex-col gap-1 rounded-md border bg-surface-secondary p-3">
		<span class="text-xs font-semibold text-emphasis">{opts.title}</span>
		{#if rules.enforcementUnknown}
			<span class="text-2xs text-secondary">
				{rules.readFailed ? 'Could not read the protection rules' : 'Checking protection rules…'}
			</span>
		{:else}
			<span class="text-2xs text-secondary flex items-center gap-1.5">
				{#if rules.enforcesDeployBlock}<Check size={12} class="text-green-600" />{:else}<Minus
						size={12}
					/>{/if}
				Direct edits {rules.enforcesDeployBlock ? 'are blocked' : 'are allowed'}
			</span>
			<span class="text-2xs text-secondary flex items-center gap-1.5">
				{#if rules.enforcesForkingBlock}<Check size={12} class="text-green-600" />{:else}<Minus
						size={12}
					/>{/if}
				Forking {rules.enforcesForkingBlock ? 'is blocked' : 'is allowed'}
			</span>
			{#if rules.enforcesDeployBlock || rules.enforcesForkingBlock}
				<!-- Only admins reach this tab, and `check_user_against_rule` lets an admin through
				     every rule, so without this the reader would try what the panel calls blocked. -->
				<span class="text-2xs text-secondary">Workspace admins always bypass these rules.</span>
			{/if}
			{#if rules.enforcingRulesets.length > 0}
				<div class="flex flex-col gap-1 mt-2 pt-2 border-t">
					<span class="text-2xs text-secondary">Enforced by</span>
					{#each rules.enforcingRulesets as ruleset (ruleset.name)}
						<div class="flex items-center justify-between gap-2">
							<div class="flex flex-col min-w-0">
								<span class="text-2xs font-mono text-emphasis truncate">{ruleset.name}</span>
								{#if ruleset.name === DEV_WORKSPACE_LOCK_RULE_NAME}
									<span class="text-2xs text-secondary">Applied by this pairing</span>
								{/if}
							</div>
							{#if onOpen}
								<Button
									variant="subtle"
									unifiedSize="2xs"
									startIcon={{ icon: Pen }}
									onclick={() => onOpen(ruleset.name)}
								>
									{opts.editLabel}
								</Button>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{/if}
		{#if onOpen && rules.enforcingRulesets.length === 0}
			<div class="self-start mt-1">
				<Button variant="subtle" unifiedSize="2xs" onclick={() => onOpen()}>
					{opts.manageLabel}
				</Button>
			</div>
		{/if}
	</div>
{/snippet}

<!-- This workspace as the prod side of a pairing: the dev it already has, or the form to give it one.
     Rendered for a root workspace and, since a dev workspace can host its own dev, for a dev
     workspace too. -->
{#snippet ownDevSection()}
	{#if pairedDev}
		<div class="flex flex-col gap-3 max-w-2xl">
			<p class="text-sm">
				This workspace's {devLabelNoun(pairedDev.label)} is <b>{pairedDev.name}</b>
				({pairedDev.id}). Edits to this workspace are redirected there.
			</p>
			{@render protectionsPanel({
				title: 'Protections in force on this workspace',
				rules: ownRules,
				onOpen: openRulesets,
				editLabel: 'Edit',
				manageLabel: 'Manage in Rulesets'
			})}
			<div class="flex gap-2">
				{#if pairedDev.isMember || $superadmin}
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
	{:else if availableAttachLabels.length === 0}
		<p class="text-sm text-secondary max-w-2xl">
			Every environment label is already taken by a dev workspace in this chain, and two carrying
			the same label would deploy to the same branch. Promote through the existing chain instead.
		</p>
	{:else}
		<div class="flex flex-col gap-3 max-w-2xl">
			<p class="text-sm text-secondary">
				Pair this workspace with a dev workspace: the same code with a different environment
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
			<DevWorkspaceLabelPicker bind:value={attachLabel} takenLabels={chainTakenLabels} />
			<!-- Left under the label row rather than replacing it: flipping the label is often the fix. -->
			{#if candidateClash}
				<p class="text-2xs text-secondary">
					<b>{candidateClash}</b> comes with the selected workspace and already deploys to a branch this
					chain would then use twice. Pick a different label, or another workspace.
				</p>
			{/if}
			<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
				<div class="flex flex-col gap-0.5">
					<span class="text-xs font-semibold text-emphasis">Protect this workspace on attach</span>
					<span class="text-2xs text-secondary">
						Nothing is enforced until you attach: these add protection rules to this workspace so
						changes are made in the dev workspace and promoted here.
					</span>
				</div>
				{#if ownRules.deployLocked}
					<div class="flex flex-col gap-0.5">
						<Toggle
							checked
							disabled
							options={{
								right: 'Block direct edits in this workspace (deploy via the dev workspace)'
							}}
						/>
						{#if ownRules.alreadyBlocksDeploy}
							<span class="text-2xs text-secondary ml-11"
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
				{#if ownRules.forkingLocked}
					<div class="flex flex-col gap-0.5">
						<Toggle checked disabled options={{ right: 'Prevent forking this workspace' }} />
						{#if ownRules.alreadyBlocksForking}
							<span class="text-2xs text-secondary ml-11"
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
			</div>
			<div class="flex gap-2">
				<Button
					variant="accent"
					disabled={busy || !selectedDevId || !!candidateClash}
					onclick={attach}
				>
					Attach dev workspace
				</Button>
				<Button
					variant="default"
					startIcon={{ icon: GitFork }}
					onclick={() => goto(`${base}/user/fork_workspace?dev=true`)}
				>
					Create a new dev workspace
				</Button>
			</div>
		</div>
	{/if}
{/snippet}

{#if isThrowawayFork}
	<p class="text-sm text-secondary max-w-2xl">
		Dev workspace pairing is not available on a throwaway fork. This workspace is a fork of
		<b>{parentId}</b>.
	</p>
	{@render deployTarget?.()}
{:else if isDev && parentId}
	<div class="flex flex-col gap-3 max-w-2xl">
		<p class="text-sm">
			This is a <b>{devLabelNoun(currentWs?.dev_workspace_label)}</b> paired with
			{parentWs?.is_dev_workspace ? devLabelNoun(parentWs.dev_workspace_label) : 'root workspace'}
			<b>{parentId}</b>. Promote changes from the home page banner or the Compare &amp; Deploy page.
		</p>
		<div class="text-2xs text-secondary">
			Environment: <Badge color="indigo" small>{devBadgeText(currentLabel)}</Badge>
			<span class="ml-1">
				Set when the workspace is created or attached. Git sync deploys to the
				<span class="font-mono">{currentLabel}</span> branch.
			</span>
		</div>
	</div>
	{@render deployTarget?.()}
	<div class="flex flex-col gap-3 max-w-2xl mt-6">
		<!-- A reader who is not a member of the parent gets a 403 listing its rules, which is expected
			     here rather than an anomaly worth a permanent error box, so drop the panel instead. -->
		{#if !parentRules.readFailed}
			{@render protectionsPanel({
				title: `Protections in force on ${parentId}`,
				rules: parentRules,
				onOpen: canEditParentRules ? openRulesetsInParent : undefined,
				editLabel: `Edit in ${parentId}`,
				manageLabel: `Manage in ${parentId}`
			})}
		{/if}
		<div>
			<Button
				variant="default"
				startIcon={{ icon: ExternalLink }}
				onclick={() => switchWorkspace(parentId)}
			>
				Go to {parentWs?.is_dev_workspace
					? devLabelNoun(parentWs.dev_workspace_label)
					: 'root workspace'}
			</Button>
		</div>
	</div>
	<div class="flex flex-col gap-3 max-w-2xl mt-8 pt-6 border-t">
		<div class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">This workspace's own dev workspace</span>
			<p class="text-2xs text-secondary">
				A dev workspace can itself be paired with one, giving a longer promotion chain (for example
				dev into staging into prod). It is not the recommended shape — each extra level is another
				promotion to run — but nothing prevents it.
			</p>
		</div>
		{@render ownDevSection()}
	</div>
{:else}
	{@render ownDevSection()}
{/if}
