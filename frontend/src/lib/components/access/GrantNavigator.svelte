<!--
@component
The access view's "+ Grant" list: the caller's own `u/` folder and every folder, with their items.
Picking a row grants the principal read on it; the "All of f/x" row grants the whole folder.
-->
<script module lang="ts">
	/** The navigator's tallest height, `max-h-[min(420px,60vh)]` below, for its host to place it. */
	export function navigatorMaxHeight(): number {
		return Math.min(420, window.innerHeight * 0.6)
	}
</script>

<script lang="ts">
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import {
		GranularAclService,
		type BrowseAccessFoldersResponse,
		type BrowseAccessItemsResponse,
		type PrincipalAccess
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { debounce } from '$lib/utils'
	import { Folder, ScanSearch, User, Users } from 'lucide-svelte'
	import { setGrant } from './access'
	import type { RunnableKind } from './detectNeeds'

	type Row = BrowseAccessItemsResponse['items'][number]
	/** `kind` `folder` is the "All of f/x" row, its `path` the folder name. */
	type Leaf = Row & { owner?: boolean }

	let {
		principal,
		access,
		onGranted,
		onFindReads
	}: {
		principal: string
		access: PrincipalAccess
		onGranted: () => void
		onFindReads: (kind: RunnableKind, path: string) => void
	} = $props()

	const own = $derived(`u/${$userStore?.username}`)

	let folders: BrowseAccessFoldersResponse | undefined = $state()
	/** Rows by top-level branch (`u/alice`, `f/billing`), then by `kind:path`. */
	let rows: Record<string, Record<string, Row>> = $state({})
	let loading: Record<string, boolean> = $state({})
	const loaded = new Set<string>()

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
			folders = await GranularAclService.browseAccessFolders({
				workspace: $workspaceStore!,
				principal
			})
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
				const page = await GranularAclService.browseAccessItems({
					workspace: $workspaceStore!,
					principal,
					prefix: `${branch}/`,
					offset
				})
				merge(page.items)
				if (!page.has_more) break
				offset += page.items.length
			}
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
			const page = await GranularAclService.browseAccessItems({
				workspace: $workspaceStore!,
				principal,
				search
			})
			merge(page.items)
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
			...(folders ?? []).map(
				(folder): DrillNode<Leaf> => ({
					type: 'branch',
					key: `f/${folder.name}`,
					label: `f/${folder.name}`,
					icon: Folder,
					searchGroup: true,
					loading: loading[`f/${folder.name}`],
					children: [
						{
							type: 'leaf',
							key: `folder:${folder.name}`,
							label: `All of f/${folder.name}`,
							searchableText: `f/${folder.name}`,
							data: {
								kind: 'folder',
								path: folder.name,
								grant: folder.grant,
								blocked: folder.blocked,
								owner: folder.target_owner
							},
							disabled: folder.target_owner || !!folder.blocked
						},
						...itemLeaves(`f/${folder.name}`)
					]
				})
			),
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

	const reaches = $derived(
		new Map([
			...access.items.map((i) => [`${i.kind}:${i.path}`, i.write] as const),
			...access.folders.map((f) => [`folder:${f.name}`, f.write] as const)
		])
	)

	async function grant(leaf: DrillLeaf<Leaf>) {
		const row = leaf.data
		if (row.grant != undefined) return
		try {
			await setGrant($workspaceStore!, row.kind, row.path, principal, 'read')
			row.grant = false
			if (row.kind === 'folder') {
				const folder = folders?.find((f) => f.name === row.path)
				if (folder) folder.grant = false
			}
			onGranted()
		} catch (e) {
			const what = row.kind === 'folder' ? `f/${row.path}` : row.path
			sendUserToast(`Could not grant ${what}: ${e?.body ?? e}`, true)
		}
	}

	function tooltip(leaf: DrillLeaf<Leaf>): string | undefined {
		const row = leaf.data
		if (row.owner) return `${principal} owns f/${row.path} and always keeps write`
		if (row.blocked) return row.blocked
		if (row.grant != undefined)
			return `Already granted ${row.grant ? 'write' : 'read'}: change it from the table`
		return undefined
	}

	function isRunnable(kind: string): kind is RunnableKind {
		return kind === 'flow' || kind === 'script' || kind === 'app'
	}
</script>

{#snippet leafIcon(leaf: DrillLeaf<Leaf>)}
	<RowIcon kind={leaf.data.kind as 'folder'} size={12} />
{/snippet}

<div class="flex flex-col max-h-[min(420px,60vh)]">
	<DrillPicker
		{tree}
		onPick={grant}
		{leafIcon}
		rootLoading={!folders}
		onScopeChange={(scope) => scope.length === 1 && loadBranch(scope[0])}
		onFilterChange={(filter) => filter.trim() && searchSoon(filter.trim())}
		rowTooltip={tooltip}
		leafBadge={(leaf) => {
			const write = reaches.get(leaf.key)
			return write === undefined
				? undefined
				: { label: write ? 'Writes' : 'Reads', color: write ? 'blue' : 'gray' }
		}}
		leafAction={(leaf) =>
			isRunnable(leaf.data.kind)
				? {
						icon: ScanSearch,
						label: 'What it reads',
						title: `Grant what ${leaf.data.path} reads when it runs`,
						run: () => onFindReads(leaf.data.kind as RunnableKind, leaf.data.path)
					}
				: undefined}
	/>
</div>
