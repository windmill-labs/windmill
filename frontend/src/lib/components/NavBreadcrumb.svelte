<!--
@component
The page header's breadcrumb: workspace / fork / kind / path, in one flat line.

Every segment is a trigger for the picker that changes it — the workspace menu, the fork family
picker, and the item drill picker for the kind and each path level — and all of them share one
size and one icon size, so the line reads as one control rather than four.

Two weights, not one: the segment that ends the trail is what the page is about and carries the
weight, and everything leading to it is a step quieter. Which segment that is depends on the
route — the workspace on home, a list page's own name, an item's summary (which the band draws,
not this component) — so the lead can run the whole line.
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
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import DevWorkspaceBadge from './DevWorkspaceBadge.svelte'
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
		/** The bar is short of room: the path holds a tighter cap, so it cannot claim a third of the
		 *  line before anything else has given way. The names truncate at every width either way. */
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
	/** The trail up to the last segment: where the thing lives, which the reader scans past. */
	const LEAD = 'font-normal text-secondary'
	/** The last segment: what the page is about, and the only part of the line with weight. */
	const LAST = 'font-medium text-emphasis'
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
	/** Whether this family has a dev workspace anywhere under it. */
	const familyHasDev = $derived.by(() => {
		const all = $userWorkspaces ?? []
		const root = family?.id
		return (
			!!root && all.some((w) => w.is_dev_workspace && findWorkspaceRoot(w.id, all)?.id === root)
		)
	})
	const envBadge = $derived.by(() => {
		// "prod" only where a dev workspace exists to be the other half of the pair: a family with
		// one environment gains nothing from naming it.
		if (!inFork) return familyHasDev ? 'prod' : undefined
		if (currentWs?.is_dev_workspace) return 'dev'
		return undefined
	})

	// Which segment ends the trail, and so carries the weight. Everything before it is `LEAD`: on
	// home that is nothing and the workspace itself is the subject, on a list page the page's name
	// takes over, and on an item page the summary does — so the workspace, the fork and the path
	// all step back to make room for it.
	const namedAfterWorkspace = $derived(!!item || !!section || !!routePage)
	const workspaceIsLast = $derived(!nameOnly && !namedAfterWorkspace)
	// The summary is the band's, not the breadcrumb's (see PageHeaderBar), so the path reads as the
	// last segment only for an item that has no summary to follow it.
	const itemIsLast = $derived(!!item && !item.summaryContent && !item.summary)

	// No hover fill: the chevron beside it is what reacts to the pointer. The accent stays on the
	// fork's own name inside, so the family's name is not painted as if it were the fork.
	//
	// Inside a fork the family's name is the lead even where the workspace part ends the trail: the
	// workspace the user stands in is the fork, and the family before it says which product that
	// fork belongs to. Weighting both would name two workspaces as the subject.
	const scopeChipClass = $derived(
		twMerge(SEGMENT, 'hover:bg-transparent', workspaceIsLast && !showFork ? LAST : LEAD)
	)

	// Capped: a draft's path is a 40-character uuid slug, and the trail yields so grudgingly that
	// one would push a page's buttons off the end of the bar.
	const pathSegment = $derived(
		twMerge(
			SEGMENT,
			'gap-1.5 overflow-hidden',
			itemIsLast ? LAST : LEAD,
			narrow ? 'max-w-[11rem]' : 'max-w-[16rem]'
		)
	)
	// Copying is the one thing a segment does that leaves no mark on the page, so it is the one
	// without a hover fill: the pointer and the confirmation under it are the affordance. A segment
	// that opens the rename editor keeps the fill, which is what tells the two apart on sight.
	const copyPathSegment = $derived(twMerge(pathSegment, 'hover:bg-transparent'))

	// The chip keeps its colour wherever it is on the line — that colour is how a fork is told apart
	// from its family. Only its weight follows the trail, dropping back once a page's name ends it.
	const forkChipClass = $derived(
		twMerge(
			'flex items-center gap-1 min-w-0 px-1 rounded',
			// A filled chip needs room around what it holds; an uncolored one is just the name and
			// would only be pushed away from the slash before it.
			forkAccent &&
				'px-1.5 py-0.5 bg-[color:var(--fork-accent-bg)] dark:bg-[color:var(--fork-accent-bg-dark)] text-[color:var(--fork-accent-text)] dark:text-[color:var(--fork-accent-text-dark)]',
			// Where the workspace part ends the trail, the fork is the segment that ends it, so it
			// takes the weight on its own. Without a colour of its own it also has to take the text
			// colour, since it would otherwise inherit the lead's from the link around it.
			workspaceIsLast ? twMerge('font-semibold', !forkAccent && 'text-emphasis') : 'font-normal'
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
	{#if envBadge === 'dev'}
		<DevWorkspaceBadge
			label={currentWs?.dev_workspace_label}
			color={currentWs?.color}
			fallbackClass="dark:bg-surface-accent-primary text-white dark:text-white"
			class="text-3xs px-1 py-0"
		/>
	{:else if envBadge === 'prod'}
		<Badge color="gray" small class="text-3xs px-1 py-0">prod</Badge>
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
		<!-- `shrink-[999]`: the whole trail can give width back, and this part gives it first. Flex
		     takes it from the parts in proportion to what they ask for, so a factor this far above
		     the rest means the names here are eaten down to their icons before the page's own name
		     loses a character. -->
		<div class="flex items-center min-w-0 shrink-[999]">
			<!-- No hover fill on the name: the chevron beside it is the thing that lights up, and two
		     boxes reacting to one pass of the pointer read as two controls fighting. -->
			<!-- bind:clientWidth: the menu hangs from the chevron, so it is shifted back by the width of
		     the name to line its left edge up with the disc. -->
			<div class="flex items-center min-w-0" bind:clientWidth={nameWidth}>
				<a
					href={homeHref}
					class={scopeChipClass}
					title={showFork ? `${familyName} / ${scopeName}` : familyName}
				>
					<!-- The family's name goes before the fork's: the fork is where the work happens, the
					     family is which product it is part of, and of the two that is the one a reader can
					     do without. Both truncate to nothing rather than disappearing at a breakpoint —
					     what is left then is what the part cannot be read without, the disc in the
					     workspace's own colour, the fork mark and the environment. The hover title
					     carries the names whole, and the picker beside it names them all. -->
					<span class="flex items-center gap-1 min-w-0 shrink-[999]">
						<BreadcrumbItemContent label={familyName} icon={workspaceDisc} />
					</span>
					{#if showFork}
						{@render slash('mx-0.5')}
						<span class={forkChipClass} style={forkAccent}>
							<GitFork size={ICON} class="flex-shrink-0" />
							<span class="truncate">{scopeName}</span>
							{@render envBadgeMark()}
						</span>
					{:else}
						{@render envBadgeMark()}
					{/if}
				</a>
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
							<WorkspacePickerBody
									item={menuItem}
									collapseFamilies={actingWorkspaceId != undefined}
									closeMenu={() => workspaceMenu?.close()}
								/>
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
				<!-- Past the cap the trail truncates; whatever the click does still has the path whole. -->
				{#snippet pathLabel()}
					{#if scopeKind === 'u'}
						{@render userIcon()}
					{:else}
						{@render folderIcon()}
					{/if}
					{#each pathLevels as level, i (i)}
						{#if i > 0}{@render slash()}{/if}
						<!-- The scopes above the name absorb the squeeze, the name itself last: a path is
						     read from its end, and which folder a thing is in is answerable from the
						     picker while the thing's own name is not. -->
						<span
							class={i === pathLevels.length - 1
								? 'truncate'
								: 'truncate min-w-0 shrink-[999] max-w-[8rem]'}>{level}</span
						>
					{/each}
				{/snippet}
				{#if item.pathTrigger}
					<!-- The page hangs its own rename off this segment, anchored here so the editor
					     opens under the path rather than beside the summary. It wears the segment's
					     own classes, so a clickable path reads the same as one that only copies. -->
					{@render item.pathTrigger(pathLabel, pathSegment)}
				{:else}
					<button
						class={copyPathSegment}
						title="Copy path"
						onclick={copyPath}
						aria-label="Copy path {item.path}"
					>
						{@render pathLabel()}
					</button>
				{/if}
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
			<!-- The path is on its way: a page registers its kind as soon as it mounts and fills the
			     path once it has loaded one. Naming the kind here reads as the answer and then jumps
			     when the real path replaces it, so hold its place instead. The width is a plausible
			     path's, which is what keeps the trail from shifting when it lands. -->
			<span
				class={twMerge(SEGMENT, 'hover:bg-transparent', itemIsLast ? LAST : LEAD)}
				aria-label="Loading {KIND_LABEL_LOWER[item.kind]} path"
				aria-busy="true"
			>
				{@render itemKindIcon()}
				<Skeleton layout={[[{ h: 0.75, w: 100, minW: 96 }]]} class="w-24" />
			</span>
		{/if}
	{:else if section?.kind}
		<BreadcrumbSegment
			label={KIND_LABEL_LOWER[section.kind]}
			icon={sectionKindIcon}
			extraClass={twMerge(SEGMENT, LAST)}
			isCurrent
			initialHighlight={kindKey(section.kind)}
			initialScope={{ kind: section.kind }}
			onPick={() => {}}
		/>
	{:else if section}
		<!-- A section's own widget lays out as a row: its title, and whatever belongs with it. -->
		<span
			class={twMerge(
				SEGMENT,
				LAST,
				'hover:bg-transparent hover:text-emphasis',
				section.content ? '' : 'truncate'
			)}
			aria-current="page"
		>
			{#if section.content}{@render section.content()}{:else}{section.label}{/if}
		</span>
		{#if afterName}{@render afterName()}{/if}
	{:else if routePage}
		{@const RouteIcon = routePage.icon}
		<!-- shrink-0: a page's own name is short and is the one part of the trail worth keeping
		     whole, so the workspace and fork names give way first when the bar is full. -->
		<span
			class={twMerge(SEGMENT, LAST, 'shrink-0 hover:bg-transparent hover:text-emphasis')}
			aria-current="page"
		>
			{#if RouteIcon}<RouteIcon size={ICON} class="flex-shrink-0 text-tertiary" />{/if}
			<span class="truncate">{routePage.label}</span>
		</span>
		{#if afterName}{@render afterName()}{/if}
	{/if}
</nav>
