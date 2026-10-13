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
	import WorkspacePickerBody from '$lib/components/sidebar/WorkspacePickerBody.svelte'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import { userWorkspaces, workspaceStore, workspaceColor, superadmin } from '$lib/stores'
	import { findWorkspaceRoot } from '$lib/utils/workspaceHierarchy'
	import { useForkableWorkspaces } from '$lib/utils/useForkableWorkspaces.svelte'
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
	import {
		BAR_STAGES,
		FLOOR,
		shownPathLevels,
		type Part,
		type TrailMeasures,
		type TrailStage
	} from './pageHeaderStages'

	let {
		item,
		section,
		afterName,
		actingWorkspaceId,
		nameOnly = false,
		stage = BAR_STAGES[0],
		caps = {},
		scopesGive = false,
		measures = $bindable()
	}: {
		item?: PageHeaderItem
		section?: PageHeaderSection
		/** What sits beside the page's name: a mark like a documentation tooltip, or a control
		 *  that acts on the thing named. */
		afterName?: Snippet
		/** The workspace the page acts on, when it differs from the one the app is pointed at. */
		actingWorkspaceId?: string
		/** Which parts the bar has dropped for room (see pageHeaderStages). */
		stage?: TrailStage
		/** Max widths the bar holds the names to, once dropping whole parts is not enough. */
		caps?: Partial<Record<Part, number>>
		/** The folders above the name truncate too: even the name at its last floor does not fit. */
		scopesGive?: boolean
		/** Filled in for the bar to choose a stage with; left undefined, nothing is measured. */
		measures?: TrailMeasures
		/** Only the page's own name, with no workspace part and no picker — the band inside a
		 *  session's preview frame, where the workspace is the host's and leading the reader out of
		 *  it is exactly what that band must not offer. The page still has to say what it is. */
		nameOnly?: boolean
	} = $props()

	const scopeId = $derived(actingWorkspaceId ?? $workspaceStore ?? undefined)
	const homeHref = $derived(
		actingWorkspaceId ? `/?workspace=${encodeURIComponent(actingWorkspaceId)}` : `/`
	)

	// With no item or section registered, the route still says where the user is: Runs, Variables,
	// and the rest name themselves in the breadcrumb rather than leaving it at the workspace.
	// Home is the root the workspace part already stands for, so it names no page of its own.
	const routePage = $derived.by(() => {
		if (item || section) return undefined
		const found = navPageFor(page.url.pathname)
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

	const pathSegment = $derived(
		twMerge(SEGMENT, 'gap-1.5 overflow-hidden', itemIsLast ? LAST : LEAD)
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

	// The off-screen copies stand in a fixed-width box for what a page renders into the trail: a
	// second render of a page's snippet would be a second live copy of its controls.
	let afterNameWidth = $state(0)
	let sectionContentWidth = $state(0)

	/** A measured copy holds each name at its floor; the trail on screen at the bar's cap. */
	function capStyle(part: Part, measure: boolean): string | undefined {
		if (measure) return `max-width: ${FLOOR[part]}px`
		const cap = caps[part]
		return cap === undefined ? undefined : `max-width: ${cap}px`
	}
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

{#snippet workspaceMenu_(measure: boolean)}
	{#if measure}
		<span class="flex items-center h-9 px-1.5 shrink-0">
			<ChevronDown size={ICON} class="flex-shrink-0" />
		</span>
	{:else}
		<Menubar>
			{#snippet children({ createMenu })}
				<Menu
					bind:this={workspaceMenu}
					{createMenu}
					usePointerDownOutside
					placement="bottom-start"
					class="h-auto"
					contentStyle="margin-left: {-nameWidth}px"
				>
					{#snippet triggr({ trigger })}
						<MeltButton
							meltElement={trigger}
							class="flex items-center h-9 px-1.5 rounded text-tertiary hover:bg-surface-hover hover:text-primary transition-colors"
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
	{/if}
{/snippet}

{#snippet trail(f: TrailStage, measure: boolean)}
	{@const scopeTitle = showFork ? `${familyName} / ${scopeName}` : familyName}
	{#if !nameOnly}
		<!-- One part for where the user is: the name of the workspace they are standing in — a fork
		     included, in its own colour — leading home, and one chevron whose picker changes it. The
		     picker lists the families and expands one to reach its forks, so a fork needs no part of
		     its own here. Its parts never shrink on their own: the bar drops them whole and caps the
		     names left, in the order pageHeaderStages sets. -->
		<div class="flex items-center shrink-0">
			<!-- No hover fill on the name: the chevron beside it is the thing that lights up, and two
			     boxes reacting to one pass of the pointer read as two controls fighting. -->
			<!-- bind:clientWidth: the menu hangs from the chevron, so it is shifted back by the width of
			     the name to line its left edge up with the disc. -->
			<div
				class="flex items-center"
				bind:clientWidth={() => nameWidth, (w) => !measure && (nameWidth = w)}
			>
				<svelte:element
					this={measure ? 'span' : 'a'}
					href={measure ? undefined : homeHref}
					class={f.wsButton ? 'flex items-center gap-1 pl-1 py-0.5 rounded' : scopeChipClass}
					title={scopeTitle}
				>
					{#if f.wsButton}
						<!-- Down to its disc, the part still leads home: the fork it stands in rides on the
						     disc as a mark, and the environment stays beside it. -->
						<span class="relative flex shrink-0">
							{@render workspaceDisc()}
							{#if showFork}
								<span
									class={twMerge(
										'absolute -right-1 -bottom-0.5 size-3.5 rounded-full grid place-items-center ring-2 ring-surface',
										forkAccent
											? 'bg-[color:var(--fork-accent-badge)] text-white'
											: 'bg-surface-sunken text-primary'
									)}
									style={forkAccent}
								>
									<GitFork size={9} />
								</span>
							{/if}
						</span>
						{@render envBadgeMark()}
					{:else}
						<!-- The family's name goes before the fork's: the fork is where the work happens,
						     the family is which product it is part of, and of the two that is the one a
						     reader can do without. Outside a fork it is the workspace's own name and stays
						     until the part is down to its disc. -->
						<span class="flex items-center gap-1 shrink-0">
							{@render workspaceDisc()}
							{#if f.familyName || !showFork}
								<span class="truncate" style={capStyle('family', measure)}>{familyName}</span>
							{/if}
						</span>
						{#if showFork}
							{@render slash('mx-0.5')}
							<span class={twMerge(forkChipClass, 'shrink-0')} style={forkAccent}>
								<GitFork size={ICON} class="flex-shrink-0" />
								{#if f.forkName}
									<span class="truncate" style={capStyle('fork', measure)}>{scopeName}</span>
								{/if}
								{@render envBadgeMark()}
							</span>
						{:else}
							{@render envBadgeMark()}
						{/if}
					{/if}
				</svelte:element>
			</div>
			{@render workspaceMenu_(measure)}
		</div>
	{/if}

	<!-- The summary that follows is the item's name for a person, so on a short bar it is the path
	     that goes (see `TrailStage.path`). What the page hangs after the name stays. -->
	{@const pathShown = f.path || itemIsLast}
	{#if !nameOnly && (item ? pathShown : section || routePage)}
		{@render slash()}
	{/if}

	{#if item}
		{#if !pathShown}
			{@render afterNameSlot(measure)}
		{:else if item.path}
			{@const shown = shownPathLevels(pathLevels, f)}
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
					{#each shown as level, i (i)}
						{#if i > 0}{@render slash()}{/if}
						{#if i === shown.length - 1}
							<span class="truncate shrink-0" style={capStyle('name', measure)}>{level}</span>
						{:else if level === '…' && !f.middleLevels}
							<span class="shrink-0 text-tertiary">…</span>
						{:else}
							<!-- The folder or user gives width only once the name is down to a few
							     characters: it is short and says whose thing this is, while a name long
							     enough to need truncating still reads from its first words. -->
							<span
								class={twMerge(
									'truncate max-w-[8rem]',
									scopesGive && !measure ? 'min-w-0' : 'shrink-0'
								)}>{level}</span
							>
						{/if}
					{/each}
				{/snippet}
				{#if measure}
					<span class={pathSegment}>{@render pathLabel()}</span>
				{:else if item.pathTrigger}
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
				{@render afterNameSlot(measure)}
				{#if pathCopied && !measure}
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
		{#if measure}
			<span class={twMerge('inline-flex items-center px-1', SEGMENT, LAST)}>
				{@render sectionKindIcon()}<span>{KIND_LABEL_LOWER[section.kind]}</span>
			</span>
		{:else}
			<BreadcrumbSegment
				label={KIND_LABEL_LOWER[section.kind]}
				icon={sectionKindIcon}
				extraClass={twMerge(SEGMENT, LAST)}
				isCurrent
				initialHighlight={kindKey(section.kind)}
				initialScope={{ kind: section.kind }}
				onPick={() => {}}
			/>
		{/if}
	{:else if section}
		{#if section.parent}
			<!-- Gives width back with the workspace part, before the section's own name does. -->
			<span
				class={twMerge(SEGMENT, LEAD, 'shrink-[999] hover:bg-transparent hover:text-secondary')}
			>
				<span class="truncate">{section.parent}</span>
			</span>
			{@render slash()}
		{/if}
		{#if section.content && measure}
			<span class="shrink-0" style="width: {sectionContentWidth}px"></span>
		{:else}
			<!-- A section's own widget lays out as a row: its title, and whatever belongs with it. -->
			<span
				class={twMerge(
					SEGMENT,
					LAST,
					'hover:bg-transparent hover:text-emphasis',
					section.content ? '' : 'shrink-0'
				)}
				aria-current={measure ? undefined : 'page'}
				bind:clientWidth={
					() => sectionContentWidth, (w) => !measure && section.content && (sectionContentWidth = w)
				}
			>
				<!-- A plain label is the page's name, so it truncates the way an item's name does, in
				     its turn: a run's id, left to flex, is squeezed to nothing under the buttons. -->
				{#if section.content}{@render section.content()}{:else}<span
						class="truncate"
						style={capStyle('name', measure)}>{section.label}</span
					>{/if}
			</span>
		{/if}
		{@render afterNameSlot(measure)}
	{:else if routePage}
		{@const RouteIcon = routePage.icon}
		<!-- shrink-0: a page's own name is short and is the one part of the trail worth keeping
		     whole, so the workspace and fork names give way first when the bar is full. -->
		<span
			class={twMerge(SEGMENT, LAST, 'shrink-0 hover:bg-transparent hover:text-emphasis')}
			aria-current={measure ? undefined : 'page'}
		>
			{#if RouteIcon}<RouteIcon size={ICON} class="flex-shrink-0 text-tertiary" />{/if}
			<span class="truncate">{routePage.label}</span>
		</span>
		{@render afterNameSlot(measure)}
	{/if}
{/snippet}

{#snippet afterNameSlot(measure: boolean)}
	{#if afterName}
		{#if measure}
			<span class="shrink-0" style="width: {afterNameWidth}px"></span>
		{:else}
			<span class="flex items-center shrink-0" bind:clientWidth={afterNameWidth}>
				{@render afterName()}
			</span>
		{/if}
	{/if}
{/snippet}

<nav
	aria-label="Breadcrumb"
	class="flex items-center gap-1.5 min-w-0 text-xs font-normal text-primary"
>
	{@render trail(stage, false)}
</nav>

{#if measures}
	<!-- The trail at every stage, each name at its floor, for the bar to choose a stage with; and
	     each name whole, for it to cap them with. Same snippet, same classes, so the widths are the
	     ones the trail on screen would take. -->
	<div class="absolute -left-[99999px] top-0 invisible" aria-hidden="true" inert>
		{#each BAR_STAGES as s, i (i)}
			<nav
				class="flex items-center gap-1.5 w-max text-xs font-normal text-primary"
				bind:clientWidth={measures.trail[i]}
			>
				{@render trail(s, true)}
			</nav>
		{/each}
		<!-- Always rendered, empty where the page has no such part: a width of 0 is how the bar
		     knows the part is not there. -->
		<span
			class={twMerge(scopeChipClass, 'inline-block whitespace-nowrap p-0')}
			bind:clientWidth={measures.family}>{nameOnly ? '' : familyName}</span
		>
		<span
			class={twMerge(forkChipClass, 'inline-block whitespace-nowrap p-0')}
			bind:clientWidth={measures.fork}>{!nameOnly && showFork ? scopeName : ''}</span
		>
		<span
			class={twMerge('inline-block whitespace-nowrap text-xs', itemIsLast || !item ? LAST : LEAD)}
			bind:clientWidth={measures.name}
			>{item?.path
				? (pathLevels[pathLevels.length - 1] ?? '')
				: !item && section && !section.kind && !section.content
					? section.label
					: ''}</span
		>
	</div>
{/if}
