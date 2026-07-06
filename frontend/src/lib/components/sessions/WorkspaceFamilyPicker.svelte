<script lang="ts">
	import type { Snippet } from 'svelte'
	import {
		enterpriseLicense,
		isPremiumStore,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import {
		findWorkspaceDescendants,
		findCanonicalDevWorkspace,
		findWorkspaceRoot,
		buildWorkspaceHierarchy
	} from '$lib/utils/workspaceHierarchy'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { getUserExt } from '$lib/user'
	import { WorkspaceService } from '$lib/gen'
	import {
		fetchProtectionRulesForWorkspace,
		isRuleActiveInRulesets,
		canUserBypassRuleKindInRulesets
	} from '$lib/workspaceProtectionRules.svelte'
	import { resource } from 'runed'
	import { isCloudHosted } from '$lib/cloud'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import InputError from '$lib/components/InputError.svelte'
	import PrefixedInput from '$lib/components/PrefixedInput.svelte'
	import { Badge, Button, CopyButton } from '$lib/components/common'
	import { devBadgeText, devLabelNoun } from '$lib/utils/devWorkspaceLabel'
	import Select from '../select/Select.svelte'
	import { Building, Check, GitFork, Plus, Settings } from 'lucide-svelte'

	type PendingFork = { parent_workspace_id: string; id: string; name: string }
	type ForkRequest = { parent_workspace_id: string; id: string; name: string }

	let {
		// Workspace currently associated with the consumer (for display only —
		// drives the family-root resolution and the "selected" highlight).
		// Defaults to the active workspace store when not set.
		selectedId,
		// A staged-but-not-yet-created fork (e.g. SessionWorkspaceBar's
		// pre-send draft). Highlighted as the active row when set.
		pendingFork,
		onPick,
		onCreateFork,
		// Alternative to onCreateFork's inline staging: when set, the
		// "Create new fork…" row becomes a one-shot button that delegates to
		// this callback (e.g. opening the global fork modal) instead of
		// revealing the inline name input. Used where there's no pending draft
		// to stage (the sidebar scope picker).
		onRequestCreateFork,
		allowCreateFork = true,
		// Optional caption rendered under the new-fork input. Lets the
		// consumer differentiate "staged for first send" vs. "will be
		// created immediately" semantics.
		createForkCaption = '',
		// Forwarded to the dropdown trigger wrapper — e.g. `min-w-0` so the
		// trigger can shrink and its label truncate inside a narrow container.
		class: triggerClass = undefined,
		// Optional settings link rendered at the very bottom of the menu (e.g.
		// "<workspace> settings"). The consumer decides visibility (admin gate)
		// and target href.
		settingsHref,
		settingsLabel,
		trigger
	}: {
		selectedId?: string
		pendingFork?: PendingFork
		onPick: (workspaceId: string) => void | Promise<void>
		onCreateFork?: (fork: ForkRequest) => void | Promise<void>
		onRequestCreateFork?: () => void
		allowCreateFork?: boolean
		createForkCaption?: string
		class?: string
		settingsHref?: string
		settingsLabel?: string
		trigger: Snippet<[{ open: boolean }]>
	} = $props()

	const WM_FORK_PREFIX = 'wm-fork-'

	const effectiveId = $derived(selectedId ?? $workspaceStore ?? undefined)
	const root = $derived(findWorkspaceRoot(effectiveId, $userWorkspaces))
	const forks = $derived(root ? findWorkspaceDescendants(root.id, $userWorkspaces) : [])

	// The family's canonical dev workspace, if any — still used for gating (a forking-locked root can be
	// forked via its dev) and as a selectable base with a "dev" badge.
	const devOfRoot = $derived(root ? findCanonicalDevWorkspace(root.id, $userWorkspaces) : undefined)
	const createForkLabel = 'Create new fork…'
	// Candidate bases ("targets") for a new fork: the root plus every fork/dev in the family, so a fork
	// can itself be the base — i.e. a fork of a fork. Root first, matching the list order below.
	const baseOptions = $derived(root ? [root, ...forks] : [])
	// Options for the base-branch <Select>. The nesting (fork of a fork) is rendered via the Select's
	// per-item `startSnippet` as a depth-based spacer, so the label text itself stays clean.
	const baseItems = $derived(
		baseOptions.map((w) => ({
			value: w.id,
			label: w.name,
			subtitle: w.id === root?.id ? 'root' : w.is_dev_workspace ? 'dev workspace' : undefined
		}))
	)

	// Depth of each workspace within its family (root = 0, its forks = 1, forks of forks = 2, …), from
	// the shared hierarchy builder — the same one the sidebar workspace menu uses — so the list indents
	// forks of forks under their parent the same way. `forks` is a DFS of descendants (parent before
	// child), so indenting each row by its depth nests it under its parent.
	const familyDepths = $derived(
		new Map(buildWorkspaceHierarchy($userWorkspaces).map((h) => [h.workspace.id, h.depth]))
	)
	// Extra left padding (on top of the row's base px-3) to nest a workspace one step per depth level,
	// matching the sidebar menu's `depth * 16px`.
	function indentStyle(id: string): string | undefined {
		const depth = familyDepths.get(id) ?? 0
		return depth > 0 ? `padding-left: ${12 + depth * 16}px` : undefined
	}

	// Judge the prod root off its OWN protection rules (fetched), not the active
	// workspace's — so it reads correctly from a dev/fork too, the same way
	// ParentWorkspaceProtectionAlert checks the parent. It's selectable when you
	// can deploy to it, and "Fork from prod" shows when you can fork it.
	const rootRulesetsResource = resource(
		() => root?.id,
		async (id) => (id ? await fetchProtectionRulesForWorkspace(id) : [])
	)
	const rootRulesets = $derived(rootRulesetsResource.current ?? [])
	// Bypass must be judged with the user's identity IN THE ROOT (is_admin/groups are per-workspace),
	// not the active workspace's `$userStore` — otherwise an admin of the dev/active workspace who is a
	// non-admin of the root would get a false bypass. `getUserExt` returns undefined for a non-member,
	// which `canUserBypassRuleKindInRulesets` treats as no bypass.
	const rootUserInfoResource = resource(
		() => root?.id,
		async (id) => (id ? await getUserExt(id) : undefined)
	)
	const rootUserInfo = $derived(rootUserInfoResource.current)
	const canDeployRoot = $derived(
		!isRuleActiveInRulesets(rootRulesets, 'DisableDirectDeployment') ||
			canUserBypassRuleKindInRulesets(rootRulesets, 'DisableDirectDeployment', rootUserInfo)
	)
	// A genuinely deploy-locked root (you can't deploy and can't bypass) is disabled regardless of
	// whether a dev workspace exists to steer to — being deploy-locked is the gate, not dev presence.
	// Roots with no rules resolve `canDeployRoot` to true, so ordinary families aren't affected. Kept
	// disabled while either fetch is in flight (both default to the conservative locked state).
	const rootDisabled = $derived(
		rootRulesetsResource.loading || rootUserInfoResource.loading || !canDeployRoot
	)

	// Structural gate: hidden in the admins workspace, or when the user can't fork; on cloud, forking
	// is premium-only (backend caps it per paid seat). DisableWorkspaceForking on the active workspace
	// (a locked prod) doesn't apply when there's a dev to fork from instead — the dev isn't locked, and
	// devOfRoot only resolves when the user is a member of it.
	const forksGateOpen = $derived(
		(!isCloudHosted() || $isPremiumStore) &&
			$workspaceStore !== 'admins' &&
			(canCreateFork($userStore) || !!devOfRoot)
	)
	// A fork is a new workspace, so it's subject to the community-edition cap on
	// the number of non-'admins' workspaces (backend _check_nb_of_workspaces,
	// run only on community builds). An enterprise license lifts the cap. We
	// mirror the backend count with the client-side workspace list to hide the
	// affordance once the cap is reached; the server still enforces the real
	// (instance-wide) check on commit, so this is purely UX.
	const CE_MAX_NON_ADMIN_WORKSPACES = 2
	const nonAdminWorkspaceCount = $derived($userWorkspaces.filter((w) => w.id !== 'admins').length)
	const ceWorkspaceCapReached = $derived(
		!$enterpriseLicense && nonAdminWorkspaceCount >= CE_MAX_NON_ADMIN_WORKSPACES
	)
	// The interactive create-fork row is shown unless the cap is reached;
	// otherwise (structural gate open but cap hit) we surface a disabled row
	// explaining the limit — never stage a fork the backend would reject.
	const forkAffordanceOpen = $derived(
		allowCreateFork && forksGateOpen && (!!onCreateFork || !!onRequestCreateFork) && !!root
	)
	// The upsell (CE workspace cap) only applies to in-place inline creation;
	// onRequestCreateFork delegates to a flow that enforces its own limits.
	const showCreateFork = $derived(
		forkAffordanceOpen && (!ceWorkspaceCapReached || !!onRequestCreateFork)
	)
	const showForkUpsell = $derived(
		forkAffordanceOpen && ceWorkspaceCapReached && !onRequestCreateFork
	)

	let dropdownOpen = $state(false)
	let creatingFork = $state(false)
	// Bare fork id, without the wm-fork- prefix. Forks have no separate display
	// name — the id is the name (it also becomes the git branch).
	let newForkId = $state('')
	// The base ("target") a new fork will branch from. Defaults to the root; the user can pick any fork
	// in the family (via the inline target selector) to create a fork of a fork.
	let createForkBaseId = $state<string | undefined>(undefined)

	// Manual keyboard navigation, modelled after SelectDropdown. melt's
	// menu API couples Enter/Space to closing the menu, which we explicitly
	// don't want for the "Create new fork" row — it swaps to inline input.
	type NavRow = { kind: 'create' } | { kind: 'root'; id: string } | { kind: 'fork'; id: string }
	// Visual order: family first, create-fork last — keep in sync with the markup.
	const navRows = $derived<NavRow[]>([
		...(root ? [{ kind: 'root' as const, id: root.id }] : []),
		...forks.map((f) => ({ kind: 'fork' as const, id: f.id })),
		...(showCreateFork ? [{ kind: 'create' as const }] : [])
	])
	let keyArrowPos = $state<number | undefined>(undefined)
	$effect(() => {
		if (!dropdownOpen) keyArrowPos = undefined
	})

	function activateRow(row: NavRow) {
		if (row.kind === 'create') {
			requestCreateFork()
		} else if (row.kind === 'root') {
			if (!rootDisabled) void pick(row.id)
		} else if (row.kind === 'fork') {
			void pick(row.id)
		}
	}

	// Delegated-create mode (onRequestCreateFork) closes the dropdown and hands
	// off; inline mode reveals the name input.
	function requestCreateFork() {
		if (onRequestCreateFork) {
			dropdownOpen = false
			creatingFork = false
			onRequestCreateFork()
		} else {
			void enterCreateMode()
		}
	}

	function defaultForkId(): string {
		const taken = new Set($userWorkspaces.map((w) => w.id))
		if (pendingFork) taken.add(pendingFork.id)
		for (let i = 0; i < 50; i++) {
			const candidate = `${random_adj()}-fork`
			if (!taken.has(`${WM_FORK_PREFIX}${candidate}`)) return candidate
		}
		const base = `${random_adj()}-fork`
		let n = 1
		while (taken.has(`${WM_FORK_PREFIX}${base}-${n}`)) n++
		return `${base}-${n}`
	}

	async function pick(id: string) {
		dropdownOpen = false
		creatingFork = false
		await onPick(id)
	}

	function enterCreateMode(initialId?: string, baseId?: string) {
		// Default the target to the root; the user can switch to any fork in the family (fork of a fork).
		createForkBaseId = baseId ?? root?.id
		creatingFork = true
		newForkId = initialId ?? defaultForkId()
		// Focus + select is handled by the input's own autofocus (it mounts with
		// the create form).
	}

	function cancelCreate() {
		creatingFork = false
		newForkId = ''
		serverIdError = undefined
	}

	// Mirror the backend's fork validation so a bad id is caught before the create call rather than
	// failing mid-creation: with the `wm-fork-` prefix the id must stay within 50 chars (a hard
	// DB/git-branch limit) and only contain slug characters.
	const clientIdError = $derived.by<string | undefined>(() => {
		const trimmed = newForkId.trim()
		if (!trimmed) return undefined
		if (!/^\w+(-\w+)*$/.test(trimmed))
			return 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		const prefixed = `${WM_FORK_PREFIX}${trimmed}`
		if (prefixed.length > 50) return `ID is too long (max ${50 - WM_FORK_PREFIX.length} characters)`
		const taken = new Set($userWorkspaces.map((w) => w.id))
		if (pendingFork) taken.delete(pendingFork.id)
		if (taken.has(prefixed)) return 'A workspace with this ID already exists'
		return undefined
	})
	// Server-reported reservation (e.g. an archived fork keeps its id, invisible
	// to the client workspace list). Set at stage time, cleared on edit.
	let serverIdError = $state<string | undefined>(undefined)
	const forkIdError = $derived(clientIdError ?? serverIdError)

	let staging = $state(false)
	async function stageNewFork() {
		const bareId = newForkId.trim()
		if (!createForkBaseId || !bareId || clientIdError || staging || !onCreateFork) return
		const prefixed = `${WM_FORK_PREFIX}${bareId}`
		staging = true
		try {
			if (await WorkspaceService.existsWorkspace({ requestBody: { id: prefixed } })) {
				serverIdError = `'${prefixed}' already exists — it may be an archived fork (archiving keeps the id reserved)`
				return
			}
		} catch {
			// Availability check failed (network/API): fall through — the create
			// call re-checks server-side anyway.
		} finally {
			staging = false
		}
		// Close optimistically; consumer can re-open + toast on error.
		creatingFork = false
		newForkId = ''
		dropdownOpen = false
		// The bare id doubles as the display name — forks have no separate name.
		await onCreateFork({ parent_workspace_id: createForkBaseId, id: prefixed, name: bareId })
	}

	function isSelected(id: string): boolean {
		if (pendingFork?.id === id) return true
		return !pendingFork && effectiveId === id
	}

	// Reopening the dropdown while a pending fork is staged drops the user
	// directly into edit mode so they can refine the id. Avoids re-
	// entering edit mode after an explicit cancel.
	let lastDropdownOpen = $state(false)
	$effect(() => {
		const wasOpen = lastDropdownOpen
		lastDropdownOpen = dropdownOpen
		if (dropdownOpen && !wasOpen && pendingFork && !creatingFork && showCreateFork) {
			// Preserve the base the fork was staged from; without this the re-entry defaults to the root
			// and silently re-parents a fork that was staged off another fork.
			void enterCreateMode(
				pendingFork.id.startsWith(WM_FORK_PREFIX)
					? pendingFork.id.slice(WM_FORK_PREFIX.length)
					: pendingFork.name,
				pendingFork.parent_workspace_id
			)
		}
	})
</script>

<svelte:window
	onkeydown={(e) => {
		if (!dropdownOpen) return
		if (creatingFork) return
		if (navRows.length === 0) return
		if (e.key === 'ArrowDown') {
			keyArrowPos = keyArrowPos === undefined ? 0 : Math.min(navRows.length - 1, keyArrowPos + 1)
			e.preventDefault()
		} else if (e.key === 'ArrowUp') {
			keyArrowPos = keyArrowPos === undefined ? navRows.length - 1 : Math.max(0, keyArrowPos - 1)
			e.preventDefault()
		} else if (e.key === 'Enter' && keyArrowPos !== undefined) {
			activateRow(navRows[keyArrowPos])
			e.preventDefault()
		} else if (e.key === 'Escape') {
			dropdownOpen = false
			e.preventDefault()
		}
	}}
/>

<DropdownV2
	bind:open={dropdownOpen}
	customMenu
	placement="bottom-start"
	fixedHeight={false}
	usePointerDownOutside
	class={triggerClass}
>
	{#snippet buttonReplacement()}
		{@render trigger({ open: dropdownOpen })}
	{/snippet}
	{#snippet menu()}
		{@const rowBase =
			'px-3 py-1.5 text-xs font-normal text-primary flex flex-row gap-2 items-center text-left rounded-sm w-full'}
		<div
			class="bg-surface-tertiary dark:border w-64 origin-top-left rounded-lg shadow-lg focus:outline-none py-1 flex flex-col max-h-[min(50vh,26rem)]"
		>
			<!-- Scrollable rows; the create-fork section and settings footer below stay pinned. -->
			<div class="flex flex-col overflow-y-auto min-h-0">
				{#if root}
					{@const rootIdx = 0}
					<!-- Rows are <button>s, so the copy affordance lives in a hover-revealed
					     absolute sibling (nested buttons are invalid HTML). pr-8 reserves its
					     spot so nothing shifts or gets covered on hover. -->
					<div class="relative group shrink-0">
						<button
							type="button"
							disabled={rootDisabled}
							title={rootDisabled
								? devOfRoot
									? `${root.name} is locked. Run in its ${devLabelNoun(devOfRoot.dev_workspace_label)} instead.`
									: `${root.name} is locked for direct deploys.`
								: undefined}
							class={`${rowBase} pr-8 ${rootDisabled ? 'opacity-50 cursor-not-allowed' : ''} ${isSelected(root.id) && !pendingFork ? 'bg-surface-selected' : ''} ${!rootDisabled && keyArrowPos === rootIdx ? 'bg-surface-hover' : !rootDisabled ? 'hover:bg-surface-hover' : ''}`}
							onmouseenter={() => !rootDisabled && (keyArrowPos = rootIdx)}
							onclick={() => !rootDisabled && void pick(root.id)}
						>
							<Building size={14} class="shrink-0 text-tertiary" />
							<span class="truncate">{root.name}</span>
							<span class="text-2xs text-tertiary shrink-0 ml-auto"
								>{rootDisabled ? 'locked' : 'root'}</span
							>
							{#if isSelected(root.id) && !pendingFork}
								<Check size={14} class="shrink-0 text-accent" />
							{/if}
						</button>
						<CopyButton
							value={root.id}
							title={`Copy id: ${root.id}`}
							class="absolute right-1.5 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
						/>
					</div>
				{/if}
				{#each forks as f, fi (f.id)}
					{@const forkIdx = (root ? 1 : 0) + fi}
					{@const accentStyle = forkAccentStyle(f.color)}
					<div class="relative group shrink-0">
						<button
							type="button"
							style={[indentStyle(f.id), accentStyle].filter(Boolean).join('; ')}
							class={`${rowBase} pr-8 ${isSelected(f.id) ? 'bg-surface-selected' : ''} ${keyArrowPos === forkIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
							onmouseenter={() => (keyArrowPos = forkIdx)}
							onclick={() => void pick(f.id)}
						>
							<GitFork
								size={14}
								class={`shrink-0 ${accentStyle ? 'text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]' : 'text-tertiary'}`}
							/>
							<span
								class={`truncate ${accentStyle ? 'text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]' : ''}`}
								>{f.name}</span
							>
							{#if f.is_dev_workspace}
								<Badge
									color="dark-blue"
									small
									class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white"
									>{devBadgeText(f.dev_workspace_label)}</Badge
								>
							{/if}
							{#if isSelected(f.id)}
								<Check size={14} class="shrink-0 ml-auto text-accent" />
							{/if}
						</button>
						<CopyButton
							value={f.id}
							title={`Copy id: ${f.id}`}
							class="absolute right-1.5 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
						/>
					</div>
				{/each}
				{#if pendingFork && !creatingFork}
					<div
						class="px-3 py-1.5 text-xs text-primary flex flex-row gap-2 items-center text-left rounded-sm bg-surface-selected cursor-default"
					>
						<GitFork size={14} class="shrink-0 text-tertiary" />
						<span class="truncate">{pendingFork.name}</span>
						<span class="text-2xs text-tertiary italic shrink-0 ml-auto">New</span>
					</div>
				{/if}
			</div>
			{#if showCreateFork}
				<div class="my-1 border-t border-border-light shrink-0"></div>
				{#if creatingFork}
					<!-- Small inline form with labels: fork id + base ("target") workspace. The base
							     defaults to the root; picking a fork there creates a fork of a fork. -->
					<div class="flex flex-col gap-2 px-2.5 py-2">
						<div class="flex flex-col gap-0.5">
							<span class="text-2xs font-normal text-hint">Fork ID</span>
							<PrefixedInput
								prefix={WM_FORK_PREFIX}
								bind:value={newForkId}
								placeholder="my-fork"
								autofocus
								error={!!forkIdError}
								class="min-h-7"
								aria-invalid={forkIdError ? 'true' : undefined}
								oninput={() => (serverIdError = undefined)}
								onkeydown={(e: KeyboardEvent) => {
									if (e.key === 'Enter') {
										e.preventDefault()
										e.stopPropagation()
										void stageNewFork()
									} else if (e.key === 'Escape') {
										e.preventDefault()
										e.stopPropagation()
										cancelCreate()
									}
								}}
							/>
						</div>
						<div class="flex flex-col gap-0.5">
							<span class="text-2xs font-normal text-hint">Base workspace</span>
							<Select
								size="sm"
								class="w-full"
								disablePortal
								placeholder="Base workspace"
								items={baseItems}
								bind:value={createForkBaseId}
							>
								{#snippet startSnippet({ item })}
									{@const d = familyDepths.get(item.value) ?? 0}
									{#if d > 0}
										<span class="inline-block shrink-0" style="width: {d * 14}px"></span>
									{/if}
								{/snippet}
							</Select>
						</div>
						{#if forkIdError || createForkCaption}
							<div>
								<InputError error={forkIdError} />
								{#if !forkIdError && createForkCaption}
									<span class="text-2xs text-tertiary">{createForkCaption}</span>
								{/if}
							</div>
						{/if}
						<div class="flex flex-row justify-end items-center gap-1.5 pt-0.5">
							<Button variant="subtle" unifiedSize="xs" on:click={() => cancelCreate()}>
								Cancel
							</Button>
							<Button
								variant="accent"
								unifiedSize="xs"
								startIcon={{ icon: Check }}
								disabled={!newForkId.trim() || !!forkIdError || staging}
								on:click={() => void stageNewFork()}
							>
								Set as target
							</Button>
						</div>
					</div>
				{:else}
					{@const createIdx = (root ? 1 : 0) + forks.length}
					<button
						type="button"
						class={`${rowBase} ${keyArrowPos === createIdx ? 'bg-surface-hover' : 'hover:bg-surface-hover'}`}
						onmouseenter={() => (keyArrowPos = createIdx)}
						onclick={() => requestCreateFork()}
					>
						<Plus size={14} class="shrink-0 text-tertiary" />
						<span>{createForkLabel}</span>
					</button>
				{/if}
			{:else if showForkUpsell}
				<div class="my-1 border-t border-border-light shrink-0"></div>
				<div
					class={`${rowBase} opacity-60 cursor-not-allowed`}
					aria-disabled="true"
					title="Community edition is limited to {CE_MAX_NON_ADMIN_WORKSPACES +
						1} workspaces. Archive a workspace or upgrade to an enterprise license to create more forks."
				>
					<Plus size={14} class="shrink-0 text-tertiary" />
					<span>{createForkLabel}</span>
					<span class="ml-auto shrink-0 text-2xs text-tertiary"> Workspace limit reached </span>
				</div>
			{/if}
			{#if settingsHref}
				<!-- Pinned footer: stays visible while the fork list above scrolls. -->
				<div class="my-1 border-t border-border-light shrink-0"></div>
				<a
					href={settingsHref}
					onclick={() => (dropdownOpen = false)}
					class={`${rowBase} shrink-0 hover:bg-surface-hover`}
				>
					<Settings size={14} class="shrink-0 text-tertiary" />
					<span class="truncate">{settingsLabel ?? 'Workspace settings'}</span>
				</a>
			{/if}
		</div>
	{/snippet}
</DropdownV2>
