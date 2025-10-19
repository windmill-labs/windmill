<script lang="ts">
	import {
		ChevronRight,
		ChevronDown,
		File,
		Folder,
		FileJson,
		FileCode,
		ImageIcon,
		Palette,
		Pencil,
		Trash2,
		Lock
	} from 'lucide-svelte'
	import Self from './FileTreeNode.svelte'

	interface TreeNode {
		name: string
		path: string
		isFolder: boolean
		children?: TreeNode[]
	}

	interface Props {
		node: TreeNode
		onFileClick?: (path: string) => void
		onAddFile?: (folderPath: string) => void
		onAddFolder?: (folderPath: string) => void
		onRename?: (oldPath: string, newName: string) => void
		onDelete?: (path: string) => void
		selectedPath?: string
		pathToRename?: string
		pathToExpand?: string
		noEdit?: boolean
		level?: number
	}

	let {
		node,
		onFileClick,
		onAddFile,
		onAddFolder,
		onRename,
		onDelete,
		selectedPath,
		pathToRename,
		pathToExpand,
		noEdit = false,
		level = 0
	}: Props = $props()

	let expanded = $state(level === 0) // Root folders start expanded
	let isHovered = $state(false)
	let isEditing = $state(false)
	let editValue = $state(node.name)
	let inputElement: HTMLInputElement | undefined = $state()

	const isSelected = $derived(selectedPath === node.path)

	function toggleExpanded() {
		if (node.isFolder) {
			expanded = !expanded
		}
	}

	function handleClick() {
		// Always notify about selection
		onFileClick?.(node.path)

		// Toggle expansion for folders
		if (node.isFolder) {
			toggleExpanded()
		}
	}

	function handleEdit(e: MouseEvent) {
		e.stopPropagation()
		isEditing = true
		editValue = node.name
	}

	function handleDelete(e: MouseEvent) {
		e.stopPropagation()
		onDelete?.(node.path)
	}

	function finishEdit() {
		if (isEditing && editValue.trim() && editValue !== node.name) {
			onRename?.(node.path, editValue.trim())
		}
		isEditing = false
	}

	function handleInputKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			finishEdit()
		} else if (e.key === 'Escape') {
			isEditing = false
			editValue = node.name
		}
	}

	function handleInputBlur() {
		finishEdit()
	}

	$effect(() => {
		if (isEditing && inputElement) {
			inputElement.focus()
			inputElement.select()
		}
	})

	// Automatically enter edit mode for newly created files
	$effect(() => {
		if (pathToRename === node.path && !isEditing) {
			isEditing = true
			editValue = node.name
			// If this is a folder, expand it
			if (node.isFolder) {
				expanded = true
			}
		}
	})

	// Expand parent folders when a child needs to be renamed
	$effect(() => {
		if (pathToRename && node.isFolder && pathToRename.startsWith(node.path + '/')) {
			expanded = true
		}
	})

	// Expand folder when pathToExpand matches this node or a parent of pathToExpand
	$effect(() => {
		if (pathToExpand && node.isFolder) {
			// Expand if this folder is the target or an ancestor of the target
			if (node.path === pathToExpand || pathToExpand.startsWith(node.path + '/')) {
				expanded = true
			}
		}
	})

	const sortedChildren = $derived(
		node.children?.slice().sort((a, b) => {
			// Folders first, then files
			if (a.isFolder !== b.isFolder) {
				return a.isFolder ? -1 : 1
			}
			return a.name.localeCompare(b.name)
		})
	)

	function getFileExtension(filename: string): string {
		const parts = filename.split('.')
		return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : ''
	}

	const fileIcon = $derived.by(() => {
		if (node.isFolder) {
			return { icon: Folder, className: 'text-secondary' }
		}

		const ext = getFileExtension(node.name)

		switch (ext) {
			case 'json':
				return { icon: FileJson, className: 'text-yellow-500' }
			case 'tsx':
			case 'jsx':
				return { icon: FileCode, className: 'text-blue-500' }
			case 'ts':
			case 'js':
				return { icon: FileCode, className: 'text-blue-400' }
			case 'svelte':
				return { icon: FileCode, className: 'text-orange-500' }
			case 'vue':
				return { icon: FileCode, className: 'text-green-500' }
			case 'css':
			case 'scss':
			case 'sass':
			case 'less':
				return { icon: Palette, className: 'text-pink-500' }
			case 'png':
			case 'jpg':
			case 'jpeg':
			case 'gif':
			case 'svg':
			case 'webp':
			case 'ico':
				return { icon: ImageIcon, className: 'text-purple-500' }
			default:
				return { icon: File, className: 'text-tertiary' }
		}
	})
</script>

<div>
	<div
		role="group"
		class="relative"
		onmouseenter={() => (isHovered = true)}
		onmouseleave={() => (isHovered = false)}
	>
		{#if isEditing}
			<div
				class="w-full flex items-center gap-1 px-2 py-1 text-xs rounded {isSelected
					? 'bg-blue-100 dark:bg-blue-900/30'
					: ''}"
				style="padding-left: {level * 12}px"
			>
				{#if node.isFolder}
					{@const IconComponent = fileIcon.icon}
					<span class="flex-shrink-0 text-secondary">
						{#if expanded}
							<ChevronDown size={12} />
						{:else}
							<ChevronRight size={12} />
						{/if}
					</span>
					<IconComponent size={12} class="flex-shrink-0 {fileIcon.className}" />
				{:else}
					{@const IconComponent = fileIcon.icon}
					<span class="flex-shrink-0"></span>
					<IconComponent size={12} class="flex-shrink-0 {fileIcon.className}" />
				{/if}
				<input
					bind:this={inputElement}
					bind:value={editValue}
					onkeydown={handleInputKeydown}
					onblur={handleInputBlur}
					class="flex-1 min-w-0 bg-surface border border-blue-500 rounded px-1 text-primary font-mono focus:outline-none focus:ring-1 focus:ring-blue-500"
					type="text"
				/>
			</div>
		{:else}
			<button
				onclick={handleClick}
				class="w-full flex items-center gap-1 px-2 py-1 text-xs hover:bg-surface-hover transition-colors rounded text-left {isSelected
					? 'bg-blue-100 dark:bg-blue-900/30'
					: ''}"
				style="padding-left: {level * 12}px"
			>
				{#if node.isFolder}
					{@const IconComponent = fileIcon.icon}
					<span class="flex-shrink-0 text-secondary">
						{#if expanded}
							<ChevronDown size={12} />
						{:else}
							<ChevronRight size={12} />
						{/if}
					</span>
					<IconComponent size={12} class="flex-shrink-0 {fileIcon.className}" />
				{:else}
					{@const IconComponent = fileIcon.icon}
					<span class="flex-shrink-0"></span>
					<IconComponent size={12} class="flex-shrink-0 {fileIcon.className}" />
				{/if}
				<span class="truncate text-primary font-mono">{node.name}</span>
			</button>

			{#if isHovered && !isEditing}
				<div
					class="absolute right-1 top-1 flex gap-0.5 bg-surface rounded shadow-sm border border-gray-200 dark:border-gray-700"
				>
					{#if !noEdit}
						<button
							onclick={handleEdit}
							class="p-0.5 hover:bg-surface-hover rounded transition-colors"
							title="Rename"
						>
							<Pencil size={12} class="text-secondary" />
						</button>
						<button
							onclick={handleDelete}
							class="p-0.5 hover:bg-surface-hover rounded transition-colors"
							title="Delete"
						>
							<Trash2 size={12} class="text-red-500" />
						</button>
					{:else}
						<Lock size={12} class="text-secondary" />
					{/if}
				</div>
			{/if}
		{/if}
	</div>

	{#if node.isFolder && expanded && sortedChildren}
		{#each sortedChildren as child (child.path)}
			<Self
				node={child}
				{onFileClick}
				{onAddFile}
				{onAddFolder}
				{onRename}
				{onDelete}
				{selectedPath}
				{pathToRename}
				{pathToExpand}
				level={level + 1}
			/>
		{/each}
	{/if}
</div>
