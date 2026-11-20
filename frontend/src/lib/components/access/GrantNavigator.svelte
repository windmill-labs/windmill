<!--
@component
The access view's "+ Grant" list: the caller's own `u/` folder and every folder, with their items.
A row does nothing when clicked. The highlighted row offers each change to the principal's own
grant on it that changes what the principal can do (grant read or write, revoke write or read),
the most useful one as its button and the others in its dropdown. The "All of f/x" row
changes the grant on the whole folder.
-->
<script module lang="ts">
	/** The navigator's tallest height, `max-h-[min(460px,60vh)]` below, for its host to place it. */
	export function navigatorMaxHeight(): number {
		return Math.min(460, window.innerHeight * 0.6)
	}
</script>

<script lang="ts">
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillLeaf, DrillLeafAction, DrillNode } from '$lib/components/drillPicker'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import {
		GranularAclService,
		type AccessReason,
		type BrowseAccessFoldersResponse,
		type BrowseAccessItemsResponse,
		type PrincipalAccess
	} from '$lib/gen'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { sendUserToast } from '$lib/toast'
	import { debounce } from '$lib/utils'
	import { Folder, User, Users } from 'lucide-svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { setOverlayHost } from '$lib/components/common/overlayHost.svelte'
	import { setGrant, type GrantLevel, type GrantSource } from './access'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()

	type Row = BrowseAccessItemsResponse['items'][number]
	/** `kind` `folder` is the "All of f/x" row, its `path` the folder name. */
	type Leaf = Row & { owner?: boolean }
	/** What the principal holds on a row besides its own entry: the level, `undefined` where the
	 * access report cannot tell, and where it comes from, in the access table's words. */
	type Inherited = { level: GrantLevel | undefined; hows: string[] }

	let {
		principal,
		access,
		onChange,
		source
	}: {
		principal: string
		access: PrincipalAccess
		/** After each change to a grant, for the host to reload `access`. */
		onChange: () => void
		source?: GrantSource
	} = $props()

	const api: GrantSource = {
		get ownSpace() {
			return `u/${operatingUser.current?.username}`
		},
		folders: () =>
			GranularAclService.browseAccessFolders({ workspace: $operatingWorkspace!, principal }),
		items: (query) =>
			GranularAclService.browseAccessItems({
				workspace: $operatingWorkspace!,
				principal,
				...query
			}),
		set: (kind, path, level) => setGrant($operatingWorkspace!, kind, path, principal, level)
	}
	const data = $derived(source ?? api)
	const own = $derived(data.ownSpace)

	let picker: { focus: () => void; escape: (target: EventTarget | null) => boolean } | undefined =
		$state()
	let folders: BrowseAccessFoldersResponse | undefined = $state()
	/** Rows by top-level branch (`u/alice`, `f/billing`), then by `kind:path`. */
	let rows: Record<string, Record<string, Row>> = $state({})
	let loading: Record<string, boolean> = $state({})
	const loaded = new Set<string>()
	/** Branches whose every page has arrived, so no rows there means the folder has no items. */
	let complete: Record<string, boolean> = $state({})
	/** The change in flight, as `kind:path:label`. */
	let pending: string | undefined = $state()
	/** Runs before this time are ignored: the second press of a double click or Enter would
	 * otherwise run the button that took the first one's place. */
	let settledUntil = 0

	function branchOf(path: string): string {
		return path.split('/', 2).join('/')
	}

	function merge(items: Row[]) {
		for (const item of items) {
			const branch = branchOf(item.path)
			rows[branch] ??= {}
			rows[branch][`${item.kind}:${item.path}`] = item
		}
	}

	async function loadFolders() {
		try {
			folders = await data.folders()
		} catch (e) {
			sendUserToast(`Could not load folders: ${e?.body ?? e}`, true)
		}
	}

	async function loadBranch(branch: string) {
		if (loaded.has(branch)) return
		loaded.add(branch)
		loading[branch] = true
		try {
			let offset = 0
			while (true) {
				const page = await data.items({ prefix: `${branch}/`, offset })
				merge(page.items)
				// The branch shows from its first page; later pages fill it in as they arrive.
				loading[branch] = false
				if (!page.has_more) break
				offset += page.items.length
			}
			complete[branch] = true
		} catch (e) {
			loaded.delete(branch)
			sendUserToast(`Could not load ${branch}: ${e?.body ?? e}`, true)
		} finally {
			loading[branch] = false
		}
	}

	// The picker searches only the rows it holds, so matches anywhere are fetched into the tree.
	const searchSoon = debounce(async (search: string) => {
		try {
			merge((await data.items({ search })).items)
		} catch (e) {
			sendUserToast(`Could not search: ${e?.body ?? e}`, true)
		}
	}, 250).debounced

	loadFolders()

	function itemLeaf(row: Row): DrillLeaf<Leaf> {
		return {
			type: 'leaf',
			key: `${row.kind}:${row.path}`,
			label: row.path.split('/').slice(2).join('/'),
			secondary: row.path,
			searchableText: row.path,
			data: row,
			disabled: !!row.blocked
		}
	}

	function itemLeaves(branch: string): DrillLeaf<Leaf>[] {
		return Object.values(rows[branch] ?? {})
			.sort((a, b) => a.path.localeCompare(b.path) || a.kind.localeCompare(b.kind))
			.map(itemLeaf)
	}

	function folderData(folder: BrowseAccessFoldersResponse[number]): Leaf {
		return {
			kind: 'folder',
			path: folder.name,
			grant: folder.grant,
			blocked: folder.blocked,
			owner: folder.target_owner
		}
	}

	const tree = $derived.by<DrillNode<Leaf>[]>(() => {
		const folderNames = new Set((folders ?? []).map((f) => `f/${f.name}`))
		const others = Object.keys(rows)
			.filter((b) => b !== own && !folderNames.has(b))
			.sort()
		return [
			{
				type: 'branch',
				key: own,
				label: own,
				icon: User,
				searchGroup: true,
				loading: loading[own],
				children: itemLeaves(own)
			},
			...(folders ?? []).map((folder): DrillNode<Leaf> => {
				const branch = `f/${folder.name}`
				const items = itemLeaves(branch)
				const level = levelOf(folderData(folder), `folder:${folder.name}`)
				return {
					type: 'branch',
					key: branch,
					label: branch,
					icon: Folder,
					searchGroup: true,
					loading: loading[branch],
					badge: level === 'none' ? undefined : LEVEL_LABELS[level],
					note:
						complete[branch] && items.length === 0
							? `${branch} has no items yet. A grant on the whole folder covers the items added later.`
							: undefined,
					children: [
						{
							type: 'leaf',
							key: `folder:${folder.name}`,
							label: `All of f/${folder.name}`,
							searchableText: branch,
							data: folderData(folder),
							disabled: folder.target_owner || !!folder.blocked
						},
						...items
					]
				}
			}),
			// Other users' and groups' items appear once a search finds them.
			...others.map(
				(b): DrillNode<Leaf> => ({
					type: 'branch',
					key: b,
					label: b,
					icon: b.startsWith('g/') ? Users : User,
					searchGroup: true,
					children: itemLeaves(b)
				})
			)
		]
	})

	const reasons = $derived(
		new Map<string, AccessReason[]>(
			access.items.map((i) => [`${i.kind}:${i.path}`, i.reasons] as const)
		)
	)
	const reachedFolders = $derived(new Map(access.folders.map((f) => [f.name, f])))

	const isGroup = $derived(principal.startsWith('g/'))
	const who = $derived(isGroup ? `members of ${principal}` : principal)

	/** What read and write let the principal do, by kind. */
	const GIVES: Record<string, { read: string; write: string }> = {
		folder: { read: 'read all its items, new ones included', write: 'edit and add items' },
		script: { read: 'see and run it', write: 'edit it' },
		flow: { read: 'see and run it', write: 'edit it' },
		app: { read: 'open and use it', write: 'edit it' },
		raw_app: { read: 'open and use it', write: 'edit it' },
		resource: { read: 'use it and see its value', write: 'change its value' },
		variable: { read: 'use it and see its value', write: 'change its value' },
		schedule: { read: 'see it', write: 'edit, enable and disable it' }
	}
	const GIVES_OTHER = { read: 'see it', write: 'edit it' }

	const LEVELS: GrantLevel[] = ['none', 'read', 'write']
	const LEVEL_LABELS: Record<GrantLevel, string> = {
		none: 'No access',
		read: 'Read',
		write: 'Write'
	}

	function maxLevel(a: GrantLevel, b: GrantLevel): GrantLevel {
		return LEVELS.indexOf(a) >= LEVELS.indexOf(b) ? a : b
	}

	function entryLevel(grant: boolean | null | undefined): GrantLevel {
		return grant == undefined ? 'none' : grant ? 'write' : 'read'
	}

	/** The principal's own entry on an item, which `grant` on its row holds. */
	function isOwnEntry(r: AccessReason): boolean {
		return r.type === 'user' || (r.type === 'group' && `g/${r.group}` === principal)
	}

	function reasonLabel(r: AccessReason): string {
		switch (r.type) {
			case 'folder':
				return `through f/${r.folder}`
			case 'group':
				return `through g/${r.group}`
			case 'own':
				return `under ${principal}/`
			case 'group_path':
				return `under g/${r.group}/`
			default:
				return 'shared directly'
		}
	}

	function inheritedOf(row: Leaf, key: string): Inherited {
		if (row.kind === 'folder') {
			if (row.owner) return { level: 'write', hows: ['folder admin'] }
			const folder = reachedFolders.get(row.path)
			const others = (folder?.via ?? []).filter((v) => v !== principal)
			if (!folder || others.length === 0) return { level: 'none', hows: [] }
			// `via` names the other entries without their levels, so next to the principal's own
			// write the report cannot tell whether one of them gives write too.
			const level = folder.grant === true ? undefined : folder.write ? 'write' : 'read'
			return { level, hows: others.map((v) => `through ${v}`) }
		}
		const others = (reasons.get(key) ?? []).filter((r) => !isOwnEntry(r))
		const level: GrantLevel =
			others.length === 0
				? 'none'
				: others.some((r) => r.type === 'own' || r.type === 'group_path' || r.write)
					? 'write'
					: 'read'
		return { level, hows: others.map(reasonLabel) }
	}

	/** The level held on a row. The own entry is read from the row, which a change made here
	 * updates before `access` reloads. */
	function levelOf(row: Leaf, key: string): GrantLevel {
		return maxLevel(inheritedOf(row, key).level ?? 'read', entryLevel(row.grant))
	}

	function pathOf(row: Leaf): string {
		return row.kind === 'folder' ? `f/${row.path}` : row.path
	}

	/** Why a dimmed row can't be changed, for its tooltip. */
	function lockedText(row: Leaf): string | undefined {
		if (row.owner)
			return `${principal} is an admin of f/${row.path}, so ${isGroup ? 'its members' : 'they'} can write every item in it.`
		return row.blocked
	}

	/** Selected on a highlighted row, the first of these it offers. */
	const PREFERRED = ['Grant read', 'Grant write', 'Revoke write', 'Revoke read']

	function actionsFor(leaf: DrillLeaf<Leaf>): DrillLeafAction[] {
		const row = leaf.data
		const ownLevel = entryLevel(row.grant)
		const inherited = inheritedOf(row, leaf.key)
		const level = maxLevel(inherited.level ?? 'read', ownLevel)
		const gives = GIVES[row.kind] ?? GIVES_OTHER
		// In the order of the level each one leaves, so the buttons read from less to more access.
		const changes: { label: string; to: GrantLevel; hint: string }[] = []
		if (ownLevel !== 'none' && inherited.level === 'none') {
			changes.push({
				label: 'Revoke read',
				to: 'none',
				hint: `${who} can no longer ${gives.read}.`
			})
		}
		if (level === 'none') {
			changes.push({ label: 'Grant read', to: 'read', hint: `${who} can ${gives.read}.` })
		}
		if (ownLevel === 'write' && inherited.level !== 'write') {
			changes.push({
				label: 'Revoke write',
				to: 'read',
				hint: inherited.level
					? `${who} can no longer ${gives.write}, only ${gives.read}.`
					: `the grant becomes read, but ${who} may still write ${inherited.hows.join(' or ')}.`
			})
		}
		if (level !== 'write') {
			const folder =
				row.kind !== 'folder' && ownLevel === 'none'
					? (reasons.get(leaf.key) ?? []).find((r) => r.type === 'folder')?.folder
					: undefined
			changes.push({
				label: 'Grant write',
				to: 'write',
				hint:
					level === 'none'
						? `${who} can ${gives.read}, and ${gives.write}.`
						: `${who} can also ${gives.write}.` +
							(folder ? ` This grant stays if f/${folder} is no longer shared.` : '')
			})
		}
		const preferred = PREFERRED.find((label) => changes.some((c) => c.label === label))
		return changes.map((c) => ({
			label: c.label,
			hint: c.hint,
			preferred: c.label === preferred,
			loading: pending === `${leaf.key}:${c.label}`,
			disabled: pending !== undefined,
			run: () => change(leaf, c.label, c.to)
		}))
	}

	/** A change to a whole folder, waiting for the dialog. */
	let confirming:
		| { leaf: DrillLeaf<Leaf>; label: string; level: GrantLevel; hint: string }
		| undefined = $state()

	function change(leaf: DrillLeaf<Leaf>, label: string, level: GrantLevel) {
		if (pending !== undefined || Date.now() < settledUntil) return
		if (leaf.data.kind !== 'folder') {
			apply(leaf, label, level)
			return
		}
		const hint = actionsFor(leaf).find((a) => a.label === label)?.hint ?? ''
		confirming = { leaf, label, level, hint }
		// Else the second press of a double click or Enter would confirm it at once.
		settledUntil = Date.now() + 400
	}

	async function confirmFolder() {
		if (!confirming || pending !== undefined || Date.now() < settledUntil) return
		await apply(confirming.leaf, confirming.label, confirming.level)
		confirming = undefined
	}

	function cancelFolder() {
		if (pending !== undefined) return
		confirming = undefined
		picker?.focus()
	}

	/** Focus stays in the search field under the dialog, so its keys are answered here: the
	 * dialog's own window listener leaves alone keys that start inside a popover, which is where
	 * the access drawer shows this list. */
	function dialogKeys(e: KeyboardEvent) {
		if (!confirming || e.key === 'Tab') return
		e.stopPropagation()
		// A focused dialog button activates itself, so Enter on Cancel cancels.
		const onButton = !!(e.target as HTMLElement | null)?.closest?.('[role="dialog"] button')
		if (onButton && (e.key === 'Enter' || e.key === ' ')) return
		e.preventDefault()
		if (e.key === 'Enter') confirmFolder()
		else if (e.key === 'Escape') cancelFolder()
	}

	/** Answers an Escape a host takes before the navigator can: cancels the folder dialog, or
	 * passes it to the picker. False when nothing here was open, so the host closes instead. */
	export function escape(target: EventTarget | null): boolean {
		if (confirming) {
			cancelFolder()
			return true
		}
		return !!picker?.escape(target)
	}

	let root: HTMLElement | undefined = $state()
	// The dialog covers this list rather than the page: a click on a dialog over the page lands
	// outside the drawer's popover, which closes it and the dialog with it.
	setOverlayHost({ el: () => root, drawers: { val: [] }, active: () => true })

	const DONE: Record<string, string> = {
		'Grant read': 'Read granted',
		'Grant write': 'Write granted',
		'Revoke write': 'Write revoked',
		'Revoke read': 'Access revoked'
	}

	/** The row a change just landed on, and what it did ("Read granted"), for 1.5s. */
	let done: { key: string; text: string } | undefined = $state()
	let doneTimer: ReturnType<typeof setTimeout> | undefined

	function showDone(key: string, text: string) {
		clearTimeout(doneTimer)
		done = { key, text }
		doneTimer = setTimeout(() => (done = undefined), 1000)
	}

	async function apply(leaf: DrillLeaf<Leaf>, label: string, level: GrantLevel) {
		const row = leaf.data
		pending = `${leaf.key}:${label}`
		try {
			await data.set(row.kind, row.path, level)
			row.grant = level === 'none' ? null : level === 'write'
			if (row.kind === 'folder') {
				const folder = folders?.find((f) => f.name === row.path)
				if (folder) folder.grant = row.grant
			}
			onChange()
			showDone(leaf.key, DONE[label] ?? 'Done')
		} catch (e) {
			sendUserToast(`Could not ${label.toLowerCase()} on ${pathOf(row)}: ${e?.body ?? e}`, true)
		} finally {
			pending = undefined
			settledUntil = Date.now() + 400
			// A click left focus on a button, which may be gone now.
			picker?.focus()
		}
	}
</script>

{#snippet leafIcon(leaf: DrillLeaf<Leaf>)}
	<RowIcon kind={leaf.data.kind as 'folder'} size={12} />
{/snippet}

<div
	bind:this={root}
	class="relative flex flex-col max-h-[min(460px,60vh)]"
	onkeydowncapture={dialogKeys}
	role="presentation"
>
	{#if confirming}
		{@const folder = pathOf(confirming.leaf.data)}
		<ConfirmationModal
			open
			title="{confirming.label} on {folder}?"
			confirmationText={confirming.label}
			type={confirming.label.startsWith('Revoke') ? 'danger' : 'info'}
			loading={pending !== undefined}
			onConfirmed={confirmFolder}
			onCanceled={cancelFolder}
		>
			<p>It applies to every item in {folder}: {confirming.hint}</p>
		</ConfirmationModal>
	{/if}
	<DrillPicker
		bind:this={picker}
		{tree}
		{leafIcon}
		rowTooltip={(leaf) => lockedText(leaf.data)}
		rootLoading={!folders}
		onScopeChange={(scope) => scope.length === 1 && loadBranch(scope[0])}
		onFilterChange={(filter) => filter.trim() && searchSoon(filter.trim())}
		leafActions={actionsFor}
		leafDone={(leaf) => (done && done.key === leaf.key ? done.text : undefined)}
		leafBadge={(leaf) => {
			const level = levelOf(leaf.data, leaf.key)
			return level === 'none'
				? undefined
				: { label: LEVEL_LABELS[level], color: level === 'write' ? 'blue' : 'gray' }
		}}
	/>
</div>
