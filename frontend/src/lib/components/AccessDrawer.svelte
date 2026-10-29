<script lang="ts">
	import { Alert, Badge, Drawer, DrawerContent, Section, Skeleton } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import { GranularAclService, type AccessReason, type PrincipalAccess } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { Tab, Tabs } from './common'
	import AccessGrantPanel from './access/AccessGrantPanel.svelte'
	import AddNeededAccess from './access/AddNeededAccess.svelte'
	import { KIND_LABELS } from './access/access'
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
	let tab: 'access' | 'grant' = $state('access')
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
		tab = 'access'
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

	function selectTab(next: 'access' | 'grant') {
		tab = next
		if (next === 'grant') logFeatureUsage('access_view', 'grant_opened')
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
				<Tabs bind:selected={() => tab, (v) => selectTab(v as 'access' | 'grant')}>
					<Tab value="access" label="Access" />
					<Tab value="grant" label="Grant" />
				</Tabs>
			{/if}

			{#if access && !access.is_admin && tab === 'grant'}
				<AddNeededAccess {principal} {reads} onGranted={reload} />
				<AccessGrantPanel {principal} onChanged={reload} />
			{:else if access && !access.is_admin}
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
									<Cell head last>Why</Cell>
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each access.folders as folder (folder.name)}
									<tr>
										<Cell first>f/{folder.name}</Cell>
										<Cell>{folder.write ? 'Write' : 'Read'}</Cell>
										<Cell last>
											<div class="flex flex-wrap gap-1">
												{#if folder.owner}
													<Badge color="blue">Owner</Badge>
												{/if}
												{#each folder.via as via (via)}
													<Badge color="gray">{folderVia(via)}</Badge>
												{/each}
											</div>
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
					<div class="flex flex-col gap-2">
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
										<Cell head last>Why</Cell>
									</tr>
								</Head>
								<tbody class="divide-y">
									{#each filteredItems.slice(0, nbDisplayed) as item (item.kind + ':' + item.path)}
										<tr>
											<Cell first><span class="font-mono text-2xs">{item.path}</span></Cell>
											<Cell>{KIND_LABELS[item.kind] ?? item.kind}</Cell>
											<Cell>{item.write ? 'Write' : 'Read'}</Cell>
											<Cell last>
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
