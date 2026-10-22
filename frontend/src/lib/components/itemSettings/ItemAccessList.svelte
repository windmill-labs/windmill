<script lang="ts">
	import { FolderService, GranularAclService, GroupService, UserService } from '$lib/gen'
	import type { Folder } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { ChevronDown, Plus, Trash } from 'lucide-svelte'
	import GroupEditorDrawer from '$lib/components/GroupEditorDrawer.svelte'
	import FolderEditorDrawer from '$lib/components/FolderEditorDrawer.svelte'
	import { Cell, DataTable, Row } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import AccessHelp from './AccessHelp.svelte'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { isOwner } from '$lib/utils'
	import { DEMO_RESTRICTION_HINT, isDemoWorkspaceRestricted } from '$lib/cloud'
	import { useScheduleLock, useTriggerLock } from '$lib/operatorWriteRights'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { byName, folderOfPath, inheritedAccess } from './inheritedAccess'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		/** The granular ACL kind of the item, as GranularAclService names it. */
		kind: string
		path: string
		/** The list's heading. */
		title?: string
		/** Variables to give the same extra members, e.g. those a resource links to. */
		alsoApplyTo?: string[]
		/** When set, members are collected here instead of written, for an item not deployed yet. */
		pending?: Record<string, boolean>
		/** For an item whose name is not a path, such as a volume. */
		isOwnerOverride?: boolean
		onChange?: () => void
	}

	let {
		kind,
		path,
		title = 'People with access',
		alsoApplyTo = [],
		pending = $bindable(),
		isOwnerOverride,
		onChange
	}: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const scheduleLock = useScheduleLock()
	const triggerLock = useTriggerLock()

	let restricted = $derived(
		isDemoWorkspaceRestricted(
			$operatingWorkspace,
			operatingUser.current?.is_admin,
			operatingUser.current?.is_super_admin
		)
	)
	// Sharing a schedule or trigger is a write operators can lose; reading stays open.
	let writeLock = $derived(
		kind === 'schedule' ? $scheduleLock : kind.endsWith('_trigger') ? $triggerLock : undefined
	)
	// Derived, not captured once: the operating user resolves after mount in a fork.
	let own = $derived(
		isOwnerOverride ??
			(!!operatingUser.current && isOwner(path, operatingUser.current, $operatingWorkspace))
	)
	let canEdit = $derived(own && !restricted && !writeLock)

	let folder: Folder | undefined = $state()
	let saved: Record<string, boolean> = $state({})
	// Every user and group of the workspace, for the Add people picker: loaded on its first use,
	// since it grows with the workspace and most opens of this list add no one.
	let principals: string[] | undefined = $state()
	let newMember: string | undefined = $state()
	let adding = $state(false)
	let groupDrawer: GroupEditorDrawer | undefined = $state()
	let folderDrawer: FolderEditorDrawer | undefined = $state()

	let members = $derived(pending ?? saved)
	let inherited = $derived(inheritedAccess(path, folder))

	type ListRow = {
		name: string
		write: boolean
		source: string
		editable: boolean
		/** The folder the access comes from. */
		folder?: string
	}
	let ownRows: ListRow[] = $derived(
		Object.entries(members)
			.map(([name, write]) => ({ name, write, source: 'Extra permission', editable: true }))
			.sort(byName)
	)
	let inheritedRows: ListRow[] = $derived(
		(inherited?.rows ?? []).map((r) => ({
			...r,
			source: inherited!.source,
			editable: false,
			folder: folder?.name
		}))
	)
	// The rows this list can change stay above a folder's members, who can run to hundreds; the
	// owner of a `u/` or `g/` path stays first.
	let rows = $derived(
		folderOfPath(path) !== undefined
			? [...ownRows, ...inheritedRows]
			: [...inheritedRows, ...ownRows]
	)

	// From this many rows a search shows, and the list may shrink to its minimum height and scroll
	// when the dialog runs out of room. Both follow every row, not the matching ones, so typing
	// never hides the field.
	const MANY_ROWS = 8
	let many = $derived(rows.length >= MANY_ROWS)
	let search = $state('')
	let query = $derived(many ? search.trim().toLowerCase() : '')
	let shownRows = $derived(query ? rows.filter((r) => r.name.toLowerCase().includes(query)) : rows)
	// The list's height with every row shown, kept while a search narrows it: a dialog sized by
	// its content would otherwise shrink, and move, as matches drop out.
	let fullHeight = $state(0)

	// Member counts of the groups the list shows, keyed `g/<name>`, one read per group. Called
	// wherever group rows can appear. A group that cannot be read gets no count and no button.
	let groupSizes: Record<string, number> = $state({})
	const requestedGroups = new Set<string>()
	function loadGroupSizes() {
		for (const { name } of rows) {
			if (!name.startsWith('g/') || requestedGroups.has(name)) continue
			requestedGroups.add(name)
			loadGroupSize(name)
		}
	}
	function loadGroupSize(name: string) {
		GroupService.getGroup({ workspace: $operatingWorkspace!, name: name.slice(2) })
			.then((g) => (groupSizes[name] = g.members?.length ?? 0))
			.catch(() => {})
	}

	async function loadFolder() {
		const name = folderOfPath(path)
		if (name === undefined) return
		try {
			folder = await FolderService.getFolder({ workspace: $operatingWorkspace!, name })
			loadGroupSizes()
		} catch {
			// A user outside the folder cannot read it; the list then shows only admins.
		}
	}

	async function loadAcls() {
		if (pending !== undefined) return
		saved = await GranularAclService.getGranularAcls({
			workspace: $operatingWorkspace!,
			path,
			kind: kind as any
		})
		loadGroupSizes()
	}

	async function loadPrincipals() {
		try {
			const [users, groups] = await Promise.all([
				UserService.listUsernames({ workspace: $operatingWorkspace! }),
				GroupService.listGroupNames({ workspace: $operatingWorkspace! })
			])
			principals = [...users.map((u) => `u/${u}`), ...groups.map((g) => `g/${g}`)]
		} catch (err) {
			sendUserToast(`Could not load users and groups: ${err}`, true)
		}
	}

	function startAdding() {
		adding = true
		if (principals === undefined) loadPrincipals()
	}

	// Once: a caller showing another item remounts this list.
	untrack(() => {
		if (!path) return
		// The owning group of a `g/` path is a row before anything loads.
		loadGroupSizes()
		loadFolder()
		loadAcls().catch((err) => sendUserToast(`Could not load who has access: ${err}`, true))
	})

	async function forEachTarget(fn: (path: string, kind: string) => Promise<unknown>) {
		await fn(path, kind)
		for (const varPath of alsoApplyTo) {
			try {
				await fn(varPath, 'variable')
			} catch (err) {
				sendUserToast(`Failed to update variable ${varPath}: ${err}`, true)
			}
		}
	}

	async function setMember(owner: string, write: boolean) {
		if (pending !== undefined) {
			pending = { ...pending, [owner]: write }
			loadGroupSizes()
			return
		}
		try {
			await forEachTarget((p, k) =>
				GranularAclService.addGranularAcls({
					workspace: $operatingWorkspace!,
					path: p,
					kind: k as any,
					requestBody: { owner, write }
				})
			)
			onChange?.()
		} catch (err) {
			sendUserToast(`Could not share with ${owner}: ${err}`, true)
		}
		await loadAcls()
	}

	async function removeMember(owner: string) {
		if (pending !== undefined) {
			const { [owner]: _, ...rest } = pending
			pending = rest
			return
		}
		try {
			await forEachTarget((p, k) =>
				GranularAclService.removeGranularAcls({
					workspace: $operatingWorkspace!,
					path: p,
					kind: k as any,
					requestBody: { owner }
				})
			)
			onChange?.()
		} catch (err) {
			sendUserToast(`Could not remove ${owner}: ${err}`, true)
		}
		await loadAcls()
	}

	let candidates = $derived(
		(principals ?? []).filter((p) => !(p in members)).map((p) => ({ label: p, value: p }))
	)
</script>

<!-- As an item of a flex column with a definite height, a long list gives up height down to its
     minimum and scrolls under its sticky header, so the rest of the dialog keeps its place. -->
<div
	class="flex flex-col gap-2 {many ? 'min-h-60' : ''}"
	style:height={query && fullHeight ? `${fullHeight}px` : undefined}
	bind:clientHeight={
		null,
		(h) => {
			if (!query) fullHeight = h ?? 0
		}
	}
>
	<div class="flex items-center gap-2 h-7">
		<h3 class="grow shrink-0 flex items-center gap-1 text-sm font-semibold text-emphasis">
			{title}
			<AccessHelp />
		</h3>
		{#if many}
			<TextInput
				size="sm"
				class="w-56 min-w-0"
				bind:value={search}
				inputProps={{
					type: 'search',
					placeholder: `Search ${rows.length} users and groups`,
					'aria-label': 'Search people with access'
				}}
			/>
		{/if}
		{#if canEdit && !adding}
			<Button variant="default" unifiedSize="sm" startIcon={{ icon: Plus }} onclick={startAdding}>
				Add people
			</Button>
		{/if}
	</div>

	{#if canEdit && adding}
		<div class="flex flex-row gap-2 items-center">
			<Select
				items={candidates}
				loading={principals === undefined}
				bind:value={newMember}
				placeholder="A user or group"
				class="grow min-w-0"
				size="sm"
				autofocus
			/>
			<Button
				variant="default"
				unifiedSize="sm"
				disabled={!newMember}
				onclick={async () => {
					if (!newMember) return
					const member = newMember
					newMember = undefined
					adding = false
					await setMember(member, false)
				}}
			>
				Add
			</Button>
			<Button
				variant="subtle"
				unifiedSize="sm"
				onclick={() => {
					newMember = undefined
					adding = false
				}}
			>
				Cancel
			</Button>
		</div>
	{/if}

	<!-- Takes what the list's own height leaves (its default `h-full`), which needs that height to
	     be definite for the table's box to scroll. Fixed column widths keep the columns still as
	     the rows change. -->
	<DataTable size="sm" tableFixed>
		<Head>
			<tr>
				<Cell head first class="text-secondary">Name</Cell>
				<Cell head class="text-secondary w-36">Access from</Cell>
				<Cell head class="text-secondary w-28">Role</Cell>
				<Cell head last actions class="w-16"></Cell>
			</tr>
		</Head>
		<tbody class="divide-y">
			{#each shownRows as row (`${row.source}:${row.name}`)}
				{@const size = groupSizes[row.name]}
				<Row>
					<Cell first>
						<div class="flex items-center gap-1.5 min-w-0">
							<span class="truncate" title={row.name}>{row.name}</span>
							{#if size !== undefined}
								<span class="text-hint shrink-0" aria-hidden="true">·</span>
								<!-- The demo workspace hides who is in `all`, answering a single placeholder.
								     `-ml-1` cancels the button's padding, so its label sits as far from the dot
								     as the name does. -->
								<div class="-my-1 -ml-1 shrink-0">
									<Button
										variant="subtle"
										unifiedSize="2xs"
										btnClasses="text-secondary hover:text-primary"
										title="Members of {row.name}"
										onclick={() => groupDrawer?.initEdit(row.name.slice(2))}
									>
										{restricted ? 'Members' : `${size} ${size === 1 ? 'member' : 'members'}`}
									</Button>
								</div>
							{/if}
						</div>
					</Cell>
					<Cell class="text-secondary truncate" title={row.source}>
						{#if row.folder}
							{@const folderName = row.folder}
							<!-- `-ml-1` cancels the button's padding, so its label lines up with the
							     sources of the other rows. -->
							<div class="flex min-w-0 -ml-1 -my-1">
								<Button
									variant="subtle"
									unifiedSize="2xs"
									wrapperClasses="min-w-0 shrink"
									btnClasses="min-w-0 text-secondary hover:text-primary"
									title="Members of folder {folderName}"
									onclick={() => folderDrawer?.initEdit(folderName)}
								>
									<span class="truncate">{row.source}</span>
								</Button>
							</div>
						{:else}
							{row.source}
						{/if}
					</Cell>
					<Cell>
						{#if row.editable && canEdit}
							<!-- Negative margins keep the row as tall as the plain ones and line the
							     button's label up with their roles. -->
							<div class="-ml-3 -my-2">
								<DropdownV2
									items={[
										{
											displayName: 'Viewer',
											action: () => row.write && setMember(row.name, false)
										},
										{
											displayName: 'Writer',
											action: () => !row.write && setMember(row.name, true)
										}
									]}
									placement="bottom-start"
								>
									{#snippet buttonReplacement()}
										<Button
											nonCaptureEvent
											variant="subtle"
											unifiedSize="sm"
											endIcon={{ icon: ChevronDown }}
											aria-label="Role of {row.name}"
										>
											{row.write ? 'Writer' : 'Viewer'}
										</Button>
									{/snippet}
								</DropdownV2>
							</div>
						{:else}
							<span class="text-secondary">{row.write ? 'Writer' : 'Viewer'}</span>
						{/if}
					</Cell>
					<Cell last actions>
						{#if row.editable && canEdit}
							<div class="-my-2">
								<Button
									variant="subtle"
									destructive
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: Trash }}
									aria-label="Remove {row.name}"
									onclick={() => removeMember(row.name)}
								/>
							</div>
						{/if}
					</Cell>
				</Row>
			{/each}
			{#if inherited?.note && !query}
				<Row>
					<Cell first class="text-secondary truncate" title={inherited.note}>
						{inherited.note}
					</Cell>
					<Cell class="text-secondary truncate" title={inherited.source}>
						{inherited.source}
					</Cell>
					<Cell></Cell>
					<Cell last actions></Cell>
				</Row>
			{:else if shownRows.length === 0}
				<Row>
					<Cell first last colspan={4} class="text-hint truncate">
						{query ? `No one matches “${search.trim()}”` : 'Only workspace admins'}
					</Cell>
				</Row>
			{/if}
		</tbody>
	</DataTable>

	{#if !own && operatingUser.current}
		<span class="text-2xs text-secondary">
			Only its owner and workspace admins can change who has access.
		</span>
	{:else if own && (restricted || writeLock)}
		<span class="text-2xs text-secondary">{writeLock ?? DEMO_RESTRICTION_HINT}</span>
	{/if}
</div>

<!-- Show who a group or folder row stands for. Each editor decides whether this user may change
     it, and opens read-only otherwise; a change there reaches everything shared with the group
     or kept in the folder. -->
<GroupEditorDrawer bind:this={groupDrawer} onSaved={(name) => loadGroupSize(`g/${name}`)} />
<FolderEditorDrawer bind:this={folderDrawer} onSaved={loadFolder} />
