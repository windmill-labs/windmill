<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { GranularAclService, type BrowseAccessFoldersResponse } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { debounce } from '$lib/utils'
	import { Folder, Lock, User } from 'lucide-svelte'
	import { KIND_LABELS, setGrant, type GrantLevel } from './access'

	type Row = { kind: string; path: string; grant?: boolean | null; blocked?: string }

	let { principal, onChanged }: { principal: string; onChanged: () => void } = $props()

	let folders: BrowseAccessFoldersResponse | undefined = $state()
	/** `undefined` browses every path the caller can read. */
	let prefix: string | undefined = $state()
	let kind: string | undefined = $state()
	let search = $state('')
	let items: Row[] = $state([])
	let hasMore = $state(false)
	let loading = $state(false)
	let pending: Record<string, boolean> = $state({})
	// Responses are not ordered: an answer for an earlier filter must not replace a later one.
	let requestId = 0

	const ownPrefix = $derived(`u/${$userStore?.username}/`)
	const selectedFolder = $derived(
		prefix?.startsWith('f/') ? folders?.find((f) => `f/${f.name}/` === prefix) : undefined
	)
	const kindItems = Object.entries(KIND_LABELS).map(([value, label]) => ({ value, label }))

	async function load(): Promise<void> {
		void loadItems(true)
		try {
			folders = await GranularAclService.browseAccessFolders({
				workspace: $workspaceStore!,
				principal
			})
		} catch (e) {
			sendUserToast(`Could not load folders: ${e?.body ?? e}`, true)
		}
	}

	async function loadItems(reset: boolean): Promise<void> {
		const id = ++requestId
		loading = true
		try {
			const page = await GranularAclService.browseAccessItems({
				workspace: $workspaceStore!,
				principal,
				prefix,
				kind,
				search: search.trim() || undefined,
				offset: reset ? 0 : items.length
			})
			if (id !== requestId) return
			items = reset ? page.items : [...items, ...page.items]
			hasMore = page.has_more
		} catch (e) {
			if (id === requestId) sendUserToast(`Could not load items: ${e?.body ?? e}`, true)
		} finally {
			if (id === requestId) loading = false
		}
	}

	const reloadSoon = debounce(() => loadItems(true), 250).debounced

	load()

	function selectScope(next: string | undefined) {
		prefix = next
		void loadItems(true)
	}

	function level(grant: boolean | null | undefined): GrantLevel {
		return grant == undefined ? 'none' : grant ? 'write' : 'read'
	}

	async function change(
		rowKind: string,
		path: string,
		next: GrantLevel,
		apply: (g: boolean | null) => void
	) {
		const key = `${rowKind}:${path}`
		pending[key] = true
		try {
			await setGrant($workspaceStore!, rowKind, path, principal, next)
			apply(next === 'none' ? null : next === 'write')
			onChanged()
		} catch (e) {
			sendUserToast(`Could not change access to ${path}: ${e?.body ?? e}`, true)
		} finally {
			delete pending[key]
		}
	}
</script>

{#snippet levelPicker(
	value: GrantLevel,
	blocked: string | undefined,
	busy: boolean,
	onSelect: (v: GrantLevel) => void
)}
	<div class="flex items-center gap-1" title={blocked}>
		{#if blocked}
			<Lock size={12} class="text-secondary shrink-0" aria-label={blocked} />
		{/if}
		<ToggleButtonGroup
			selected={value}
			disabled={!!blocked || busy}
			noWFull
			onSelected={(v) => v !== value && onSelect(v)}
		>
			{#snippet children({ item })}
				<ToggleButton value="none" label="None" small {item} />
				<ToggleButton value="read" label="Read" small {item} />
				<ToggleButton value="write" label="Write" small {item} />
			{/snippet}
		</ToggleButtonGroup>
	</div>
{/snippet}

<div class="flex gap-4 min-h-[400px]">
	<nav class="w-56 shrink-0 flex flex-col gap-0.5 text-xs" aria-label="Where to browse">
		<button
			class="text-left px-2 py-1.5 rounded-md hover:bg-surface-hover {prefix === undefined
				? 'bg-surface-selected font-medium text-emphasis'
				: 'text-primary'}"
			onclick={() => selectScope(undefined)}
		>
			Everything you can read
		</button>
		<button
			class="text-left px-2 py-1.5 rounded-md hover:bg-surface-hover flex items-center gap-1.5 {prefix ===
			ownPrefix
				? 'bg-surface-selected font-medium text-emphasis'
				: 'text-primary'}"
			onclick={() => selectScope(ownPrefix)}
		>
			<User size={12} />
			<span class="truncate">{ownPrefix}</span>
		</button>
		<div class="px-2 pt-3 pb-1 text-2xs font-semibold text-secondary">Folders</div>
		{#each folders ?? [] as folder (folder.name)}
			<button
				class="text-left px-2 py-1.5 rounded-md hover:bg-surface-hover flex items-center gap-1.5 {prefix ===
				`f/${folder.name}/`
					? 'bg-surface-selected font-medium text-emphasis'
					: 'text-primary'}"
				onclick={() => selectScope(`f/${folder.name}/`)}
			>
				<Folder size={12} />
				<span class="truncate grow">f/{folder.name}</span>
				{#if folder.target_owner}
					<span class="text-2xs text-secondary">owner</span>
				{:else if folder.grant != undefined}
					<span class="text-2xs text-secondary">{folder.grant ? 'write' : 'read'}</span>
				{/if}
			</button>
		{:else}
			<p class="px-2 text-2xs text-secondary">{folders ? 'No folder you can read.' : 'Loading…'}</p>
		{/each}
	</nav>

	<div class="grow min-w-0 flex flex-col gap-3">
		{#if selectedFolder}
			{@const folder = selectedFolder}
			<div class="flex items-center justify-between gap-4 rounded-md border px-3 py-2">
				<div class="flex flex-col">
					<span class="text-xs font-medium text-emphasis">The whole folder f/{folder.name}</span>
					<span class="text-2xs text-secondary">
						{folder.target_owner
							? `${principal} owns this folder and always keeps write.`
							: 'Covers every item in it, including ones added later.'}
					</span>
				</div>
				{@render levelPicker(
					folder.target_owner ? 'write' : level(folder.grant),
					folder.target_owner ? `${principal} owns f/${folder.name}` : folder.blocked,
					!!pending[`folder:${folder.name}`],
					(v) => change('folder', folder.name, v, (g) => (folder.grant = g))
				)}
			</div>
		{/if}

		<div class="flex gap-2">
			<TextInput
				bind:value={() => search, (v) => ((search = String(v ?? '')), reloadSoon())}
				inputProps={{ placeholder: 'Search paths' }}
				size="sm"
				class="grow"
			/>
			<div class="w-48">
				<Select
					items={kindItems}
					bind:value={() => kind, (v) => ((kind = v), loadItems(true))}
					placeholder="All kinds"
					clearable
					size="sm"
				/>
			</div>
		</div>

		<p class="text-2xs text-secondary">
			Only {principal}'s own grants are shown. It may also reach an item through a group or a
			folder: the Access tab lists those.
		</p>

		<div class="flex flex-col divide-y border rounded-md">
			{#each items as item (item.kind + ':' + item.path)}
				<div class="flex items-center justify-between gap-4 px-3 py-1.5">
					<div class="flex flex-col min-w-0">
						<span class="font-mono text-2xs truncate text-emphasis">{item.path}</span>
						<span class="text-2xs text-secondary">{KIND_LABELS[item.kind] ?? item.kind}</span>
					</div>
					{@render levelPicker(
						level(item.grant),
						item.blocked,
						!!pending[`${item.kind}:${item.path}`],
						(v) => change(item.kind, item.path, v, (g) => (item.grant = g))
					)}
				</div>
			{:else}
				<p class="px-3 py-4 text-xs text-secondary">
					{loading ? 'Loading…' : 'No item matches.'}
				</p>
			{/each}
		</div>
		{#if hasMore}
			<Button variant="subtle" unifiedSize="sm" {loading} onClick={() => loadItems(false)}>
				Load more
			</Button>
		{/if}
	</div>
</div>
