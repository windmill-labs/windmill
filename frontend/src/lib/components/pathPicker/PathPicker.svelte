<!--
@component
Select-like picker for workspace paths: items (`f/team/report`) and folders (`f/team/*`).
The dropdown browses folders and searches every path; a typed path that matches nothing
is offered as is, so a pattern needs no second input.
-->
<script lang="ts">
	import {
		ChevronDown,
		ChevronLeft,
		ChevronRight,
		Check,
		Folder,
		Loader2,
		Plus,
		Search,
		User,
		X
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { resource } from 'runed'
	import { FolderService } from '$lib/gen'
	import { clickOutside } from '$lib/utils'
	import GenericDropdown from '../select/GenericDropdown.svelte'
	import RowIcon from '../common/table/RowIcon.svelte'
	import { inputBaseClass, inputBorderClass } from '../text_input/TextInput.svelte'
	import { useWorkspaceItemsLoader } from '../workspaceItemsLoader.svelte'
	import {
		dirPathFromKey,
		workspaceItemDisplayPath,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '../workspacePicker'
	import { buildWorkspaceTree } from '../workspaceTree'
	import {
		collectLeavesGrouped,
		leafHaystack,
		type DrillBranch,
		type DrillLeaf,
		type DrillNode
	} from '../drillPicker'
	import { useOperatingWorkspace } from '../operatingWorkspace.svelte'
	import { overlayStack } from '../common/overlayHost.svelte'
	import { randomUUID } from '$lib/utils/uuid'
	import { onDestroy } from 'svelte'

	interface Props {
		value: string[]
		kinds?: WorkspaceItemKind[]
		/** Several paths (chips in the field) or one. */
		multiple?: boolean
		size?: 'sm' | 'md'
		/** Type in the field itself, like `Select`, or in a search row atop the dropdown. */
		searchIn?: 'field' | 'dropdown'
		/** Browse one folder at a time, or list every folder and item at once. */
		navigation?: 'drill' | 'flat'
		placeholder?: string
		/** Error text for a typed path, or null when it is valid. */
		validate?: (path: string) => string | null
		disabled?: boolean
		class?: string
		workspaceId?: string
	}

	let {
		value = $bindable(),
		kinds = ['script', 'flow'],
		multiple = true,
		size = 'sm',
		searchIn = 'field',
		navigation = 'drill',
		placeholder = 'All paths',
		validate,
		disabled = false,
		class: className = '',
		workspaceId
	}: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const workspace = $derived(workspaceId ?? $operatingWorkspace)
	const loader = useWorkspaceItemsLoader(
		() => workspace,
		() => kinds
	)
	const folderNames = resource(
		() => workspace,
		async (ws) => (ws ? await FolderService.listFolderNames({ workspace: ws }) : [])
	)

	let open = $state(false)
	// While open, the picker sits on the overlay stack, so an enclosing Modal or Drawer leaves
	// Escape to it instead of closing itself (and discarding the form) under the list.
	const overlays = overlayStack()
	const overlayId = `path-picker-${randomUUID()}`
	function leaveOverlayStack() {
		overlays.val = overlays.val.filter((id) => id !== overlayId)
	}
	onDestroy(leaveOverlayStack)
	let query = $state('')
	let cwd = $state('')
	let highlighted = $state(0)
	let triggerEl: HTMLDivElement | undefined = $state()
	let fieldInput: HTMLInputElement | undefined = $state()

	// The context picker's tree: folders and items placed by display path, so a draft shows
	// under its friendly path rather than its `draft_<uuid>` storage path.
	const tree = $derived(
		buildWorkspaceTree({
			loaded: loader.loaded,
			kinds,
			loadingKind: loader.loadingKind,
			layout: 'flat'
		})
	)
	const branchesByPath = $derived.by(() => {
		const byPath = new Map<string, DrillBranch<WorkspaceItem>>()
		const walk = (nodes: DrillNode<WorkspaceItem>[]) => {
			for (const n of nodes) {
				if (n.type !== 'branch') continue
				const path = dirPathFromKey(n.key)
				if (path) byPath.set(path, n)
				walk(n.children)
			}
		}
		walk(tree)
		return byPath
	})
	// Folders holding no item of these kinds yet are still valid restrictions.
	const emptyFolders = $derived(
		(folderNames.current ?? []).map((n) => `f/${n}`).filter((p) => !branchesByPath.has(p))
	)
	const dirs = $derived([...branchesByPath.keys(), ...emptyFolders])
	const leaves = $derived(collectLeavesGrouped(tree).map(({ leaf }) => leaf))
	const loading = $derived(leaves.length === 0 && kinds.some((k) => loader.loadingKind[k]))

	/** `f/team/sub/x` → `f/team/sub`; a top-level `f/team` → ''. */
	function parentOf(path: string): string {
		const parts = path.split('/')
		return parts.length <= 2 ? '' : parts.slice(0, -1).join('/')
	}
	function lastSegment(path: string): string {
		return path.slice(path.lastIndexOf('/') + 1)
	}

	type Row =
		| { type: 'up'; key: string; dir: string }
		| { type: 'folder-all'; key: string; dir: string; path: string }
		| { type: 'folder'; key: string; dir: string; path: string; drill: boolean }
		| { type: 'item'; key: string; item: WorkspaceItem; path: string }
		| { type: 'custom'; key: string; path: string; error: string | null }

	const folderRow = (dir: string, drill: boolean): Row => ({
		type: 'folder',
		key: `folder:${dir}`,
		dir,
		path: `${dir}/*`,
		drill
	})
	const itemRow = (leaf: DrillLeaf<WorkspaceItem>): Row => ({
		type: 'item',
		key: leaf.key,
		item: leaf.data,
		path: workspaceItemDisplayPath(leaf.data)
	})

	const rows = $derived.by<Row[]>(() => {
		const q = query.trim().toLowerCase()
		if (q) {
			const out: Row[] = [
				...dirs.filter((d) => `${d}/*`.toLowerCase().includes(q)).map((d) => folderRow(d, false)),
				...leaves.filter((l) => leafHaystack(l).toLowerCase().includes(q)).map(itemRow)
			]
			const typed = query.trim()
			if (!out.some((r) => 'path' in r && r.path === typed)) {
				out.push({ type: 'custom', key: 'custom', path: typed, error: validate?.(typed) ?? null })
			}
			return out
		}
		if (navigation === 'flat') {
			return [...dirs.map((d) => folderRow(d, false)), ...leaves.map(itemRow)]
		}
		if (cwd === '') {
			return dirs
				.filter((d) => parentOf(d) === '')
				.sort(
					(a, b) => Number(b.startsWith('u/')) - Number(a.startsWith('u/')) || a.localeCompare(b)
				)
				.map((d) => folderRow(d, true))
		}
		const children = branchesByPath.get(cwd)?.children ?? []
		return [
			{ type: 'up', key: 'up', dir: cwd },
			{ type: 'folder-all', key: 'all', dir: cwd, path: `${cwd}/*` },
			...children.map((n) =>
				n.type === 'branch' ? folderRow(dirPathFromKey(n.key) ?? n.label, true) : itemRow(n)
			)
		]
	})

	function openPicker() {
		if (disabled || open) return
		open = true
		overlays.val.push(overlayId)
		loader.ensureAll()
		highlighted = 0
	}

	function close() {
		open = false
		leaveOverlayStack()
		query = ''
		cwd = ''
	}

	function setQuery(q: string) {
		query = q
		highlighted = 0
		if (!open) openPicker()
	}

	function goTo(dir: string) {
		cwd = dir
		// Inside a folder, land on "Everything in" rather than the row going back up.
		highlighted = dir === '' ? 0 : 1
	}

	function pick(path: string) {
		if (multiple) {
			value = value.includes(path) ? value.filter((p) => p !== path) : [...value, path]
			query = ''
			highlighted = Math.min(highlighted, Math.max(0, rows.length - 1))
		} else {
			value = [path]
			close()
		}
	}

	function activate(row: Row | undefined) {
		if (!row) return
		switch (row.type) {
			case 'up':
				return goTo(parentOf(row.dir))
			case 'folder':
				return row.drill ? goTo(row.dir) : pick(row.path)
			case 'custom':
				return row.error ? undefined : pick(row.path)
			default:
				return pick(row.path)
		}
	}

	function onKeydown(e: KeyboardEvent) {
		const row = rows[highlighted]
		if (e.key === 'ArrowDown') {
			openPicker()
			highlighted = Math.min(rows.length - 1, highlighted + 1)
		} else if (e.key === 'ArrowUp') {
			highlighted = Math.max(0, highlighted - 1)
		} else if (e.key === 'Enter') {
			if (!open) openPicker()
			else activate(row)
		} else if (e.key === 'ArrowRight' && query === '' && row?.type === 'folder' && row.drill) {
			goTo(row.dir)
		} else if ((e.key === 'ArrowLeft' || e.key === 'Backspace') && query === '' && cwd !== '') {
			goTo(parentOf(cwd))
		} else if (e.key === 'Backspace' && query === '' && multiple && value.length > 0) {
			value = value.slice(0, -1)
		} else if (e.key === 'Escape' && open) {
			close()
		} else return
		e.preventDefault()
		e.stopPropagation()
		requestAnimationFrame(() =>
			document
				.querySelector(`[data-path-picker-row="${highlighted}"]`)
				?.scrollIntoView({ block: 'nearest' })
		)
	}

	function focusOnMount(el: HTMLElement) {
		requestAnimationFrame(() => el.focus())
	}

	const heightClass = $derived(size === 'sm' ? 'min-h-7' : 'min-h-8')
	const isSelected = (path: string) => value.includes(path)
</script>

{#snippet chip(path: string)}
	<span
		class="inline-flex items-center gap-0.5 h-5 max-w-full pl-1.5 pr-0.5 rounded bg-surface-secondary border border-border-light font-mono text-2xs text-emphasis"
	>
		<span class="truncate">{path}</span>
		<button
			type="button"
			class="shrink-0 p-0.5 rounded text-secondary hover:text-primary hover:bg-surface-hover"
			title="Remove {path}"
			{disabled}
			onpointerdown={(e) => e.stopPropagation()}
			onclick={(e) => {
				e.stopPropagation()
				value = value.filter((p) => p !== path)
			}}
		>
			<X size={10} />
		</button>
	</span>
{/snippet}

{#snippet selectedMark(path: string)}
	{#if isSelected(path)}
		<Check size={14} class="shrink-0 text-accent" />
	{/if}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class={twMerge('relative', className)} use:clickOutside={{ onClickOutside: close }}>
	<div
		bind:this={triggerEl}
		class={twMerge(
			inputBaseClass,
			inputBorderClass({ forceFocus: open }),
			heightClass,
			'flex flex-wrap items-center gap-1 pl-1.5 pr-7 py-0.5',
			disabled ? '!bg-surface-disabled cursor-not-allowed' : 'cursor-pointer'
		)}
		onclick={() => {
			openPicker()
			fieldInput?.focus()
		}}
		role={searchIn === 'dropdown' ? 'button' : undefined}
		tabindex={searchIn === 'dropdown' && !disabled ? 0 : undefined}
		onkeydown={searchIn === 'dropdown' ? onKeydown : undefined}
	>
		{#if multiple}
			{#each value as path, i (i)}
				{@render chip(path)}
			{/each}
		{/if}
		{#if searchIn === 'field'}
			<input
				bind:this={fieldInput}
				type="text"
				autocomplete="off"
				{disabled}
				value={query}
				oninput={(e) => setQuery(e.currentTarget.value)}
				onfocus={openPicker}
				onkeydown={onKeydown}
				placeholder={multiple ? (value.length > 0 ? '' : placeholder) : (value[0] ?? placeholder)}
				class={twMerge(
					'no-default-style flex-1 min-w-16 h-5 !p-0 !pl-0.5 !border-0 !bg-transparent !shadow-none !ring-0 outline-none text-xs',
					!multiple && value[0] && !open
						? 'placeholder:text-primary font-mono'
						: 'placeholder:text-hint'
				)}
			/>
		{:else if !multiple && value[0]}
			<span class="pl-0.5 font-mono text-xs text-primary truncate">{value[0]}</span>
		{:else if value.length === 0}
			<span class="pl-0.5 text-xs text-hint">{placeholder}</span>
		{/if}
		<ChevronDown
			size={14}
			class="absolute right-2 top-1/2 -translate-y-1/2 text-secondary pointer-events-none transition-transform {open
				? 'rotate-180'
				: ''}"
		/>
	</div>

	<GenericDropdown
		{open}
		{disabled}
		maxHeight={320}
		strictWidth
		getInputRect={triggerEl && (() => triggerEl!.getBoundingClientRect())}
	>
		{#if searchIn === 'dropdown'}
			<div class="flex items-center gap-2 px-3 h-8 border-b border-border-light shrink-0">
				<Search size={12} class="shrink-0 text-hint" />
				<input
					use:focusOnMount
					type="text"
					autocomplete="off"
					value={query}
					oninput={(e) => setQuery(e.currentTarget.value)}
					onkeydown={onKeydown}
					placeholder="Search or type a path"
					class="no-default-style flex-1 min-w-0 !p-0 !border-0 !bg-transparent !shadow-none !ring-0 outline-none text-xs placeholder:text-hint"
				/>
			</div>
		{/if}
		<ul class="flex-1 min-h-0 overflow-y-auto py-1" role="listbox">
			{#if loading}
				<li class="flex items-center gap-2 px-3 py-2 text-xs text-secondary">
					<Loader2 size={12} class="animate-spin" /> Loading
				</li>
			{/if}
			{#each rows as row, i (row.key)}
				{@const hl = i === highlighted}
				<li>
					<!-- Highlight on mousemove, not mouseenter: opening a folder swaps the rows under a
					     still pointer, which must not take the highlight Enter acts on. -->
					<button
						type="button"
						data-path-picker-row={i}
						role="option"
						aria-selected={hl}
						disabled={row.type === 'custom' && !!row.error}
						class={twMerge(
							'w-full flex items-center gap-2 px-3 py-1.5 text-left text-xs font-normal text-primary',
							hl ? 'bg-surface-secondary dark:bg-surface-tertiary' : 'hover:bg-surface-hover',
							row.type === 'up' ? 'text-secondary border-b border-border-light mb-1' : '',
							row.type === 'custom' && row.error ? 'cursor-not-allowed' : ''
						)}
						onmousedown={(e) => e.preventDefault()}
						onmousemove={() => (highlighted = i)}
						onclick={() => activate(row)}
					>
						{#if row.type === 'up'}
							<ChevronLeft size={12} class="shrink-0" />
							<span class="font-mono truncate">{row.dir}</span>
						{:else if row.type === 'folder-all'}
							<Folder size={12} class="shrink-0 text-secondary" />
							<span class="flex-1 truncate">
								Everything in <span class="font-mono">{lastSegment(row.dir)}</span>
							</span>
							<span class="font-mono text-2xs text-hint">{row.path}</span>
							{@render selectedMark(row.path)}
						{:else if row.type === 'folder'}
							{#if row.dir.startsWith('u/') && !row.dir.slice(2).includes('/')}
								<User size={12} class="shrink-0 text-secondary" />
							{:else}
								<Folder size={12} class="shrink-0 text-secondary" />
							{/if}
							<span class="flex-1 font-mono truncate">
								{row.drill ? (cwd === '' ? row.dir : lastSegment(row.dir)) : row.path}
							</span>
							{#if row.drill}
								<ChevronRight size={12} class="shrink-0 text-secondary" />
							{:else}
								{@render selectedMark(row.path)}
							{/if}
						{:else if row.type === 'item'}
							<RowIcon kind={row.item.kind} size={12} />
							<span class="font-mono truncate shrink-0 max-w-[60%]">
								{query || navigation === 'flat' ? row.path : lastSegment(row.path)}
							</span>
							<span class="flex-1 min-w-0 truncate text-hint">{row.item.summary}</span>
							{@render selectedMark(row.path)}
						{:else}
							<Plus size={12} class="shrink-0 text-secondary" />
							<span class="flex-1 min-w-0 flex flex-col">
								<span class="truncate">
									Use <span class="font-mono">{row.path}</span>
								</span>
								{#if row.error}
									<span class="text-2xs text-red-500">{row.error}</span>
								{/if}
							</span>
						{/if}
					</button>
				</li>
			{:else}
				{#if !loading}
					<li class="px-3 py-2 text-xs text-secondary">No folders or items</li>
				{/if}
			{/each}
		</ul>
	</GenericDropdown>
</div>
