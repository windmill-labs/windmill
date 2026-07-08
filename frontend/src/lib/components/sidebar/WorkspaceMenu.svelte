<script lang="ts">
	import { workspaceMenuHref } from './workspaceMenuHref'
	import {
		isPremiumStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore,
		workspaceColor,
		clearWorkspaceFromStorage,
		globalForkModal,
		type UserWorkspace
	} from '$lib/stores'
	import { Building, Check, ChevronDown, ChevronRight, Plus, Settings } from 'lucide-svelte'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { SvelteSet } from 'svelte/reactivity'
	import { Badge, CopyButton, NameIdTooltip } from '$lib/components/common'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import WorkspaceIcon from '$lib/components/workspace/WorkspaceIcon.svelte'
	import { fixupUrlAfterWorkspaceSwitch } from './workspaceSwitchUrl'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/state'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { workspaceAIClients } from '../copilot/lib'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import { buildWorkspaceHierarchy } from '$lib/utils/workspaceHierarchy'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { getContrastTextColor } from '$lib/utils'
	import { workspaceRootId } from '$lib/components/sessions/sessionScope.svelte'
	import { devBadgeText } from '$lib/utils/devWorkspaceLabel'

	interface Props {
		isCollapsed?: boolean
		createMenu: MenubarBuilders['createMenu']
		// When used outside of the side bar, where links to workspace settings and such don't make as much sense.
		strictWorkspaceSelect?: boolean
	}

	let { isCollapsed = false, createMenu, strictWorkspaceSelect = false }: Props = $props()

	async function toggleSwitchWorkspace(id: string) {
		if ($workspaceStore === id) {
			return
		}
		workspaceAIClients.init(id)
		switchWorkspace(id)
		// The sessions page needs no navigation here: the item's link navigation
		// (workspaceHref) keeps the route, and the page's family reconcile swaps
		// out a chat that doesn't belong to the new workspace's family.
		await fixupUrlAfterWorkspaceSwitch(id)
	}

	// Href for the item's navigation (including modifier/middle clicks opening a
	// new tab): same page, `workspace` param swapped to the clicked id, the open
	// session kept only within its family. Pure logic lives in workspaceMenuHref
	// (unit-tested).
	function workspaceHref(id: string): string {
		const all = $userWorkspaces ?? []
		return workspaceMenuHref({
			pathname: page.url.pathname,
			searchParams: page.url.searchParams,
			id,
			sameFamily: workspaceRootId(id, all) === workspaceRootId($workspaceStore ?? undefined, all)
		})
	}

	function onWorkspaceItemClick(e: MouseEvent, workspace: { id: string; disabled?: boolean }) {
		if (workspace.disabled) {
			e.preventDefault()
			return
		}
		// Let modifier-keyed clicks fall through so the browser can open in a new tab.
		if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) {
			return
		}
		e.preventDefault()
		toggleSwitchWorkspace(workspace.id)
	}

	// Family-first picker: list the workspace families (roots) with their forks
	// collapsed behind a per-family chevron, so direct fork navigation stays
	// available without flattening fork-heavy instances into a long menu. The
	// scope header's WorkspaceFamilyPicker remains the primary fork surface.
	//
	// strictWorkspaceSelect is used on standalone pages (e.g. svix webhook
	// creation) that render this menu with no scope header, so there forks must
	// stay directly selectable — list the full hierarchy unconditionally.
	const hierarchy = $derived($userWorkspaces ? buildWorkspaceHierarchy($userWorkspaces) : [])
	const expandedFamilies = new SvelteSet<string>()
	// Root ids with at least one fork — only they get the expand chevron.
	// hierarchy is a DFS (parent before child), so a depth>0 row belongs to the
	// last depth-0 row seen.
	const familiesWithForks = $derived.by(() => {
		const withForks = new Set<string>()
		let rootId: string | undefined
		for (const h of hierarchy) {
			if (h.depth === 0) rootId = h.workspace.id
			else if (rootId) withForks.add(rootId)
		}
		return withForks
	})
	// Gate for the "Workspace fork" entry pinned below the list (the global fork
	// modal carries its own base-workspace picker). Hidden on non-premium cloud,
	// in the admins workspace, or when forking is disabled.
	const canForkHere = $derived(
		(!isCloudHosted() || $isPremiumStore) &&
			$workspaceStore !== 'admins' &&
			canCreateFork($userStore)
	)
	const familyWorkspaces = $derived.by(() => {
		if (strictWorkspaceSelect) return hierarchy
		let rootId: string | undefined
		return hierarchy.filter((h) => {
			if (h.depth === 0) {
				rootId = h.workspace.id
				return true
			}
			return !!rootId && expandedFamilies.has(rootId)
		})
	})

	let menuOpen = $state(false)

	// ArrowRight/ArrowLeft expand/collapse the keyboard-highlighted family (melt
	// stamps data-highlighted on the item; the row wrapper carries the workspace
	// id). Capture phase, so melt's menubar left/right (adjacent-menu switching)
	// doesn't fire when the keypress means expansion here.
	function onExpandKeydown(e: KeyboardEvent) {
		if (!menuOpen || strictWorkspaceSelect) return
		if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return
		// Scoped inside a row wrapper: the menubar trigger also carries
		// data-highlighted while its menu is open.
		const id = document
			.querySelector('[data-workspace-id] [data-highlighted]')
			?.closest('[data-workspace-id]')
			?.getAttribute('data-workspace-id')
		if (!id || !familiesWithForks.has(id)) return
		if (e.key === 'ArrowRight' && !expandedFamilies.has(id)) expandedFamilies.add(id)
		else if (e.key === 'ArrowLeft' && expandedFamilies.has(id)) expandedFamilies.delete(id)
		else return
		e.preventDefault()
		e.stopPropagation()
	}

	function findRoot(id: string | undefined): UserWorkspace | undefined {
		if (!id || !$userWorkspaces) return undefined
		let current = $userWorkspaces.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = $userWorkspaces.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	// The active workspace's family root — shown in the trigger so a forked
	// active workspace still surfaces its family name here (the fork itself is
	// shown in the breadcrumb).
	const currentFamily = $derived(findRoot($workspaceStore ?? undefined))

	// The active workspace itself (fork included) — names the settings entry.
	const activeWorkspace = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const canManageWorkspace = $derived($userStore?.is_admin || $superadmin)

	// font-normal is explicit: href-less MenuItems render as <button>, which the
	// global stylesheet makes semibold, unlike the <a> the href entries get.
	const itemClass =
		'text-primary font-normal w-full flex flex-row gap-2 px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
</script>

<svelte:window onkeydowncapture={onExpandKeydown} />

<!-- Expansion is per-open: every open starts with all families collapsed,
     including the active fork's. -->
<Menu
	{createMenu}
	usePointerDownOutside
	placement="bottom-start"
	bind:open={menuOpen}
	on:close={() => expandedFamilies.clear()}
>
	{#snippet triggr({ trigger })}
		<!-- Family header reflects the family (root) color, not the active
		     workspace's — switching into a fork must not recolor it. -->
		{@const familyColor = currentFamily?.color ?? $workspaceColor}
		{@const iconColor = getContrastTextColor(familyColor)}
		<!-- Collapsed mode already shows MenuButton's own right-side popover, so the
		     rich tooltip only takes over when expanded. -->
		<NameIdTooltip
			name={currentFamily?.name ?? $workspaceStore ?? ''}
			id={currentFamily?.id ?? $workspaceStore ?? ''}
			disablePopup={isCollapsed}
			class="block w-full"
		>
			<MenuButton
				icon={Building}
				iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
				label={currentFamily?.name ?? $workspaceStore ?? ''}
				{isCollapsed}
				color={familyColor}
				showChevron
				emphasizeLabel
				disableTitle
				{trigger}
			/>
		</NameIdTooltip>
	{/snippet}

	{#snippet children({ item })}
		<div class="divide-y" role="none">
			<!-- The list scrolls internally so the sections below (+ Workspace,
			     Workspace fork, All workspaces) stay visible however long it gets. -->
			<div class="py-1 overflow-y-auto max-h-[min(50vh,26rem)]">
				{#each familyWorkspaces as { workspace, depth, isForked, parentName }}
					{@const isActive = $workspaceStore === workspace.id}
					{@const forkAccent = isForked ? forkAccentStyle(workspace.color) : undefined}
					<!-- Selection is a trailing tick (single-choice picker convention). A
					     collapsed root carries the tick while one of its forks is active —
					     the fork row is hidden and its family is the only trace of the
					     selection; expanding moves the tick to the fork itself. -->
					{@const isSelected =
						isActive ||
						(!strictWorkspaceSelect &&
							depth === 0 &&
							currentFamily?.id === workspace.id &&
							!expandedFamilies.has(workspace.id))}
					{@const expandable =
						!strictWorkspaceSelect && depth === 0 && familiesWithForks.has(workspace.id)}
					<!-- The expand chevron sits OUTSIDE the melt item: melt activates items
					     via document-level handlers, so a nested button can't stop the row's
					     navigate-and-close with stopPropagation. As a sibling it toggles the
					     family without selecting the row or closing the menu. -->
					<!-- Hover/keyboard-highlight backgrounds live on the wrapper (the
					     highlight via :has(), since melt puts data-highlighted on the item) so
					     they span the full row, chevron included; the chevron keeps only its
					     own hover tint. The active workspace is marked by its row's check
					     icon, not a background or text accent. -->
					<div
						data-workspace-id={workspace.id}
						class={twMerge(
							'group flex items-center min-w-0 w-full',
							workspace.disabled
								? ''
								: 'hover:bg-surface-hover [&:has([data-highlighted])]:bg-surface-hover'
						)}
					>
						<MenuItem
							class={twMerge(
								'text-xs min-w-0 flex-1 overflow-hidden flex flex-col py-1.5 px-3',
								workspace.disabled && 'opacity-50 cursor-not-allowed',
								isActive ? 'cursor-default' : workspace.disabled ? '' : 'cursor-pointer'
							)}
							href={workspace.disabled ? undefined : workspaceHref(workspace.id)}
							onClick={(e) => onWorkspaceItemClick(e, workspace)}
							{item}
						>
							<div class="flex items-center justify-between gap-2 min-w-0 w-full">
								<div class="flex items-center gap-2 min-w-0" style:padding-left={`${depth * 16}px`}>
									<WorkspaceIcon
										workspaceColor={workspace.color}
										{isForked}
										isDevWorkspace={workspace.is_dev_workspace}
										devWorkspaceLabel={workspace.dev_workspace_label}
										{parentName}
										padding="p-1"
									/>
									<div class="min-w-0 flex-1">
										<div class="flex items-center gap-1 min-w-0">
											<div
												class="truncate text-left text-xs font-normal {forkAccent
													? 'text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]'
													: 'text-primary'}"
												style={forkAccent}
												title={workspace.name}
											>
												{workspace.name}{workspace.disabled ? ' (user disabled)' : ''}
											</div>
											{#if workspace.is_dev_workspace}
												<Badge
													color="dark-blue"
													small
													class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white"
													>{devBadgeText(workspace.dev_workspace_label)}</Badge
												>
											{/if}
										</div>
									</div>
								</div>
								{#if isSelected}
									<Check size={14} class="shrink-0 ml-2 text-accent" />
								{/if}
							</div>
						</MenuItem>
						<!-- Hover-revealed like the fork picker's rows; a flex sibling of the melt
						     item (nested buttons are invalid HTML) that always occupies its slot so
						     nothing shifts on hover. -->
						<div
							class="shrink-0 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
						>
							<CopyButton value={workspace.id} title={`Copy id: ${workspace.id}`} />
						</div>
						{#if expandable}
							{@const expanded = expandedFamilies.has(workspace.id)}
							<button
								type="button"
								title={expanded ? 'Hide forks' : 'Show forks'}
								aria-expanded={expanded}
								class="shrink-0 mr-1 px-2 self-stretch flex items-center rounded text-tertiary hover:bg-surface-hover hover:text-primary"
								onclick={() => {
									if (expanded) expandedFamilies.delete(workspace.id)
									else expandedFamilies.add(workspace.id)
								}}
							>
								{#if expanded}
									<ChevronDown size={14} />
								{:else}
									<ChevronRight size={14} />
								{/if}
							</button>
						{:else}
							<!-- Reserve the expand-chevron's slot on fork-less rows so the hover copy
							     button lines up with the forked rows' and never sits flush against the
							     menu's right edge. Matches the chevron's px-2 (30px) + mr-1. -->
							<div class="shrink-0 mr-1 w-[30px]" aria-hidden="true"></div>
						{/if}
					</div>
				{/each}
			</div>
			{#if (isCloudHosted() || $superadmin || canForkHere) && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					{#if isCloudHosted() || $superadmin}
						<MenuItem href="{base}/user/create_workspace" class={itemClass} {item}>
							<Plus size={16} />
							Workspace
						</MenuItem>
					{/if}
					{#if canForkHere}
						<MenuItem
							class={itemClass}
							onClick={() => (globalForkModal.val = { opened: true })}
							{item}
						>
							<Plus size={16} />
							Workspace fork
						</MenuItem>
					{/if}
				</div>
			{/if}
			{#if canManageWorkspace && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem
						href="{base}/workspace_settings?workspace={$workspaceStore}"
						class={itemClass}
						{item}
					>
						<Settings size={16} />
						{activeWorkspace?.name ?? $workspaceStore} settings
					</MenuItem>
				</div>
			{/if}
			{#if !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem
						href="{base}/user/workspaces"
						onClick={() => clearWorkspaceFromStorage()}
						class={itemClass}
						{item}
					>
						All workspaces
					</MenuItem>
				</div>
			{/if}
		</div>
		{#if isCloudHosted() && !$isPremiumStore && !strictWorkspaceSelect}
			<div class="py-1" role="none">
				{#if $workspaceStore != 'demo'}
					<span class="text-secondary block w-full text-left px-4 py-2 text-xs"
						>{$workspaceUsageStore}/1000 free workspace execs</span
					>
					<div class="w-full bg-gray-200 h-1">
						<div class="bg-blue-400 h-1" style="width: {Math.min($workspaceUsageStore, 1000) / 10}%"
						></div>
					</div>
				{/if}
				{#if $userStore?.is_admin}
					<MenuItem
						class={twMerge(
							'text-secondary block font-normal w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900',
							'data-[highlighted]:bg-gray-100 data-[highlighted]:text-gray-900'
						)}
						onClick={() => {
							goto('/workspace_settings?tab=premium')
						}}
						{item}
					>
						Upgrade
					</MenuItem>
				{/if}
			</div>
		{/if}
		{#if $enterpriseLicense && !strictWorkspaceSelect}
			<MultiplayerMenu />
		{/if}
	{/snippet}
</Menu>
