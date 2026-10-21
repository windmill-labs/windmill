<script lang="ts">
	import {
		AtSign,
		Check,
		ChevronLeft,
		ChevronRight,
		Copy,
		Eye,
		EyeOff,
		GitFork,
		LayoutGrid,
		List,
		Pen,
		Star as StarIcon,
		StarOff,
		X
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { roleCapabilities } from '$lib/stores'
	import { AppService } from '$lib/gen'
	import { copyToClipboard, type Item } from '$lib/utils'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import Star from '$lib/components/Star.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import {
		favoriteManager,
		getFavoriteHref,
		type FavoriteKind
	} from '$lib/components/sidebar/FavoriteMenu.svelte'
	import { MAX_AGE_DAYS, recentItems, type RecentKind } from './recentItems.svelte'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import {
		homeComposerContext,
		itemContextElement
	} from '$lib/components/sessions/homeComposerContext.svelte'
	import HomeSectionLabel from './HomeSectionLabel.svelte'
	import HomeSectionCount from './HomeSectionCount.svelte'
	import { QUICK_ACCESS_ROWS, homeQuickAccessSettings } from './homeQuickAccessSettings.svelte'

	interface Props {
		workspace: string
		/** Cleared where direct-deploy protection hides the rows' edit actions. */
		showEditButtons?: boolean
		/** For the items card's heading, which shows only under this section: alone on the
		 * page, a heading has nothing to tell apart. */
		itemCount?: number
	}

	let { workspace, showEditButtons = true, itemCount }: Props = $props()

	type Entry = {
		kind: RecentKind
		/** The kind its favorite is stored under: every app, raw ones included, stars as `app`
		 * (AppRow, and the app listing's starred join), so a raw app's must not be `raw_app`. */
		starKind: FavoriteKind
		path: string
		title: string
		at?: number
	}

	function asStarKind(kind: RecentKind): FavoriteKind {
		return kind === 'raw_app' ? 'app' : kind
	}

	const RECENT_COUNT = 6

	// The way back from Hide from Home is in the header menu HomeAIChat adds, which Home mounts
	// only with the AI sessions beta on. Without it the section can't be hidden, and one hidden
	// earlier shows again.
	const restoreMenuOnPage = isGlobalAiEnabled()
	let hidden = $derived(restoreMenuOnPage && homeQuickAccessSettings.hidden)

	// Turning off the last list shown would leave an empty section; Hide from Home does that.
	const sectionMenu = (): Item[] => [
		{
			displayName: 'Show favorites',
			description: 'Items you starred',
			toggle: homeQuickAccessSettings.showFavorites,
			disabled: homeQuickAccessSettings.showFavorites && !homeQuickAccessSettings.showRecent,
			action: () => homeQuickAccessSettings.setShowFavorites(!homeQuickAccessSettings.showFavorites)
		},
		{
			displayName: 'Show recent',
			description: `Scripts, flows and apps you opened in their editor in the last ${MAX_AGE_DAYS} days, in this browser`,
			toggle: homeQuickAccessSettings.showRecent,
			disabled: homeQuickAccessSettings.showRecent && !homeQuickAccessSettings.showFavorites,
			action: () => homeQuickAccessSettings.setShowRecent(!homeQuickAccessSettings.showRecent)
		},
		{
			displayName: 'Grid size',
			separatorTop: true,
			icon: LayoutGrid,
			submenuItems: QUICK_ACCESS_ROWS.map((rows) => ({
				displayName: rows === 1 ? '1 row' : `${rows} rows`,
				icon: homeQuickAccessSettings.rows === rows ? Check : undefined,
				action: () => homeQuickAccessSettings.setRows(rows)
			}))
		},
		{
			displayName: 'Hide from Home',
			icon: EyeOff,
			hide: !restoreMenuOnPage,
			action: () => homeQuickAccessSettings.setHidden(true)
		}
	]

	// Only the role is checked: the cards don't carry the item's own permissions, and an
	// editor opened without write access says so itself.
	const editRights = useEditRights()

	// A favorite app arrives as plain `app` (the layout lists apps without their subtype), and
	// `/apps/edit` mounts the low-code editor whatever it opens, so the editor routes ask which
	// one it is, once, when its menu opens.
	async function editorKind(item: Entry): Promise<RecentKind> {
		if (item.kind !== 'app') return item.kind
		try {
			const app: any = await AppService.getAppLiteByPath({ workspace, path: item.path })
			return app?.raw_app ? 'raw_app' : 'app'
		} catch {
			return 'app'
		}
	}

	// The row menus' actions that need nothing but the kind and path. Moving, deleting,
	// deploying and sharing need the full item and the row's own drawers, so they stay there.
	async function menuItems(item: Entry, favorite: boolean): Promise<Item[]> {
		const kind = await editorKind(item)
		const base = kind === 'raw_app' ? 'apps_raw' : `${kind}s`
		const contextElement = itemContextElement({
			kind: kind === 'raw_app' ? 'app' : kind,
			rawApp: kind === 'raw_app',
			path: item.path,
			summary: item.title === item.path ? undefined : item.title
		})
		return [
			{
				displayName: 'Add to AI session context',
				icon: AtSign,
				type: 'ai',
				action: () => contextElement && homeComposerContext.attach(contextElement),
				hide: !contextElement || !homeComposerContext.available
			},
			{
				displayName: 'Edit',
				icon: Pen,
				href: `/${base}/edit/${item.path}`,
				hide: !showEditButtons || !editRights.roleCanAuthor(kind)
			},
			{
				displayName: 'Duplicate/Fork',
				icon: GitFork,
				href: `/${base}/add?template=${item.path}`,
				hide: !showEditButtons || !$roleCapabilities.code || kind === 'agent'
			},
			{
				displayName: 'View runs',
				icon: List,
				// An agent run is a flow preview, which the runs page hides by default.
				href: `/runs/${item.path}${item.kind === 'agent' ? '?job_kinds=previews' : ''}`,
				hide: item.kind === 'app' || item.kind === 'raw_app'
			},
			{
				displayName: 'Audit logs',
				icon: Eye,
				href: `/audit_logs?resource=${item.path}`,
				hide: !$roleCapabilities.code || !$roleCapabilities.visibility.audit_logs
			},
			{ displayName: 'Copy path', icon: Copy, action: () => copyToClipboard(item.path) },
			{
				displayName: favorite ? 'Remove from favorites' : 'Add to favorites',
				icon: favorite ? StarOff : StarIcon,
				separatorTop: true,
				action: () =>
					favorite
						? favoriteManager.unstar(item.path, item.starKind, workspace)
						: favoriteManager.star(item.path, item.starKind, workspace, item.title)
			},
			{
				displayName: 'Remove from recent',
				icon: X,
				action: () => recentItems.forget(workspace, item.kind, item.path),
				hide: favorite
			}
		]
	}

	// Assets are favorites too, but they open in the database manager rather than at a page,
	// so they stay in the sidebar's menu.
	let favorites: Entry[] = $derived(
		favoriteManager.current.flatMap((f) =>
			f.kind === 'asset' ? [] : [{ kind: f.kind, starKind: f.kind, path: f.path, title: f.label }]
		)
	)
	// Favorites first, then what was edited lately; a starred item is listed once, as a favorite.
	// With the favorites turned off, a starred item edited lately shows among the recent ones.
	let recentAll: Entry[] = $derived.by(() => {
		const { showFavorites, showRecent } = homeQuickAccessSettings
		const starred = new Set(
			showFavorites ? favorites.map((f) => `${asStarKind(f.kind)}/${f.path}`) : []
		)
		return (showRecent ? recentItems.list(workspace) : [])
			.filter((r) => !starred.has(`${asStarKind(r.kind)}/${r.path}`))
			.map((r) => ({
				kind: r.kind,
				starKind: asStarKind(r.kind),
				path: r.path,
				title: r.summary || r.path,
				at: r.at
			}))
	})
	// Grows by a page from the "Load next" card; back to one page on the next visit.
	let recentLimit = $state(RECENT_COUNT)
	let recentMore = $derived(Math.max(0, recentAll.length - recentLimit))
	let entries: { item: Entry; favorite: boolean }[] = $derived([
		...(homeQuickAccessSettings.showFavorites ? favorites : []).map((item) => ({
			item,
			favorite: true
		})),
		...recentAll.slice(0, recentLimit).map((item) => ({ item, favorite: false }))
	])

	// The list turned off has items, the one left on has none. The section stays, or its menu
	// (the only way to turn the list back on) would leave with it.
	let emptiedBySettings = $derived(
		entries.length === 0 &&
			((!homeQuickAccessSettings.showFavorites && favorites.length > 0) ||
				(!homeQuickAccessSettings.showRecent && recentItems.list(workspace).length > 0))
	)

	// Read when the pointer reaches the time, so the tooltip is current however long the
	// page has been open.
	let now = $state(Date.now())

	function editedTooltip(at: number): string {
		const mins = Math.max(0, Math.floor((now - at) / 60_000))
		const unit = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'} ago`
		const ago =
			mins < 1
				? 'just now'
				: mins < 60
					? unit(mins, 'minute')
					: mins < 1440
						? unit(Math.floor(mins / 60), 'hour')
						: unit(Math.floor(mins / 1440), 'day')
		return `Edited by you ${ago}`
	}

	// Everything the lists hold, the recent ones not loaded yet included.
	let favoriteCount = $derived(homeQuickAccessSettings.showFavorites ? favorites.length : 0)
	let recentCount = $derived(recentAll.length)

	// The cards fill one screen at a time, row by row, and the next screen scrolls in from the
	// right. Placed by hand: the grid's own flow fills either whole rows across every screen or
	// whole columns. The column counts follow the `auto-cols` breakpoints below.
	let innerWidth = $state(1280)
	let cols = $derived(innerWidth >= 1024 ? 3 : innerWidth >= 640 ? 2 : 1)
	let cellCount = $derived(entries.length + (recentMore > 0 ? 1 : 0))
	// Fewer rows when there aren't enough cards to fill a screen.
	let rows = $derived(Math.min(homeQuickAccessSettings.rows, Math.ceil(cellCount / cols)))

	function cellPosition(index: number): { row: number; column: number } {
		const perScreen = cols * rows
		const screen = Math.floor(index / perScreen)
		const within = index % perScreen
		return { row: Math.floor(within / cols) + 1, column: screen * cols + (within % cols) + 1 }
	}

	let scroller: HTMLDivElement | undefined = $state(undefined)
	let canBack = $state(false)
	let canForward = $state(false)

	function measure() {
		if (!scroller) return
		canBack = scroller.scrollLeft > 1
		canForward = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 1
	}

	// No scroll snap: when the favorites load in front of the recent cards, a snapped scroller
	// follows the card it had snapped to and opens scrolled to the end. One page (the width
	// inside the scroller's 1px padding) plus the grid's 8px gap lands on a column edge by itself.
	// Animated by hand: `behavior: 'smooth'` runs at the browser's own pace, which is slow for a
	// page this wide. A click during the animation moves on from where the last one was going.
	const PAGE_MS = 200
	const reducedMotion = useReducedMotion()
	let scrollTarget: number | undefined
	let scrollFrame: number | undefined

	function page(direction: 1 | -1) {
		const el = scroller
		if (!el) return
		const max = el.scrollWidth - el.clientWidth
		const from = el.scrollLeft
		const to = Math.max(
			0,
			Math.min(max, (scrollTarget ?? from) + direction * (el.clientWidth - 2 + 8))
		)
		if (scrollFrame != undefined) cancelAnimationFrame(scrollFrame)
		if (reducedMotion.val) {
			el.scrollLeft = to
			return
		}
		scrollTarget = to
		const start = performance.now()
		const step = (now: number) => {
			const t = Math.min(1, (now - start) / PAGE_MS)
			el.scrollLeft = from + (to - from) * (1 - Math.pow(1 - t, 3))
			if (t < 1) {
				scrollFrame = requestAnimationFrame(step)
			} else {
				scrollFrame = undefined
				scrollTarget = undefined
			}
		}
		scrollFrame = requestAnimationFrame(step)
	}

	// A resize changes what fits, and so does a change of cards or rows, which can leave the
	// scroller's own box the same size.
	function watchOverflow(node: HTMLElement) {
		void cellCount
		void rows
		measure()
		const observer = new ResizeObserver(measure)
		observer.observe(node)
		return () => observer.disconnect()
	}
</script>

<svelte:window bind:innerWidth />

{#snippet card(item: Entry, favorite: boolean)}
	<div
		class="group flex items-center gap-1 rounded-md border border-light bg-surface-tertiary hover:bg-surface-hover transition-colors min-w-0"
	>
		<a
			href={getFavoriteHref(item.path, item.kind)}
			class="flex items-center gap-2 min-w-0 pl-2.5 py-2"
			title={item.title === item.path ? item.path : `${item.title}\n${item.path}`}
		>
			<span class="shrink-0"><RowIcon kind={item.kind} size={14} /></span>
			<span class="text-2xs font-normal text-primary truncate">{item.title}</span>
		</a>
		<!-- The star sits after the title, as on the tree's rows: a set one stays, an unset one
		     shows on hover or focus. -->
		<Star
			kind={item.starKind}
			path={item.path}
			workspaceId={workspace}
			summary={item.title}
			size={13}
			class="inline-flex shrink-0 p-px text-yellow-500 opacity-0 data-[starred]:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100"
		/>
		<!-- A button can't sit inside the link, so the rest of the card is a second, untabbable
		     link to the same page. -->
		<a
			href={getFavoriteHref(item.path, item.kind)}
			class="flex-1 self-stretch"
			tabindex="-1"
			aria-hidden="true"
		></a>
		{#if !favorite && item.at}
			<span
				class="shrink-0 text-2xs text-secondary"
				title={editedTooltip(item.at)}
				onmouseenter={() => (now = Date.now())}
				role="note"
			>
				<TimeAgo date={new Date(item.at).toISOString()} compact />
			</span>
		{/if}
		<div class="shrink-0 pr-1 text-secondary">
			<DropdownV2 items={() => menuItems(item, favorite)} size="xs" fixedHeight={false} />
		</div>
	</div>
{/snippet}

{#snippet arrow(direction: 1 | -1)}
	<!-- In the page's side padding, beside the cards rather than over them, so the cards keep
	     the items card's width. A touch screen swipes instead. -->
	<div
		class="absolute inset-y-0 hidden sm:block [@media(hover:none)]:hidden {direction === -1
			? 'right-full mr-1'
			: 'left-full ml-1'}"
	>
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			wrapperClasses="h-full"
			btnClasses="!h-full !w-6 !px-0"
			startIcon={{ icon: direction === -1 ? ChevronLeft : ChevronRight }}
			title={direction === -1 ? 'Scroll left' : 'Scroll right'}
			aria-label={direction === -1 ? 'Scroll left' : 'Scroll right'}
			onClick={() => page(direction)}
		/>
	</div>
{/snippet}

<!-- Held back until the favorites are in: the recent cards are ready first, and the favorites
     landing in front of them would push every card over. -->
{#if favoriteManager.loadedFor === workspace && (entries.length > 0 || emptiedBySettings) && !hidden}
	<!-- 30px under the cards: the gap from the composer down to this section's heading text. -->
	<section class="flex flex-col gap-2 px-4 sm:px-0 pb-[30px]">
		<div class="flex items-center gap-1 -my-1">
			<HomeSectionLabel>
				{#if favoriteCount > 0}
					Favorites <HomeSectionCount count={favoriteCount} />
				{/if}
				{#if favoriteCount > 0 && recentCount > 0}
					<span class="text-hint">·</span>
				{/if}
				{#if recentCount > 0}
					Recent <HomeSectionCount count={recentCount} />
				{/if}
			</HomeSectionLabel>
			<div class="text-secondary">
				<DropdownV2 items={sectionMenu} size="xs" fixedHeight={false} />
			</div>
		</div>
		{#if entries.length === 0}
			<p class="text-2xs text-hint">
				{homeQuickAccessSettings.showFavorites
					? 'No favorites yet. Star a script, flow or app to keep it here.'
					: 'Nothing opened lately in this browser.'}
			</p>
		{/if}
		<div class="relative">
			<!-- The scrollbar is hidden, so a trackpad, shift-wheel or a swipe scrolls it and the
			     arrows stand in for the bar. The 1px padding, taken back by the margin, keeps the
			     cards' borders off the clip edge: a third of the width is rarely a whole pixel, and
			     the edge then cuts the outer border. -->
			<div
				bind:this={scroller}
				{@attach watchOverflow}
				onscroll={measure}
				class="grid gap-2 overflow-x-auto no-scrollbar px-px -mx-px auto-cols-[85%] sm:auto-cols-[calc((100%_-_0.5rem)/2)] lg:auto-cols-[calc((100%_-_1rem)/3)]"
				style:grid-template-rows="repeat({rows}, auto)"
			>
				{#each entries as { item, favorite }, i (item.kind + '/' + item.path)}
					{@const at = cellPosition(i)}
					<div class="min-w-0" style:grid-row={at.row} style:grid-column={at.column}>
						{@render card(item, favorite)}
					</div>
				{/each}
				{#if recentMore > 0}
					{@const next = Math.min(recentMore, RECENT_COUNT)}
					{@const at = cellPosition(entries.length)}
					<div class="min-w-0" style:grid-row={at.row} style:grid-column={at.column}>
						<Button
							variant="subtle"
							unifiedSize="sm"
							wrapperClasses="h-full min-w-0"
							btnClasses="!h-full rounded-md border border-dashed border-border-light !text-2xs text-secondary font-normal"
							onClick={() => (recentLimit += RECENT_COUNT)}
						>
							Load next {next}
							{next === 1 ? 'item' : 'items'}
						</Button>
					</div>
				{/if}
			</div>
			{#if canBack}
				{@render arrow(-1)}
			{/if}
			{#if canForward}
				{@render arrow(1)}
			{/if}
		</div>
	</section>
	<HomeSectionLabel class="px-4 sm:px-0 pb-1">
		Workspace items
		{#if itemCount != undefined}
			<HomeSectionCount count={itemCount} />
		{/if}
	</HomeSectionLabel>
{/if}
