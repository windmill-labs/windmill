<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { GoogleDriveFile, SharedDriveEntry } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import {
		Loader2,
		Folder,
		File,
		ChevronRight,
		Search,
		X,
		RefreshCw,
		Check
	} from 'lucide-svelte'
	import GoogleDriveIcon from '$lib/components/icons/GoogleDriveIcon.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Debounced, watch } from 'runed'
	import { untrack } from 'svelte'

	interface Props {
		resourceId: string
		resourceName: string
		disabled?: boolean
	}

	let {
		resourceId = $bindable(),
		resourceName = $bindable(),
		disabled = false
	}: Props = $props()

	type BreadcrumbItem = { id: string; name: string }
	type DriveTab = 'my_drive' | 'shared_with_me' | 'shared_drives'

	let files = $state<GoogleDriveFile[]>([])
	let sharedDrives = $state<SharedDriveEntry[]>([])
	let nextPageToken = $state<string | undefined>(undefined)
	let loadingFiles = $state(false)
	let loadingMore = $state(false)
	let searchQuery = $state('')
	let breadcrumbs = $state<BreadcrumbItem[]>([{ id: 'root', name: 'My Drive' }])
	let activeTab = $state<DriveTab>('my_drive')

	const currentParentId = $derived(breadcrumbs[breadcrumbs.length - 1]?.id ?? 'root')

	const debouncedSearch = new Debounced(() => searchQuery, 400)
	let loadVersion = 0

	async function loadFiles(pageToken?: string) {
		if (!$workspaceStore) return

		const version = ++loadVersion

		if (pageToken) {
			loadingMore = true
		} else {
			loadingFiles = true
			files = []
			nextPageToken = undefined
		}

		try {
			const params: Parameters<typeof NativeTriggerService.listGoogleDriveFiles>[0] = {
				workspace: $workspaceStore,
				pageToken
			}

			const query = searchQuery.trim()
			const parentId = currentParentId
			const tab = activeTab

			if (query) {
				params.q = query
			} else if (parentId !== 'root') {
				params.parentId = parentId
			} else if (tab === 'shared_with_me') {
				params.sharedWithMe = true
			}

			const response = await NativeTriggerService.listGoogleDriveFiles(params)

			if (version !== loadVersion) return // stale request, skip

			if (pageToken) {
				files = [...files, ...response.files]
			} else {
				files = response.files
			}
			nextPageToken = response.next_page_token
		} catch (err: any) {
			if (version !== loadVersion) return
			sendUserToast(`Failed to load Drive files: ${err.body || err.message}`, true)
		} finally {
			if (version === loadVersion) {
				loadingFiles = false
				loadingMore = false
			}
		}
	}

	async function loadSharedDrives() {
		if (!$workspaceStore) return

		loadingFiles = true
		files = []
		sharedDrives = []

		try {
			sharedDrives = await NativeTriggerService.listGoogleSharedDrives({
				workspace: $workspaceStore
			})
		} catch (err: any) {
			sendUserToast(`Failed to load shared drives: ${err.body || err.message}`, true)
		} finally {
			loadingFiles = false
		}
	}

	function navigateToFolder(folder: GoogleDriveFile | SharedDriveEntry) {
		searchQuery = ''
		breadcrumbs = [...breadcrumbs, { id: folder.id, name: folder.name }]
		loadFiles()
	}

	function navigateToBreadcrumb(index: number) {
		searchQuery = ''
		breadcrumbs = breadcrumbs.slice(0, index + 1)
		if (activeTab === 'shared_drives' && index === 0) {
			loadSharedDrives()
		} else {
			loadFiles()
		}
	}

	function selectItem(item: GoogleDriveFile) {
		resourceId = item.id
		resourceName = item.name
	}

	function clearSearch() {
		searchQuery = ''
		loadFiles()
	}

	function switchTab(tab: DriveTab) {
		activeTab = tab
		searchQuery = ''
		if (tab === 'my_drive') {
			breadcrumbs = [{ id: 'root', name: 'My Drive' }]
			loadFiles()
		} else if (tab === 'shared_with_me') {
			breadcrumbs = [{ id: 'root', name: 'Shared with me' }]
			loadFiles()
		} else {
			breadcrumbs = [{ id: 'root', name: 'Shared drives' }]
			loadSharedDrives()
		}
	}

	// Initial load when workspace is available
	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadFiles())
		}
	})

	// React to debounced search changes (skip initial)
	watch(
		() => debouncedSearch.current,
		() => { loadFiles() },
		{ lazy: true }
	)
</script>

<div class="flex flex-col gap-2 border rounded-md p-2 bg-surface">
	{#if resourceId}
		<div class="flex items-center gap-2 px-2 py-1 rounded bg-surface-selected text-secondary text-xs border">
			<GoogleDriveIcon width="14px" height="14px" />
			<span>
				Selected: <strong>{resourceName || resourceId}</strong>
			</span>
			<button
				class="ml-auto text-tertiary hover:text-secondary"
				onclick={() => { resourceId = ''; resourceName = '' }}
				{disabled}
			>
				<X size={14} />
			</button>
		</div>
	{:else}
		<div class="flex items-center gap-2 px-2 py-1 rounded text-tertiary text-xs border border-dashed">
			<GoogleDriveIcon width="14px" height="14px" />
			<span>No file selected</span>
		</div>
	{/if}

	<div class="flex items-center gap-1 text-2xs">
		<button
			class="px-2 py-0.5 rounded {activeTab === 'my_drive' ? 'bg-surface-selected font-semibold' : 'hover:bg-surface-hover'}"
			onclick={() => switchTab('my_drive')}
			{disabled}
		>
			My Drive
		</button>
		<button
			class="px-2 py-0.5 rounded {activeTab === 'shared_with_me' ? 'bg-surface-selected font-semibold' : 'hover:bg-surface-hover'}"
			onclick={() => switchTab('shared_with_me')}
			{disabled}
		>
			Shared with me
		</button>
		<button
			class="px-2 py-0.5 rounded {activeTab === 'shared_drives' ? 'bg-surface-selected font-semibold' : 'hover:bg-surface-hover'}"
			onclick={() => switchTab('shared_drives')}
			{disabled}
		>
			Shared drives
		</button>
		<div class="flex-1"></div>
		<button
			class="p-1 text-tertiary hover:text-secondary"
			title="Refresh"
			onclick={() => activeTab === 'shared_drives' && currentParentId === 'root' ? loadSharedDrives() : loadFiles()}
			{disabled}
		>
			<RefreshCw size={12} />
		</button>
	</div>

	{#if activeTab !== 'shared_drives' || currentParentId !== 'root'}
		<div class="relative">
			<TextInput
				bind:value={searchQuery}
				inputProps={{ placeholder: 'Search files...', disabled }}
				size="xs"
				class="!pl-7"
			/>
			<Search size={14} class="absolute left-2 top-1/2 -translate-y-1/2 text-tertiary pointer-events-none" />
			{#if searchQuery}
				<button
					class="absolute right-2 top-1/2 -translate-y-1/2 text-tertiary hover:text-secondary"
					onclick={clearSearch}
				>
					<X size={14} />
				</button>
			{/if}
		</div>
	{/if}

	{#if !searchQuery}
		<div class="flex items-center gap-0.5 text-2xs text-tertiary overflow-x-auto">
			{#each breadcrumbs as crumb, i (crumb.id + '-' + i)}
				{#if i > 0}
					<ChevronRight size={10} class="shrink-0" />
				{/if}
				{#if i < breadcrumbs.length - 1}
					<button
						class="hover:text-secondary hover:underline shrink-0"
						onclick={() => navigateToBreadcrumb(i)}
					>
						{crumb.name}
					</button>
				{:else}
					<span class="text-secondary font-medium shrink-0">{crumb.name}</span>
				{/if}
			{/each}
		</div>
	{/if}

	<div class="max-h-64 overflow-y-auto -mx-2 -mb-2 px-2 pb-1">
		{#if loadingFiles}
			<div class="flex items-center justify-center gap-2 py-4 text-xs text-tertiary">
				<Loader2 class="animate-spin" size={14} />
				Loading...
			</div>
		{:else if activeTab === 'shared_drives' && currentParentId === 'root'}
			{#if sharedDrives.length === 0}
				<div class="text-center py-4 text-xs text-tertiary">
					No shared drives found
				</div>
			{:else}
				{#each sharedDrives as drive (drive.id)}
					<div
						class="flex items-center gap-2 px-2 py-1.5 text-xs hover:bg-surface-hover group border-b border-light last:border-b-0"
					>
						<Folder size={14} class="text-blue-500 shrink-0" />
						<button
							class="flex-1 text-left truncate hover:underline"
							title="Open {drive.name}"
							onclick={() => navigateToFolder(drive)}
							{disabled}
						>
							{drive.name}
						</button>
						<ChevronRight size={14} class="text-tertiary shrink-0" />
					</div>
				{/each}
			{/if}
		{:else if files.length === 0}
			<div class="text-center py-4 text-xs text-tertiary">
				{searchQuery ? 'No files match your search' : 'This folder is empty'}
			</div>
		{:else}
			{#each files as file (file.id)}
				{#if file.is_folder}
					<div
						class="flex items-center gap-2 px-2 py-1.5 text-xs hover:bg-surface-hover group border-b border-light last:border-b-0"
					>
						<Folder size={14} class="text-blue-500 shrink-0" />
						<button
							class="flex-1 text-left truncate hover:underline"
							onclick={() => navigateToFolder(file)}
							title="Open {file.name}"
							{disabled}
						>
							{file.name}
						</button>
						<ChevronRight size={14} class="text-tertiary shrink-0" />
					</div>
				{:else}
					<div
						class="flex items-center gap-2 px-2 py-1.5 text-xs hover:bg-surface-hover group border-b border-light last:border-b-0
							{file.id === resourceId ? 'bg-surface-selected' : ''}"
					>
						{#if file.id === resourceId}
							<Check size={14} class="text-green-600 shrink-0" />
						{:else}
							<File size={14} class="text-tertiary shrink-0" />
						{/if}
						<button
							class="flex-1 text-left truncate hover:underline"
							title="Select {file.name}"
							onclick={() => selectItem(file)}
							{disabled}
						>
							{file.name}
						</button>
					</div>
				{/if}
			{/each}

			{#if nextPageToken}
				<div class="flex justify-center pt-1">
					<Button
						size="xs"
						variant="subtle"
						loading={loadingMore}
						onClick={() => loadFiles(nextPageToken)}
						{disabled}
					>
						Load more
					</Button>
				</div>
			{/if}
		{/if}
	</div>
</div>
