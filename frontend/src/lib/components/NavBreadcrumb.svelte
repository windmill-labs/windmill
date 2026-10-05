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
	import MeltButton from '$lib/components/meltComponents/MeltButton.svelte'
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
		actingWorkspaceId,
		narrow = false,
		nameOnly = false
	}: {
		item?: PageHeaderItem
		section?: PageHeaderSection
		/** What sits beside the page's name: a mark like a documentation tooltip, or a control
		 *  that acts on the thing named. */
		afterName?: Snippet
		/** The workspace the page acts on, when it differs from the one the app is pointed at. */
		actingWorkspaceId?: string
		/** The bar is short of room: the workspace part drops its names and keeps its marks. */
		narrow?: boolean
		/** Only the page's own name, with no workspace part and no picker — the band inside a
		 *  session's preview frame, where the workspace is the host's and leading the reader out of
		 *  it is exactly what that band must not offer. The page still has to say what it is. */
		nameOnly?: boolean
	} = $props()

	const scopeId = $derived(actingWorkspaceId ?? $workspaceStore ?? undefined)
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
	const showFork = $derived(inFork)
	const familyName = $derived(family?.name ?? scopeId ?? '')
	// The workspace the user actually stands in, which is the fork's own name inside a fork.
	const scopeName = $derived(currentWs?.name ?? familyName)

	// A fork wears its workspace colour and, for a dev workspace, its environment badge — the same
	// two marks the fork chip carries everywhere else.
	const forkAccent = $derived(inFork ? forkAccentStyle(currentWs?.color) : undefined)
	// A dev workspace wears its environment; the root wears "prod", so which environment the user
	// is standing in is answered the same way whichever one it is.
	const envBadge = $derived.by(() => {
		if (!inFork) return { text: 'prod', dev: false }
		if (currentWs?.is_dev_workspace)
			return { text: devBadgeText(currentWs.dev_workspace_label), dev: true }
		return undefined
	})

	// No hover fill: the chevron beside it is what reacts to the pointer. The accent stays on the
	// fork's own name inside, so the family's name is not painted as if it were the fork.
	const scopeChipClass = twMerge(SEGMENT, 'hover:bg-transparent')

	const forkChipClass = $derived(
		twMerge(
			'flex items-center gap-1 min-w-0 px-1 rounded',
			forkAccent &&
				'bg-[color:var(--fork-accent-bg)] dark:bg-[color:var(--fork-accent-bg-dark)] text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)] font-semibold'
		)
	)

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
	<!-- No kind, no glyph: WorkspaceItemKindIcon falls through to the script icon, which would
	     label an agent a script. -->
	{#if item?.kind}<WorkspaceItemKindIcon kind={item.kind} size={ICON} />{/if}
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

{#snippet slash(extra = '')}
	<!-- `extra` tops the workspace part's own 4px gap up to the 6px the breadcrumb puts around its
	     other separators, so the slash before a fork is spaced like every other slash on the line. -->
	<span class={twMerge('shrink-0 text-hint/40 text-xs', extra)} aria-hidden="true">/</span>
{/snippet}

<nav
	aria-label="Breadcrumb"
	class="flex items-center gap-1.5 min-w-0 text-xs font-normal text-primary"
>
	{#if !nameOnly}
		<!-- One part for where the user is: the name of the workspace they are standing in — a fork
	     included, in its own colour — leading home, and one chevron whose picker changes it. The
	     picker lists the families and expands one to reach its forks, so a fork needs no part of
	     its own here. -->
		<div class={narrow ? 'flex items-center shrink-0' : 'flex items-center min-w-0'}>
			<!-- No hover fill on the name: the chevron beside it is the thing that lights up, and two
		     boxes reacting to one pass of the pointer read as two controls fighting. -->
			<!-- bind:clientWidth: the menu hangs from the chevron, so it is shifted back by the width of
		     the name to line its left edge up with the disc. -->
			<div class="flex items-center min-w-0" bind:clientWidth={nameWidth}>
				<svelte:element
					this={'a'}
					href={homeHref}
					class={scopeChipClass}
					title={showFork ? `${familyName} / ${scopeName}` : familyName}
				>
					{#if narrow}
						<!-- Short of room, the part keeps what it cannot be read without: the disc in the
					     workspace's own colour, the fork mark, and the environment. The names go — the
					     hover title still carries them, and the picker beside it names them all. -->
						{@render workspaceDisc()}
						{#if showFork}
							<span class={forkChipClass} style={forkAccent}>
								<GitFork size={ICON} class="flex-shrink-0" />
								{@render envBadgeMark()}
							</span>
						{:else}
							{@render envBadgeMark()}
						{/if}
					{:else}
						<BreadcrumbItemContent label={familyName} icon={workspaceDisc} />
						{#if showFork}
							<!-- Both names: the fork is where the work happens, the family is which product it
					     is part of, and either alone leaves the other to be guessed. -->
							{@render slash('mx-0.5')}
							<span class={forkChipClass} style={forkAccent}>
								<GitFork size={ICON} class="flex-shrink-0" />
								<span class="truncate">{scopeName}</span>
								{@render envBadgeMark()}
							</span>
						{:else}
							{@render envBadgeMark()}
						{/if}
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
							<MeltButton
								meltElement={trigger}
								class="flex items-center p-1.5 rounded text-tertiary hover:bg-surface-hover hover:text-primary transition-colors"
								title="Switch workspace"
							>
								<ChevronDown size={ICON} class="flex-shrink-0" />
							</MeltButton>
						{/snippet}
						{#snippet children({ item: menuItem })}
							<WorkspacePickerBody item={menuItem} closeMenu={() => workspaceMenu?.close()} />
						{/snippet}
					</Menu>
				{/snippet}
			</Menubar>
		</div>
	{/if}

	{#if !nameOnly && (item || section || routePage)}
		{@render slash()}
	{/if}

	{#if item}
		{#if item.path}
			<!-- The path whole, as one label rather than a row of pickers: it is what a person reads
			     to know which item this is, and what they reach for to paste somewhere else. -->
			<div class="relative flex items-center min-w-0">
				<!-- No hover fill: a fill offers to take you somewhere, and this only copies. The
				     pointer and the popup after the click are affordance enough. -->
				<!-- Capped: a draft's path is a 40-character uuid slug, and the trail yields so
				     grudgingly that one would push a page's buttons off the end of the bar. Past the
				     cap it truncates; the click still copies the path whole. -->
				<button
					class={twMerge(
						SEGMENT,
						'gap-1.5 hover:bg-transparent overflow-hidden',
						narrow ? 'max-w-[11rem]' : 'max-w-[16rem]'
					)}
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
						<!-- The last level absorbs the squeeze: the scopes above it are a couple of
						     characters each, and cutting them first leaves "m… / draft_a5b1…". -->
						<span
							class={i === pathLevels.length - 1 ? 'truncate' : 'truncate shrink-0 max-w-[8rem]'}
							>{level}</span
						>
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
		{:else if item.kind}
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
