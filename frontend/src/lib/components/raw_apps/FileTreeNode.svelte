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
		Plus
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
		selectedPath?: string
		level?: number
	}

	let { node, onFileClick, onAddFile, onAddFolder, selectedPath, level = 0 }: Props = $props()

	let expanded = $state(level === 0) // Root folders start expanded
	let isHovered = $state(false)

	const isSelected = $derived(selectedPath === node.path)

	function toggleExpanded() {
		if (node.isFolder) {
			expanded = !expanded
		}
	}

	function handleClick() {
		if (node.isFolder) {
			toggleExpanded()
		} else {
			onFileClick?.(node.path)
		}
	}

	function handleAddFile(e: MouseEvent) {
		e.stopPropagation()
		onAddFile?.(node.path)
	}

	function handleAddFolder(e: MouseEvent) {
		e.stopPropagation()
		onAddFolder?.(node.path)
	}

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

		{#if node.isFolder && isHovered}
			<div class="absolute right-1 top-1 flex gap-0.5 bg-surface rounded shadow-sm border border-gray-200 dark:border-gray-700">
				<button
					onclick={handleAddFile}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
					title="Add file"
				>
					<Plus size={10} class="text-secondary" />
					<File size={10} class="text-tertiary" />
				</button>
				<button
					onclick={handleAddFolder}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
					title="Add folder"
				>
					<Plus size={10} class="text-secondary" />
					<Folder size={10} class="text-tertiary" />
				</button>
			</div>
		{/if}
	</div>

	{#if node.isFolder && expanded && sortedChildren}
		{#each sortedChildren as child (child.path)}
			<Self
				node={child}
				{onFileClick}
				{onAddFile}
				{onAddFolder}
				{selectedPath}
				level={level + 1}
			/>
		{/each}
	{/if}
</div>
