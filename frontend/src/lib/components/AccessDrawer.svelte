<script lang="ts">
	import { Alert, Badge, Button, Drawer, DrawerContent, Section, Skeleton } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import Row from './table/Row.svelte'
	import RowIcon from './common/table/RowIcon.svelte'
	import { Tooltip } from './meltComponents'
	import Popover from './meltComponents/Popover.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import { GranularAclService, type PrincipalAccess } from '$lib/gen'
	import { useOperatingWorkspace } from './operatingWorkspace.svelte'
	import { sendUserToast } from '$lib/toast'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { pluralize, type Item } from '$lib/utils'
	import { itemHref } from './offboarding-utils'
	import GrantNavigator, { navigatorMaxHeight } from './access/GrantNavigator.svelte'
	import { KIND_LABELS, setGrant, type GrantLevel } from './access/access'
	import {
		buildAccessTree,
		flattenAccessTree,
		type AccessItem,
		type AccessOwnerNode
	} from './access/accessTree'
	import {
		ChevronDown,
		ChevronRight,
		ChevronsDownUp,
		ChevronsUpDown,
		ExternalLink,
		Folder,
		FolderTree,
		Lock,
		Plus,
		User,
		Users
	} from 'lucide-svelte'

	const operatingWorkspace = useOperatingWorkspace()

	type Target = 'self' | 'user' | 'service_account' | 'group'

	const PAGE = 100
	// A short list opens fully, so every item shows at once. A longer one starts at its folders.
	const OPEN_UP_TO = 30

	let drawer: Drawer | undefined = $state()
	let principal = $state('')
	let access: PrincipalAccess | undefined = $state()
	let error: string | undefined = $state()
	let search = $state('')
	let kind: string | undefined = $state()
	let nbDisplayed = $state(PAGE)
	/** Rows opened or closed by hand, by path prefix. The rest follow `expandAll`. */
	let openState: Record<string, boolean> = $state({})
	let expandAll: boolean | undefined = $state()
	let grantOpen = $state(false)
	/** The row menu that is open, as `kind:path`. */
	let openMenu: string | undefined = $state()
	let grantPopover: Popover | undefined = $state()
	let grantAnchor: HTMLElement | undefined = $state()
	// Responses are not ordered: a slow answer for the previous principal must not land on this one.
	let requestId = 0

	export async function open(nextPrincipal: string, target: Target): Promise<void> {
		principal = nextPrincipal
		access = undefined
		error = undefined
		search = ''
		kind = undefined
		nbDisplayed = PAGE
		openState = {}
		expandAll = undefined
		grantOpen = false
		drawer?.openDrawer()
		logFeatureUsage('access_view', 'opened', { key: target })
		await reload()
	}

	async function reload(): Promise<void> {
		const id = ++requestId
		try {
			const result = await GranularAclService.getPrincipalAccess({
				workspace: $operatingWorkspace!,
				principal
			})
			if (id === requestId) access = result
		} catch (e) {
			if (id === requestId) error = e?.body ?? String(e)
		}
	}

	function setGrantOpen(next: boolean) {
		if (next) placeGrantPopover()
		grantOpen = next
		if (next) logFeatureUsage('access_view', 'grant_opened')
	}

	// The side is picked once, for the navigator's tallest height: melt fits the popover to the room
	// on the side it first lands on, so a list that grows later would never flip on its own.
	function placeGrantPopover() {
		if (!grantAnchor) return
		const rect = grantAnchor.getBoundingClientRect()
		const gutter = 8
		const spaceBelow = window.innerHeight - rect.bottom - gutter
		const spaceAbove = rect.top - gutter
		const below = navigatorMaxHeight() <= spaceBelow || spaceBelow >= spaceAbove
		grantPopover?.updatePositioning({
			placement: below ? 'bottom-end' : 'top-end',
			strategy: 'absolute',
			gutter,
			overflowPadding: 16,
			flip: false,
			fitViewport: true,
			overlap: false
		})
	}

	/** The principal's own entry on an item: `true` for write, `false` for read. */
	function ownItemGrant(item: AccessItem): boolean | undefined {
		const reason = item.reasons.find((r) =>
			isGroup ? r.type === 'group' && r.group === name : r.type === 'user'
		)
		return reason ? !!reason.write : undefined
	}

	async function changeGrant(kind: string, path: string, level: GrantLevel) {
		try {
			await setGrant($operatingWorkspace!, kind, path, principal, level)
		} catch (e) {
			const what = kind === 'folder' ? `f/${path}` : path
			sendUserToast(`Could not change access to ${what}: ${e?.body ?? e}`, true)
		}
		await reload()
	}

	/** `current` is the grant's level: `true` for write, `false` for read. */
	function grantActions(kind: string, path: string, current: boolean): Item[] {
		return [
			{
				displayName: 'Read',
				disabled: current === false,
				action: () => changeGrant(kind, path, 'read')
			},
			{
				displayName: 'Write',
				disabled: current === true,
				action: () => changeGrant(kind, path, 'write')
			},
			{ displayName: 'Remove', type: 'delete', action: () => changeGrant(kind, path, 'none') }
		]
	}

	// The tree's folder rows carry no grant of their own, so the menu reads it from here.
	const foldersByName = $derived(new Map((access?.folders ?? []).map((f) => [f.name, f])))

	const name = $derived(principal.slice(2))
	const isGroup = $derived(principal.startsWith('g/'))
	const who = $derived(isGroup ? `members of ${principal}` : principal)

	const kindItems = $derived.by(() => {
		const counts = new Map<string, number>()
		for (const item of access?.items ?? []) counts.set(item.kind, (counts.get(item.kind) ?? 0) + 1)
		return [...counts].map(([value, n]) => ({
			value,
			label: `${KIND_LABELS[value] ?? value} (${n})`
		}))
	})

	const query = $derived(search.trim().toLowerCase())
	const filtering = $derived(!!query || !!kind)
	const allOpen = $derived(expandAll ?? (access?.items.length ?? 0) <= OPEN_UP_TO)
	const tree = $derived(access && !access.is_admin ? buildAccessTree(access) : [])
	const rows = $derived(
		flattenAccessTree(tree, {
			matches: (item) =>
				(!kind || item.kind === kind) && (!query || item.path.toLowerCase().includes(query)),
			isOpen: (key) => openState[key] ?? allOpen,
			filtering
		})
	)

	function toggle(key: string, open: boolean): void {
		openState[key] = !open
	}

	function toggleAll(): void {
		expandAll = !allOpen
		openState = {}
	}

	function viaLabel(via: string): string {
		return via === principal ? 'Shared directly' : `Through ${via}`
	}

	/** Where holding the whole folder or space comes from. Empty when only some items are shared. */
	function ownerGrant(node: AccessOwnerNode): string {
		if (!node.whole) return ''
		if (node.key.startsWith('f/')) {
			return [...(node.admin ? ['Folder admin'] : []), ...node.via.map(viaLabel)].join(', ')
		}
		return node.key === principal ? 'Own path' : `Through ${node.key}`
	}

	function ownerExplanation(node: AccessOwnerNode): string {
		const verb = node.write ? 'write' : 'read'
		if (!node.whole) {
			return node.key.startsWith('f/')
				? `${node.key} itself isn't shared with ${principal}. Only the items listed under it are, one by one, so items added to ${node.key} later stay out of reach.`
				: `Only the items listed here are shared with ${principal}. Other items under ${node.key}/, and any added later, stay out of reach.`
		}
		if (node.key.startsWith('f/')) {
			return node.via.length
				? `${node.key} itself is shared with ${node.via.join(' and ')}, so ${who} can ${verb} every item in it, including items added later.`
				: `${principal} administers ${node.key}, so ${who} can write every item in it, including items added later.`
		}
		if (node.key === principal) {
			return isGroup
				? `Items under ${principal}/ belong to the group, so its members can write every one of them, including items added later.`
				: `Items under ${principal}/ belong to ${principal}, who can write every one of them, including items added later.`
		}
		return `${principal} is a member of ${node.key}, so ${principal} can write every item under ${node.key}/, including items added later.`
	}

	/** Grants on the item itself, which decide whether its level reads as its own. */
	function itemGrants(item: AccessItem): string[] {
		return item.reasons.flatMap((r) =>
			r.type === 'user' ? ['Shared directly'] : r.type === 'group' ? [viaLabel(`g/${r.group}`)] : []
		)
	}

	/** Every reason the item is held: its own grants, then its folder or space. */
	function itemReasons(item: AccessItem): string[] {
		const labels = item.reasons.map((r) =>
			r.type === 'user'
				? 'Shared directly'
				: r.type === 'group'
					? viaLabel(`g/${r.group}`)
					: r.type === 'folder'
						? `Through f/${r.folder}`
						: r.type === 'group_path'
							? `Through g/${r.group}`
							: 'Own path'
		)
		return [...new Set(labels)]
	}

	function levelClass(item: AccessItem, grants: string[], owner: AccessOwnerNode): string {
		if (!grants.length) return 'text-hint'
		return item.write && owner.whole && !owner.write ? 'font-semibold text-emphasis' : ''
	}

	// The drawer can report on the workspace a session editor acts in, which is not
	// necessarily the one the new tab would otherwise open.
	function withWorkspace(href: string): string {
		const [route, hash] = href.split(/#(.*)/s)
		const query = `?workspace=${encodeURIComponent($operatingWorkspace ?? '')}`
		return route + query + (hash !== undefined ? `#${hash}` : '')
	}

	function itemLink(kind: string, path: string): string | undefined {
		const href = itemHref(kind, path)
		return href ? withWorkspace(href) : undefined
	}
</script>

{#snippet chevron(key: string, open: boolean)}
	<!-- Keyboard access only: its click bubbles to the row, which does the toggling. -->
	<button
		type="button"
		class="text-secondary shrink-0"
		aria-expanded={open}
		aria-label={`${open ? 'Collapse' : 'Expand'} ${key}`}
	>
		{#if open}
			<ChevronDown size={14} />
		{:else}
			<ChevronRight size={14} />
		{/if}
	</button>
{/snippet}

{#snippet grantMenu(kind: string, path: string, current: boolean, blocked: string | undefined)}
	{#if blocked}
		<Tooltip>
			<Lock size={14} class="text-secondary" aria-label={blocked} />
			{#snippet text()}{blocked}{/snippet}
		</Tooltip>
	{:else}
		{@const key = `${kind}:${path}`}
		<DropdownV2
			size="xs"
			fixedHeight={false}
			class="-my-0.5"
			items={grantActions(kind, path, current)}
			bind:open={
				() => openMenu === key,
				(next) => {
					if (next) openMenu = key
					else if (openMenu === key) openMenu = undefined
				}
			}
		/>
	{/if}
{/snippet}

<!-- The drawer answers Escape on window too, and `preventEscape` is read after a popover or menu
     has already closed, so with the Grant popover or a row menu open its Escape is taken here
     before either sees it. -->
<svelte:window
	onkeydowncapture={(e) => {
		if (e.key !== 'Escape' || (!grantOpen && !openMenu)) return
		e.stopPropagation()
		e.preventDefault()
		grantOpen = false
		openMenu = undefined
	}}
/>

<Drawer bind:this={drawer} size="1000px">
	<DrawerContent title={`Access of ${principal}`} on:close={() => drawer?.closeDrawer()}>
		<div class="flex flex-col gap-6">
			<p class="text-xs text-secondary">
				{#if isGroup}
					What members of g/{name} can read through this group alone, and which grant gives each item.
					Members may reach more through their own grants and other groups.
				{:else}
					What {principal} can read, and which grant gives each item.
					{#if access && !access.is_admin}Jobs that run as {principal} read exactly this set.{/if}
				{/if}
			</p>

			{#if error}
				<Alert type="error" title="Could not load access">{error}</Alert>
			{:else if !access}
				<Skeleton layout={[[2], 1, [6], 1, [12]]} />
			{:else if access.is_admin}
				<Alert type="info" title="Workspace admin">
					{principal} is an admin of this workspace and can read and write every item in it.
				</Alert>
			{:else}
				{#if !isGroup}
					<Section label="Groups" small>
						{#if access.groups.length}
							<div class="flex flex-wrap gap-1">
								{#each access.groups as group (group)}
									<Badge color="gray">g/{group}</Badge>
								{/each}
							</div>
						{:else}
							<p class="text-xs text-secondary">Not in any group.</p>
						{/if}
					</Section>
				{/if}

				<Section label={`Items (${access.items.length})`} small>
					{#snippet action()}
						<div bind:this={grantAnchor}>
							<Popover
								bind:this={grantPopover}
								placement="bottom-end"
								usePointerDownOutside
								disableFocusTrap
								closeOnOtherPopoverOpen
								bind:isOpen={() => grantOpen, setGrantOpen}
								openFocus="[data-workspace-picker-search]"
								contentClasses="flex flex-col overflow-hidden"
							>
								{#snippet trigger()}
									<Button
										variant="default"
										unifiedSize="sm"
										nonCaptureEvent
										startIcon={{ icon: Plus }}
									>
										Grant
									</Button>
								{/snippet}
								{#snippet content()}
									{#if access}
										<GrantNavigator {principal} {access} onChange={reload} />
									{/if}
								{/snippet}
							</Popover>
						</div>
					{/snippet}
					<div class="flex flex-col gap-2">
						<div class="flex gap-2 items-center">
							<TextInput
								bind:value={search}
								inputProps={{ placeholder: 'Search paths' }}
								size="sm"
								class="grow"
							/>
							<div class="w-64">
								<Select
									items={kindItems}
									bind:value={kind}
									placeholder="All kinds"
									clearable
									size="sm"
								/>
							</div>
							<Button
								variant="subtle"
								unifiedSize="sm"
								disabled={filtering}
								startIcon={{ icon: allOpen ? ChevronsDownUp : ChevronsUpDown }}
								on:click={toggleAll}
							>
								{allOpen ? 'Collapse all' : 'Expand all'}
							</Button>
						</div>
						{#if rows.length}
							<DataTable
								size="xs"
								shouldLoadMore={rows.length > nbDisplayed}
								loadMore={PAGE}
								on:loadMore={() => (nbDisplayed += PAGE)}
							>
								<Head>
									<tr>
										<Cell head first>Name</Cell>
										<Cell head>Access</Cell>
										<Cell head>Granted by</Cell>
										<Cell head last actions />
									</tr>
								</Head>
								<tbody>
									{#each rows.slice(0, nbDisplayed) as row (row.type === 'item' ? `${row.item.kind}:${row.item.path}` : row.node.key)}
										{#if row.type === 'owner'}
											{@const node = row.node}
											{@const OwnerIcon = node.key.startsWith('f/')
												? Folder
												: node.key.startsWith('g/')
													? Users
													: User}
											<Row
												hoverable={row.count > 0}
												class="group"
												on:click={() => row.count > 0 && toggle(node.key, row.open)}
											>
												<Cell first>
													<div class="flex items-center gap-1.5">
														{#if row.count > 0}
															{@render chevron(node.key, row.open)}
														{:else}
															<span class="w-3.5 shrink-0"></span>
														{/if}
														<OwnerIcon size={14} class="text-secondary shrink-0" />
														<span class="font-semibold text-emphasis">{node.key}</span>
														<span class="text-secondary">{pluralize(row.count, 'item')}</span>
														{#if node.key.startsWith('f/')}
															<a
																href={withWorkspace(`/folders#${node.key}`)}
																target="_blank"
																class="text-secondary hover:text-emphasis opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
																aria-label={`Open ${node.key} in a new tab`}
																title={`Open ${node.key} in a new tab`}
																onclick={(e) => e.stopPropagation()}
															>
																<ExternalLink size={12} />
															</a>
														{/if}
													</div>
												</Cell>
												<Cell>
													<!-- A badge is a level held on the folder or space itself; item rows show theirs as text. -->
													<Tooltip class="-my-0.5">
														<Badge
															color="gray"
															class={node.whole
																? 'border border-transparent py-0'
																: 'bg-transparent border border-dashed border-border-normal py-0'}
														>
															{node.whole ? (node.write ? 'Write' : 'Read') : 'Some items'}
														</Badge>
														{#snippet text()}{ownerExplanation(node)}{/snippet}
													</Tooltip>
												</Cell>
												<Cell>{ownerGrant(node)}</Cell>
												<Cell last actions shouldStopPropagation>
													{@const folder = node.key.startsWith('f/')
														? foldersByName.get(node.key.slice(2))
														: undefined}
													{#if folder && !folder.owner && folder.grant != undefined}
														{@render grantMenu('folder', folder.name, folder.grant, folder.blocked)}
													{/if}
												</Cell>
											</Row>
										{:else if row.type === 'subfolder'}
											<Row hoverable on:click={() => toggle(row.node.key, row.open)}>
												<Cell first>
													<div
														class="flex items-center gap-1.5"
														style="padding-left: {row.depth * 20}px"
													>
														{@render chevron(row.node.key, row.open)}
														<FolderTree size={14} class="text-secondary shrink-0" />
														<span class="font-medium text-emphasis">{row.node.name}</span>
														<span class="text-secondary">{pluralize(row.count, 'item')}</span>
													</div>
												</Cell>
												<Cell></Cell>
												<Cell></Cell>
												<Cell last actions></Cell>
											</Row>
										{:else}
											{@const item = row.item}
											{@const grants = itemGrants(item)}
											{@const href = itemLink(item.kind, item.path)}
											<tr>
												<Cell first>
													<div
														class="flex items-center gap-1.5"
														style="padding-left: {row.depth * 20}px"
													>
														<span class="w-3.5 shrink-0"></span>
														<span class="flex shrink-0" title={KIND_LABELS[item.kind] ?? item.kind}>
															<RowIcon kind={item.kind as 'script'} size={14} />
														</span>
														{#if href}
															<a
																{href}
																target="_blank"
																title={item.path}
																class="group/link inline-flex items-center gap-1 text-primary hover:underline"
																>{row.name}<ExternalLink
																	size={12}
																	class="opacity-0 group-hover/link:opacity-100 group-focus-visible/link:opacity-100 shrink-0"
																/></a
															>
														{:else}
															<span title={item.path}>{row.name}</span>
														{/if}
													</div>
												</Cell>
												<Cell>
													<span class={levelClass(item, grants, row.owner)}
														>{item.write ? 'Write' : 'Read'}</span
													>
												</Cell>
												<Cell>{itemReasons(item).join(', ')}</Cell>
												<Cell last actions>
													{@const own = ownItemGrant(item)}
													{#if own !== undefined}
														{@render grantMenu(item.kind, item.path, own, item.blocked)}
													{/if}
												</Cell>
											</tr>
										{/if}
									{/each}
								</tbody>
							</DataTable>
						{:else}
							<p class="text-xs text-secondary">
								{access.items.length || access.folders.length
									? 'No item matches.'
									: `${principal} cannot read any item.`}
							</p>
						{/if}
					</div>
				</Section>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
