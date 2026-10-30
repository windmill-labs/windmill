<script lang="ts">
	import { Alert, Badge, Button, Drawer, DrawerContent, Section, Skeleton } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import { GranularAclService, type AccessReason, type PrincipalAccess } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import type { Item } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { Lock, Plus } from 'lucide-svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import AddNeededAccess from './access/AddNeededAccess.svelte'
	import GrantNavigator, { navigatorMaxHeight } from './access/GrantNavigator.svelte'
	import { KIND_LABELS, setGrant, type GrantLevel } from './access/access'
	import type { RunnableKind } from './access/detectNeeds'
	import DeployersField from './access/DeployersField.svelte'

	type Target = 'self' | 'user' | 'service_account' | 'group'

	const PAGE = 100

	let drawer: Drawer | undefined = $state()
	let principal = $state('')
	let target: Target = $state('user')
	let access: PrincipalAccess | undefined = $state()
	let error: string | undefined = $state()
	let search = $state('')
	let kind: string | undefined = $state()
	let nbDisplayed = $state(PAGE)
	let grantOpen = $state(false)
	let grantPopover: Popover | undefined = $state()
	let grantAnchor: HTMLElement | undefined = $state()
	let needsOf: { kind: RunnableKind; path: string } | undefined = $state()
	// Responses are not ordered: a slow answer for the previous principal must not land on this one.
	let requestId = 0

	export async function open(nextPrincipal: string, nextTarget: Target): Promise<void> {
		principal = nextPrincipal
		target = nextTarget
		access = undefined
		error = undefined
		search = ''
		kind = undefined
		nbDisplayed = PAGE
		grantOpen = false
		needsOf = undefined
		drawer?.openDrawer()
		logFeatureUsage('access_view', 'opened', { key: nextTarget })
		await reload()
	}

	async function reload(): Promise<void> {
		const id = ++requestId
		try {
			const result = await GranularAclService.getPrincipalAccess({
				workspace: $workspaceStore!,
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

	function findReads(kind: RunnableKind, path: string) {
		grantOpen = false
		needsOf = { kind, path }
	}

	/** The principal's own entry on an item: `true` for write, `false` for read. */
	function ownItemGrant(item: PrincipalAccess['items'][number]): boolean | undefined {
		const reason = item.reasons.find((r) =>
			isGroup ? r.type === 'group' && r.group === name : r.type === 'user'
		)
		return reason ? !!reason.write : undefined
	}

	async function changeGrant(kind: string, path: string, level: GrantLevel) {
		try {
			await setGrant($workspaceStore!, kind, path, principal, level)
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

	const reads = $derived(new Set((access?.items ?? []).map((i) => `${i.kind}:${i.path}`)))

	const name = $derived(principal.slice(2))
	const isGroup = $derived(principal.startsWith('g/'))

	const kindItems = $derived.by(() => {
		const counts = new Map<string, number>()
		for (const item of access?.items ?? []) counts.set(item.kind, (counts.get(item.kind) ?? 0) + 1)
		return [...counts].map(([value, n]) => ({
			value,
			label: `${KIND_LABELS[value] ?? value} (${n})`
		}))
	})

	const filteredItems = $derived.by(() => {
		const q = search.trim().toLowerCase()
		return (access?.items ?? []).filter(
			(item) => (!kind || item.kind === kind) && (!q || item.path.toLowerCase().includes(q))
		)
	})

	function reasonLabel(reason: AccessReason): string {
		switch (reason.type) {
			case 'own':
				return 'Own path'
			case 'group_path':
				return `Path of g/${reason.group}`
			case 'user':
				return `Shared with ${principal}`
			case 'group':
				return `Shared with g/${reason.group}`
			case 'folder':
				return `Folder f/${reason.folder}`
		}
	}

	// `own` and `group_path` always grant write, so only grants that can be either say which.
	function reasonAccess(reason: AccessReason): string | undefined {
		if (reason.type === 'own' || reason.type === 'group_path') return undefined
		return reason.write ? 'write' : 'read'
	}

	function folderVia(via: string): string {
		return via === principal ? 'Shared directly' : `Through ${via}`
	}
</script>

{#snippet grantMenu(kind: string, path: string, current: boolean, blocked: string | undefined)}
	{#if blocked}
		<Tooltip>
			<Lock size={14} class="text-secondary" aria-label={blocked} />
			{#snippet text()}{blocked}{/snippet}
		</Tooltip>
	{:else}
		<DropdownV2 items={grantActions(kind, path, current)} />
	{/if}
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
				{#if target === 'service_account' && $userStore?.is_admin}
					<Section label="Who can deploy as it" small>
						{#key principal}
							<DeployersField {principal} {access} />
						{/key}
					</Section>
				{/if}
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

				<Section label="Folders" small>
					{#if access.folders.length}
						<DataTable size="xs">
							<Head>
								<tr>
									<Cell head first>Folder</Cell>
									<Cell head>Access</Cell>
									<Cell head>Why</Cell>
									<Cell head last />
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each access.folders as folder (folder.name)}
									<tr>
										<Cell first>f/{folder.name}</Cell>
										<Cell>{folder.write ? 'Write' : 'Read'}</Cell>
										<Cell>
											<div class="flex flex-wrap gap-1">
												{#if folder.owner}
													<Badge color="blue">Owner</Badge>
												{/if}
												{#each folder.via as via (via)}
													<Badge color="gray">{folderVia(via)}</Badge>
												{/each}
											</div>
										</Cell>
										<Cell last>
											{#if !folder.owner && folder.grant != undefined}
												{@render grantMenu('folder', folder.name, folder.grant, folder.blocked)}
											{/if}
										</Cell>
									</tr>
								{/each}
							</tbody>
						</DataTable>
					{:else}
						<p class="text-xs text-secondary">No folder is shared with {principal}.</p>
					{/if}
				</Section>

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
										<GrantNavigator
											{principal}
											{access}
											onGranted={reload}
											onFindReads={findReads}
										/>
									{/if}
								{/snippet}
							</Popover>
						</div>
					{/snippet}
					<div class="flex flex-col gap-2">
						{#if needsOf}
							{#key `${needsOf.kind}:${needsOf.path}`}
								<AddNeededAccess
									{principal}
									kind={needsOf.kind}
									path={needsOf.path}
									{reads}
									onGranted={reload}
									onClose={() => (needsOf = undefined)}
								/>
							{/key}
						{/if}
						<div class="flex gap-2">
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
						</div>
						{#if filteredItems.length}
							<DataTable
								size="xs"
								shouldLoadMore={filteredItems.length > nbDisplayed}
								loadMore={PAGE}
								on:loadMore={() => (nbDisplayed += PAGE)}
							>
								<Head>
									<tr>
										<Cell head first>Path</Cell>
										<Cell head>Kind</Cell>
										<Cell head>Access</Cell>
										<Cell head>Why</Cell>
										<Cell head last />
									</tr>
								</Head>
								<tbody class="divide-y">
									{#each filteredItems.slice(0, nbDisplayed) as item (item.kind + ':' + item.path)}
										{@const own = ownItemGrant(item)}
										<tr>
											<Cell first><span class="font-mono text-2xs">{item.path}</span></Cell>
											<Cell>{KIND_LABELS[item.kind] ?? item.kind}</Cell>
											<Cell>{item.write ? 'Write' : 'Read'}</Cell>
											<Cell>
												<div class="flex flex-wrap gap-1">
													{#each item.reasons as reason, i (i)}
														{@const level = reasonAccess(reason)}
														<Badge color="gray">
															{reasonLabel(reason)}{#if level}<span class="text-secondary"
																	>&nbsp;· {level}</span
																>{/if}
														</Badge>
													{/each}
												</div>
											</Cell>
											<Cell last>
												{#if own !== undefined}
													{@render grantMenu(item.kind, item.path, own, item.blocked)}
												{/if}
											</Cell>
										</tr>
									{/each}
								</tbody>
							</DataTable>
						{:else}
							<p class="text-xs text-secondary">
								{access.items.length ? 'No item matches.' : `${principal} cannot read any item.`}
							</p>
						{/if}
					</div>
				</Section>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
