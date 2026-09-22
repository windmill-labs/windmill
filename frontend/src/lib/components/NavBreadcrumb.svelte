<!--
@component
The page header's breadcrumb: workspace / fork / kind / path, in one flat line.

Every segment is a trigger for the picker that changes it — the workspace menu, the fork family
picker, and the item drill picker for the kind and each path level — and every segment wears the
same weight, size and icon size, so the line reads as one control rather than four.
-->
<script lang="ts">
	import { Building, Folder, GitFork, User } from 'lucide-svelte'
	import WorkspaceItemKindIcon from './WorkspaceItemKindIcon.svelte'
	import { Menu, Menubar } from '$lib/components/meltComponents'
	import NavBreadcrumbTrigger from './NavBreadcrumbTrigger.svelte'
	import BreadcrumbItemContent from './BreadcrumbItemContent.svelte'
	import WorkspacePickerBody from '$lib/components/sidebar/WorkspacePickerBody.svelte'
	import WorkspaceFamilyPicker from '$lib/components/sessions/WorkspaceFamilyPicker.svelte'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import {
		userWorkspaces,
		workspaceStore,
		workspaceColor,
		globalForkModal,
		userStore,
		superadmin
	} from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { fixupUrlAfterWorkspaceSwitch } from './sidebar/workspaceSwitchUrl'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import {
		findWorkspaceRoot,
		findWorkspaceDescendants,
		isForkOwner
	} from '$lib/utils/workspaceHierarchy'
	import { useForkableWorkspaces } from '$lib/utils/useForkableWorkspaces.svelte'
	import { base } from '$lib/base'
	import { twMerge } from 'tailwind-merge'
	import { Badge } from '$lib/components/common'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { devBadgeText } from '$lib/utils/devWorkspaceLabel'
	import { page } from '$app/state'
	import { navPageFor } from './sidebar/navPages'
	import { getContrastTextColor } from '$lib/utils'
	import { splitItemPath } from './breadcrumbPath'
	import {
		dirKey,
		KIND_LABEL_LOWER,
		kindKey,
		leafKeyFor,
		type WorkspaceItem
	} from '$lib/components/workspacePicker'
	import type { PageHeaderItem, PageHeaderSection } from './pageHeaderRegistry.svelte'

	let {
		item,
		section,
		actingWorkspaceId
	}: {
		item?: PageHeaderItem
		section?: PageHeaderSection
		/** The workspace the page acts on, when it differs from the one the app is pointed at. */
		actingWorkspaceId?: string
	} = $props()

	const scopeId = $derived(actingWorkspaceId ?? $workspaceStore ?? undefined)

	// With no item or section registered, the route still says where the user is: Runs, Variables,
	// and the rest name themselves in the breadcrumb rather than leaving it at the workspace.
	// Home is the root the workspace part already stands for, so it names no page of its own.
	const routePage = $derived.by(() => {
		if (item || section) return undefined
		const found = navPageFor(page.url.pathname, base)
		return found?.path === '/' ? undefined : found
	})

	/** One look for every segment, whichever picker it opens. */
	const SEGMENT =
		'flex items-center gap-1 min-w-0 px-1 py-0.5 rounded text-xs font-normal text-primary hover:bg-surface-hover hover:text-emphasis transition-colors'
	const ICON = 14

	const family = $derived(findWorkspaceRoot(scopeId, $userWorkspaces ?? []))
	const workspaceName = $derived(family?.name ?? scopeId ?? '')
	const discColor = $derived(family?.color ?? $workspaceColor ?? 'rgb(var(--color-surface-sunken))')
	const glyphColor = $derived(getContrastTextColor(discColor))

	const forkable = useForkableWorkspaces({
		workspaces: () => $userWorkspaces,
		currentWorkspaceId: () => scopeId,
		isSuperadmin: () => !!$superadmin
	})
	const forkableWorkspaces = $derived(forkable.current)
	const currentWs = $derived(forkableWorkspaces.find((w) => w.id === scopeId))
	const forkCount = $derived.by(() => {
		const root = findWorkspaceRoot(scopeId, forkableWorkspaces)
		return root ? findWorkspaceDescendants(root.id, forkableWorkspaces).length : 0
	})
	// A fork names itself; a root says how many it has, which is the hint that this opens them.
	const forkLabel = $derived(
		currentWs && currentWs.id !== family?.id
			? currentWs.name
			: `${forkCount} fork${forkCount === 1 ? '' : 's'}`
	)
	// Forks are a developer's workspace. An operator sees the segment only when one of them is
	// theirs — they were added to a fork, or are standing in one — and never the row that makes one.
	const inFork = $derived(!!currentWs?.parent_workspace_id)
	// A page acting on another workspace (a session) names that fork and nothing else: "0 forks"
	// would be about the workspace the user is browsing, not the one the session runs in.
	const showFork = $derived(
		actingWorkspaceId ? inFork : !$userStore?.operator || forkCount > 0 || inFork
	)

	// A fork wears its workspace colour and, for a dev workspace, its environment badge — the same
	// two marks the fork chip carries everywhere else.
	const forkAccent = $derived(inFork ? forkAccentStyle(currentWs?.color) : undefined)
	const devBadge = $derived(
		inFork && currentWs?.is_dev_workspace ? devBadgeText(currentWs.dev_workspace_label) : undefined
	)

	const canManageWorkspace = $derived(
		$userStore?.is_admin || $superadmin || isForkOwner(currentWs, $userStore?.email)
	)

	const segments = $derived(item?.path ? splitItemPath(item.path) : undefined)
	const currentItem = $derived<(WorkspaceItem & { savedPath?: string }) | undefined>(
		item
			? {
					path: item.path ?? '',
					summary: item.summary ?? '',
					kind: item.kind,
					raw_app: item.raw_app,
					savedPath: item.savedPath
				}
			: undefined
	)

	/** `f/demo` and `u/alice` name a folder and a user: the prefix becomes the segment's icon. */
	const scopeKind = $derived(segments?.dirs[0]?.fullPath.split('/')[0])

	function switchWorkspaceDirect(id: string) {
		if ($workspaceStore === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		void fixupUrlAfterWorkspaceSwitch(id)
	}
</script>

{#snippet workspaceDisc()}
	<svg width={ICON} height={ICON} viewBox="0 0 16 16" class="flex-shrink-0">
		<circle cx="8" cy="8" r="8" fill={discColor} />
		<foreignObject x="3" y="3" width="10" height="10">
			<Building size={10} style={glyphColor ? `color: ${glyphColor}` : undefined} />
		</foreignObject>
	</svg>
{/snippet}

{#snippet forkIcon()}
	<GitFork size={ICON} class="flex-shrink-0 text-tertiary" />
{/snippet}

{#snippet itemKindIcon()}
	{#if item}<WorkspaceItemKindIcon kind={item.kind} size={ICON} />{/if}
{/snippet}

{#snippet sectionKindIcon()}
	{#if section?.kind}<WorkspaceItemKindIcon kind={section.kind} size={ICON} />{/if}
{/snippet}

{#snippet folderIcon()}
	<Folder size={ICON} class="flex-shrink-0 text-tertiary" />
{/snippet}

{#snippet userIcon()}
	<User size={ICON} class="flex-shrink-0 text-tertiary" />
{/snippet}

{#snippet slash()}
	<span class="shrink-0 text-hint/40 text-xs" aria-hidden="true">/</span>
{/snippet}

<nav
	aria-label="Breadcrumb"
	class="flex items-center gap-1.5 min-w-0 text-xs font-normal text-primary"
>
	<Menubar>
		{#snippet children({ createMenu })}
			<Menu {createMenu} usePointerDownOutside placement="bottom-start">
				{#snippet triggr({ trigger })}
					<NavBreadcrumbTrigger {trigger} class={SEGMENT} title={workspaceName}>
						<BreadcrumbItemContent label={workspaceName} icon={workspaceDisc} />
					</NavBreadcrumbTrigger>
				{/snippet}
				{#snippet children({ item: menuItem })}
					<WorkspacePickerBody item={menuItem} />
				{/snippet}
			</Menu>
		{/snippet}
	</Menubar>

	{#if showFork}
		{@render slash()}

		<WorkspaceFamilyPicker
			selectedId={scopeId}
			{forkableWorkspaces}
			onPick={switchWorkspaceDirect}
			allowCreateFork={!$userStore?.operator}
			onRequestCreateFork={() => (globalForkModal.val = { opened: true })}
			settingsHref={canManageWorkspace ? `${base}/workspace_settings` : undefined}
			settingsLabel={`${currentWs?.name ?? scopeId ?? 'Workspace'} settings`}
			class="min-w-0"
		>
			{#snippet trigger()}
				<!-- twMerge so the accent's text colour wins over the segment's own. -->
				<span
					class={twMerge(
						SEGMENT,
						forkAccent &&
							'bg-[color:var(--fork-accent-bg)] dark:bg-[color:var(--fork-accent-bg-dark)] text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)] font-semibold'
					)}
					style={forkAccent}
				>
					<BreadcrumbItemContent label={forkLabel} icon={forkIcon} />
					{#if devBadge}
						<Badge
							color="dark-blue"
							small
							class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white"
							>{devBadge}</Badge
						>
					{/if}
				</span>
			{/snippet}
		</WorkspaceFamilyPicker>
	{/if}

	{#if item || section || routePage}
		{@render slash()}
	{/if}

	{#if item}
		{#if segments}
			{#each segments.dirs as dir, i (dir.fullPath)}
				{#if i > 0}{@render slash()}{/if}
				<BreadcrumbSegment
					label={i === 0 ? dir.name.split('/').slice(1).join('/') : dir.name}
					icon={i === 0 ? (scopeKind === 'u' ? userIcon : folderIcon) : undefined}
					extraClass="{SEGMENT} {i === 0 ? 'max-w-[40%]' : ''}"
					initialScope={i === 0
						? { kind: 'all' }
						: { kind: 'all', dir: segments.dirs[i - 1].fullPath }}
					initialHighlight={dirKey('all', dir.fullPath)}
					{currentItem}
					workspaceId={item.workspaceId}
					onPick={(picked) => item?.onNavigate?.(picked)}
				/>
			{/each}
			{@render slash()}
			<BreadcrumbSegment
				label={segments.leaf.name}
				extraClass={SEGMENT}
				initialScope={segments.dirs.length
					? { kind: 'all', dir: segments.dirs[segments.dirs.length - 1].fullPath }
					: { kind: 'all' }}
				initialHighlight={leafKeyFor(item.kind, segments.leaf.fullPath)}
				isCurrent
				{currentItem}
				workspaceId={item.workspaceId}
				onPick={(picked) => item?.onNavigate?.(picked)}
			/>
		{:else}
			<BreadcrumbSegment
				label={KIND_LABEL_LOWER[item.kind]}
				icon={itemKindIcon}
				extraClass={SEGMENT}
				initialHighlight={kindKey(item.kind)}
				isCurrent
				{currentItem}
				workspaceId={item.workspaceId}
				onPick={(picked) => item?.onNavigate?.(picked)}
			/>
		{/if}
	{:else if section?.kind}
		<BreadcrumbSegment
			label={KIND_LABEL_LOWER[section.kind]}
			icon={sectionKindIcon}
			extraClass={SEGMENT}
			isCurrent
			initialHighlight={kindKey(section.kind)}
			initialScope={{ kind: section.kind }}
			onPick={() => {}}
		/>
	{:else if section}
		<!-- A section's own widget lays out as a row: its title, and whatever belongs with it. -->
		<span
			class="{SEGMENT} hover:bg-transparent hover:text-primary {section.content ? '' : 'truncate'}"
			aria-current="page"
		>
			{#if section.content}{@render section.content()}{:else}{section.label}{/if}
		</span>
	{:else if routePage}
		{@const RouteIcon = routePage.icon}
		<span class="{SEGMENT} hover:bg-transparent hover:text-primary" aria-current="page">
			{#if RouteIcon}<RouteIcon size={ICON} class="flex-shrink-0 text-tertiary" />{/if}
			<span class="truncate">{routePage.label}</span>
		</span>
	{/if}
</nav>
