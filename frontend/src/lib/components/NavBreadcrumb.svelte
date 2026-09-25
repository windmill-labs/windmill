<!--
@component
The page header's breadcrumb: workspace / fork / kind / path, in one flat line.

Every segment is a trigger for the picker that changes it — the workspace menu, the fork family
picker, and the item drill picker for the kind and each path level — and every segment wears the
same weight, size and icon size, so the line reads as one control rather than four.
-->
<script lang="ts">
	import { Building, ChevronDown, Folder, GitFork, User } from 'lucide-svelte'
	import WorkspaceItemKindIcon from './WorkspaceItemKindIcon.svelte'
	import { Menu, Menubar } from '$lib/components/meltComponents'
	import NavBreadcrumbTrigger from './NavBreadcrumbTrigger.svelte'
	import BreadcrumbItemContent from './BreadcrumbItemContent.svelte'
	import WorkspacePickerBody from '$lib/components/sidebar/WorkspacePickerBody.svelte'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import { userWorkspaces, workspaceStore, workspaceColor, superadmin } from '$lib/stores'
	import { findWorkspaceRoot } from '$lib/utils/workspaceHierarchy'
	import { useForkableWorkspaces } from '$lib/utils/useForkableWorkspaces.svelte'
	import { base } from '$lib/base'
	import { twMerge } from 'tailwind-merge'
	import { Badge } from '$lib/components/common'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { devBadgeText } from '$lib/utils/devWorkspaceLabel'
	import { page } from '$app/state'
	import { navPageFor } from './sidebar/navPages'
	import { copyToClipboard, getContrastTextColor } from '$lib/utils'
	import { KIND_LABEL_LOWER, kindKey } from '$lib/components/workspacePicker'
	import type { PageHeaderItem, PageHeaderSection } from './pageHeaderRegistry.svelte'
	import type { Snippet } from 'svelte'

	let {
		item,
		section,
		afterName,
		actingWorkspaceId
	}: {
		item?: PageHeaderItem
		section?: PageHeaderSection
		/** What sits beside the page's name: a mark like a documentation tooltip, or a control
		 *  that acts on the thing named. */
		afterName?: Snippet
		/** The workspace the page acts on, when it differs from the one the app is pointed at. */
		actingWorkspaceId?: string
	} = $props()

	const scopeId = $derived(actingWorkspaceId ?? $workspaceStore ?? undefined)
	// In session mode the workspace is the session's scope, not a place to navigate to: the name
	// says which workspace the chat acts on, and going home from it would leave the session.
	const sessionMode = $derived(page.url.pathname.startsWith(`${base}/sessions`))
	const homeHref = $derived(
		actingWorkspaceId ? `${base}/?workspace=${encodeURIComponent(actingWorkspaceId)}` : `${base}/`
	)

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
	/** The workspace disc: the same glyph as the rest, with a ring of its own around it. */
	const DISC = 20

	/** Width of the name beside the chevron, so the picker can hang from the part's own left edge. */
	let nameWidth = $state(0)
	let workspaceMenu: Menu | undefined = $state(undefined)

	// Its own confirmation under the path rather than the global toast: the answer belongs where
	// the click was, and a toast for something this small is a lot of furniture.
	let pathCopied = $state(false)
	let copiedReset: ReturnType<typeof setTimeout> | undefined
	async function copyPath() {
		if (!item?.path || !(await copyToClipboard(item.path, false))) return
		pathCopied = true
		clearTimeout(copiedReset)
		copiedReset = setTimeout(() => (pathCopied = false), 1500)
	}

	const family = $derived(findWorkspaceRoot(scopeId, $userWorkspaces ?? []))
	const discColor = $derived(family?.color ?? $workspaceColor ?? 'rgb(var(--color-surface-sunken))')
	const glyphColor = $derived(getContrastTextColor(discColor))

	const forkable = useForkableWorkspaces({
		workspaces: () => $userWorkspaces,
		currentWorkspaceId: () => scopeId,
		isSuperadmin: () => !!$superadmin
	})
	const forkableWorkspaces = $derived(forkable.current)
	const currentWs = $derived(forkableWorkspaces.find((w) => w.id === scopeId))
	const inFork = $derived(!!currentWs?.parent_workspace_id)
	// A session names the workspace it acts on beside its own title, so here the family name alone
	// says where the user is browsing — the fork in this spot would read as a workspace switch.
	const showFork = $derived(inFork && !actingWorkspaceId)
	const familyName = $derived(family?.name ?? scopeId ?? '')
	// The workspace the user actually stands in, which is the fork's own name inside a fork.
	const scopeName = $derived(currentWs?.name ?? familyName)

	// A fork wears its workspace colour and, for a dev workspace, its environment badge — the same
	// two marks the fork chip carries everywhere else.
	const forkAccent = $derived(inFork ? forkAccentStyle(currentWs?.color) : undefined)
	// A dev workspace wears its environment; the root wears "prod", so which environment the user
	// is standing in is answered the same way whichever one it is.
	const envBadge = $derived.by(() => {
		if (actingWorkspaceId) return undefined
		if (!inFork) return { text: 'prod', dev: false }
		if (currentWs?.is_dev_workspace)
			return { text: devBadgeText(currentWs.dev_workspace_label), dev: true }
		return undefined
	})

	// No hover fill: the chevron beside it is what reacts to the pointer. The accent stays on the
	// fork's own name inside, so the family's name is not painted as if it were the fork.
	const scopeChipClass = twMerge(SEGMENT, 'hover:bg-transparent')

	/** `f/demo` and `u/alice` name a folder and a user: the prefix becomes the path's icon. */
	const scopeKind = $derived(item?.path?.split('/')[0])
	/** The path's levels, the `f`/`u` prefix dropped — that is what the icon says. */
	const pathLevels = $derived((item?.path ?? '').split('/').slice(1).filter(Boolean))
</script>

{#snippet workspaceDisc()}
	<!-- The glyph is the same size as every other icon on the line; only the disc around it is
	     larger, by the ring it needs to read as a disc at all. -->
	<svg width={DISC} height={DISC} viewBox="0 0 {DISC} {DISC}" class="flex-shrink-0">
		<circle cx={DISC / 2} cy={DISC / 2} r={DISC / 2} fill={discColor} />
		<foreignObject x={(DISC - ICON) / 2} y={(DISC - ICON) / 2} width={ICON} height={ICON}>
			<Building size={ICON} style={glyphColor ? `color: ${glyphColor}` : undefined} />
		</foreignObject>
	</svg>
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

{#snippet envBadgeMark()}
	{#if envBadge}
		<Badge
			color={envBadge.dev ? 'dark-blue' : 'gray'}
			small
			class={envBadge.dev
				? 'text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white'
				: 'text-3xs px-1 py-0'}>{envBadge.text}</Badge
		>
	{/if}
{/snippet}

{#snippet slash()}
	<span class="shrink-0 text-hint/40 text-xs" aria-hidden="true">/</span>
{/snippet}

<nav
	aria-label="Breadcrumb"
	class="flex items-center gap-1.5 min-w-0 text-xs font-normal text-primary"
>
	<!-- One part for where the user is: the name of the workspace they are standing in — a fork
	     included, in its own colour — leading home, and one chevron whose picker changes it. The
	     picker lists the families and expands one to reach its forks, so a fork needs no part of
	     its own here. -->
	<div class="flex items-center min-w-0">
		<!-- No hover fill on the name: the chevron beside it is the thing that lights up, and two
		     boxes reacting to one pass of the pointer read as two controls fighting. -->
		<!-- bind:clientWidth: the menu hangs from the chevron, so it is shifted back by the width of
		     the name to line its left edge up with the disc. -->
		<div class="flex items-center min-w-0" bind:clientWidth={nameWidth}>
			<svelte:element
				this={sessionMode ? 'span' : 'a'}
				href={sessionMode ? undefined : homeHref}
				class={scopeChipClass}
				title={showFork ? `${familyName} / ${scopeName}` : familyName}
			>
				<BreadcrumbItemContent label={familyName} icon={workspaceDisc} />
				{#if showFork}
					<!-- Both names: the fork is where the work happens, the family is which product it is
				     part of, and either alone leaves the other to be guessed. -->
					{@render slash()}
					<span
						class={twMerge(
							'flex items-center gap-1 min-w-0 px-1 rounded',
							forkAccent &&
								'bg-[color:var(--fork-accent-bg)] dark:bg-[color:var(--fork-accent-bg-dark)] text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)] font-semibold'
						)}
						style={forkAccent}
					>
						<GitFork size={ICON} class="flex-shrink-0" />
						<span class="truncate">{scopeName}</span>
						{@render envBadgeMark()}
					</span>
				{:else}
					{@render envBadgeMark()}
				{/if}
			</svelte:element>
		</div>
		<Menubar>
			{#snippet children({ createMenu })}
				<Menu
					bind:this={workspaceMenu}
					{createMenu}
					usePointerDownOutside
					placement="bottom-start"
					contentStyle="margin-left: {-nameWidth}px"
				>
					{#snippet triggr({ trigger })}
						<NavBreadcrumbTrigger
							{trigger}
							class="flex items-center p-1.5 rounded text-tertiary hover:bg-surface-hover hover:text-primary transition-colors"
							title="Switch workspace"
						>
							<ChevronDown size={ICON} class="flex-shrink-0" />
						</NavBreadcrumbTrigger>
					{/snippet}
					{#snippet children({ item: menuItem })}
						<WorkspacePickerBody item={menuItem} closeMenu={() => workspaceMenu?.close()} />
					{/snippet}
				</Menu>
			{/snippet}
		</Menubar>
	</div>

	{#if item || section || routePage}
		{@render slash()}
	{/if}

	{#if item}
		{#if item.path}
			<!-- The path whole, as one label rather than a row of pickers: it is what a person reads
			     to know which item this is, and what they reach for to paste somewhere else. -->
			<div class="relative flex items-center min-w-0">
				<!-- No hover fill: a fill offers to take you somewhere, and this only copies. The
				     pointer and the popup after the click are affordance enough. -->
				<button
					class={twMerge(SEGMENT, 'gap-1.5 hover:bg-transparent')}
					title="Copy path"
					onclick={copyPath}
					aria-label="Copy path {item.path}"
				>
					{#if scopeKind === 'u'}
						{@render userIcon()}
					{:else}
						{@render folderIcon()}
					{/if}
					{#each pathLevels as level, i (i)}
						{#if i > 0}{@render slash()}{/if}
						<span class="truncate">{level}</span>
					{/each}
				</button>
				{#if afterName}{@render afterName()}{/if}
				{#if pathCopied}
					<span
						class="absolute left-0 top-full mt-1 z-[6000] rounded-md border bg-surface px-2 py-1 text-2xs text-secondary shadow-md whitespace-nowrap"
						role="status"
					>
						Path copied successfully
					</span>
				{/if}
			</div>
		{:else}
			<span class="{SEGMENT} hover:bg-transparent" aria-current="page">
				{@render itemKindIcon()}
				<span class="truncate">{KIND_LABEL_LOWER[item.kind]}</span>
			</span>
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
		{#if afterName}{@render afterName()}{/if}
	{:else if routePage}
		{@const RouteIcon = routePage.icon}
		<!-- shrink-0: a page's own name is short and is the one part of the trail worth keeping
		     whole, so the workspace and fork names give way first when the bar is full. -->
		<span class="{SEGMENT} shrink-0 hover:bg-transparent hover:text-primary" aria-current="page">
			{#if RouteIcon}<RouteIcon size={ICON} class="flex-shrink-0 text-tertiary" />{/if}
			<span class="truncate">{routePage.label}</span>
		</span>
		{#if afterName}{@render afterName()}{/if}
	{/if}
</nav>
