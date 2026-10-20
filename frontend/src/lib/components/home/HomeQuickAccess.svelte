<script lang="ts">
	import { Copy, Eye, GitFork, List, Pen, Star as StarIcon, StarOff, X } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { userStore } from '$lib/stores'
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
	import { recentItems, type RecentKind } from './recentItems.svelte'
	import HomeSectionLabel from './HomeSectionLabel.svelte'

	interface Props {
		workspace: string
		/** Cleared where direct-deploy protection hides the rows' edit actions. */
		showEditButtons?: boolean
	}

	let { workspace, showEditButtons = true }: Props = $props()

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
	// Three rows of the three-column grid.
	const COLLAPSED_COUNT = 9

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
		const operator = !!$userStore?.operator
		const kind = await editorKind(item)
		const base = kind === 'raw_app' ? 'apps_raw' : `${kind}s`
		return [
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
				hide: !showEditButtons || operator || kind === 'agent'
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
				hide: operator
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
	let entries: { item: Entry; favorite: boolean }[] = $derived.by(() => {
		const starred = new Set(favorites.map((f) => `${asStarKind(f.kind)}/${f.path}`))
		const recent = recentItems
			.list(workspace)
			.filter((r) => !starred.has(`${asStarKind(r.kind)}/${r.path}`))
			.slice(0, RECENT_COUNT)
			.map((r) => ({
				kind: r.kind,
				starKind: asStarKind(r.kind),
				path: r.path,
				title: r.summary || r.path,
				at: r.at
			}))
		return [
			...favorites.map((item) => ({ item, favorite: true })),
			...recent.map((item) => ({ item, favorite: false }))
		]
	})

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

	let expanded = $state(false)
	let shown = $derived(expanded ? entries : entries.slice(0, COLLAPSED_COUNT))
</script>

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
			<span class="text-xs font-medium text-emphasis truncate">{item.title}</span>
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

{#if entries.length > 0}
	<section class="flex flex-col gap-2 px-4 sm:px-0 pb-4">
		<HomeSectionLabel class="-mb-1">Favorites & recent</HomeSectionLabel>
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
			{#each shown as { item, favorite } (item.kind + '/' + item.path)}
				{@render card(item, favorite)}
			{/each}
		</div>
		{#if entries.length > COLLAPSED_COUNT}
			<div class="self-start">
				<Button variant="subtle" unifiedSize="xs" onClick={() => (expanded = !expanded)}>
					{expanded ? 'Show less' : `Show all ${entries.length}`}
				</Button>
			</div>
		{/if}
	</section>
{/if}
