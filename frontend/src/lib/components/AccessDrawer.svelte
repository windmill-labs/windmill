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
	import { GranularAclService, type PrincipalAccess } from '$lib/gen'
	import { useOperatingWorkspace } from './operatingWorkspace.svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { pluralize } from '$lib/utils'
	import { itemHref } from './offboarding-utils'
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
		User,
		Users
	} from 'lucide-svelte'

	const operatingWorkspace = useOperatingWorkspace()

	type Target = 'self' | 'user' | 'service_account' | 'group'

	const PAGE = 100
	// A short list opens fully, so every item shows at once. A longer one starts at its folders.
	const OPEN_UP_TO = 30

	const KIND_LABELS: Record<string, string> = {
		script: 'Script',
		flow: 'Flow',
		app: 'App',
		resource: 'Resource',
		variable: 'Variable',
		schedule: 'Schedule',
		http_trigger: 'HTTP route',
		websocket_trigger: 'WebSocket trigger',
		kafka_trigger: 'Kafka trigger',
		nats_trigger: 'NATS trigger',
		postgres_trigger: 'Postgres trigger',
		mqtt_trigger: 'MQTT trigger',
		amqp_trigger: 'AMQP trigger',
		gcp_trigger: 'GCP Pub/Sub trigger',
		azure_trigger: 'Azure trigger',
		sqs_trigger: 'SQS trigger',
		email_trigger: 'Email trigger'
	}

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
		drawer?.openDrawer()
		logFeatureUsage('access_view', 'opened', { key: target })
		const id = ++requestId
		try {
			const result = await GranularAclService.getPrincipalAccess({
				workspace: $operatingWorkspace!,
				principal: nextPrincipal
			})
			if (id === requestId) access = result
		} catch (e) {
			if (id === requestId) error = e?.body ?? String(e)
		}
	}

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

	/** Grants on the item itself. What its folder or space gives is said once, on that row. */
	function itemGrants(item: AccessItem): string[] {
		return item.reasons.flatMap((r) =>
			r.type === 'user' ? ['Shared directly'] : r.type === 'group' ? [viaLabel(`g/${r.group}`)] : []
		)
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

<Drawer bind:this={drawer} size="1000px">
	<DrawerContent title={`Access of ${principal}`} on:close={() => drawer?.closeDrawer()}>
		<div class="flex flex-col gap-6">
			<p class="text-xs text-secondary">
				{#if isGroup}
					What members of g/{name} can read through this group alone, and which grant gives each item.
					Members may reach more through their own grants and other groups.
				{:else}
					What {principal} can read, and which grant gives each item. Jobs that run as
					{principal} read exactly this set.
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
										<Cell head last>Granted by</Cell>
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
												<Cell last>{ownerGrant(node)}</Cell>
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
												<Cell last></Cell>
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
												<Cell last>{grants.join(', ')}</Cell>
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
