<script lang="ts">
	import {
		ChevronRight,
		ChevronDown,
		File,
		Folder,
		Pencil,
		Trash2,
		Lock,
		Ellipsis,
		ImageIcon
	} from 'lucide-svelte'
	import Self from './FileTreeNode.svelte'
	import { twMerge } from 'tailwind-merge'
	import DropdownV2 from '../DropdownV2.svelte'
	import { Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import TypeScript from '../common/languageIcons/TypeScript.svelte'
	import JavaScriptIcon from '../icons/JavaScriptIcon.svelte'
	import JsonIcon from '../icons/JsonIcon.svelte'
	import ReactIcon from '../icons/ReactIcon.svelte'
	import SvelteIcon from '../icons/SvelteIcon.svelte'
	import VueIcon from '../icons/VueIcon.svelte'
	import CssIcon from '../icons/CssIcon.svelte'
	import SassIcon from '../icons/SassIcon.svelte'
	import LessIcon from '../icons/LessIcon.svelte'
	import HtmlIcon from '../icons/HtmlIcon.svelte'
	import MarkdownIcon from '../icons/MarkdownIcon.svelte'
	import YamlIcon from '../icons/YamlIcon.svelte'
	import { tick } from 'svelte'

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
		onRequestEdit?: (path: string) => void
		onCancelEdit?: () => void
		selectedPath?: string
		pathToEdit?: string
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
		onRequestEdit,
		onCancelEdit,
		selectedPath,
		pathToEdit,
		noEdit = false,
		level = 0
	}: Props = $props()

	let userExpanded = $state<boolean | null>(null) // null = not set by user
	let isHovered = $state(false)
	let editValue = $state(node.name)
	let textInputElement: TextInput | undefined = $state()
	let dropdownOpen = $state(false)

	const isSelected = $derived(selectedPath === node.path)
	const isEditing = $derived(pathToEdit === node.path)
	const expanded = $derived(
		// Auto-expand for editing nested paths takes priority
		pathToEdit && node.isFolder && pathToEdit.startsWith(node.path)
			? true
			: userExpanded !== null
				? userExpanded
				: level === 0 // Default: root expanded
	)

	function toggleExpanded() {
		if (node.isFolder) {
			userExpanded = !expanded
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
		onRequestEdit?.(node.path)
	}

	function handleDelete(e: MouseEvent) {
		e.stopPropagation()
		onDelete?.(node.path)
	}

	function finishEdit() {
		if (isEditing && editValue.trim()) {
			// Always call onRename - parent handles whether it's a new file or actual rename
			onRename?.(node.path, editValue.trim())
		} else {
			onCancelEdit?.()
		}
	}

	function handleInputKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			finishEdit()
		} else if (e.key === 'Escape') {
			e.stopPropagation()
			editValue = node.name // Reset to original
			onCancelEdit?.()
		}
	}

	function handleInputBlur(e: FocusEvent) {
		finishEdit()
	}

	// Single effect for DOM operations only
	$effect(() => {
		if (isEditing && textInputElement) {
			// Reset edit value when entering edit mode
			editValue = node.name
			// Focus and select input (DOM side effects)
			tick().then(() => {
				textInputElement?.focus()
				textInputElement?.select()
			})
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
				return { icon: JsonIcon }
			case 'tsx':
				return { icon: ReactIcon }
			case 'jsx':
				return { icon: ReactIcon }
			case 'ts':
				return { icon: TypeScript }
			case 'js':
				return { icon: JavaScriptIcon }
			case 'svelte':
				return { icon: SvelteIcon }
			case 'vue':
				return { icon: VueIcon }
			case 'css':
				return { icon: CssIcon }
			case 'scss':
			case 'sass':
				return { icon: SassIcon }
			case 'less':
				return { icon: LessIcon }
			case 'png':
			case 'jpg':
			case 'jpeg':
			case 'gif':
			case 'svg':
			case 'webp':
			case 'ico':
				return { icon: ImageIcon, className: 'text-purple-500' }
			case 'html':
			case 'htm':
				return { icon: HtmlIcon }
			case 'md':
			case 'markdown':
				return { icon: MarkdownIcon }
			case 'yaml':
			case 'yml':
				return { icon: YamlIcon }
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
				class="w-full flex items-center gap-1 px-2 min-h-6 text-xs rounded {isSelected
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
					<IconComponent size={14} class="flex-shrink-0 {fileIcon.className}" />
				{:else}
					{@const IconComponent = fileIcon.icon}
					<span class="flex-shrink-0"></span>
					<IconComponent size={14} class="flex-shrink-0 {fileIcon.className}" />
				{/if}
				<TextInput
					bind:this={textInputElement}
					bind:value={editValue}
					inputProps={{
						onkeydown: handleInputKeydown,
						onblur: (e) => handleInputBlur(e),
						type: 'text'
					}}
					size="xs"
				/>
			</div>
		{:else}
			<button
				onclick={handleClick}
				class="w-full flex items-center gap-1 px-2 py-1 text-xs hover:bg-surface-hover transition-colors rounded text-left {isSelected
					? 'bg-surface-accent-selected'
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
				<span class={twMerge('truncate text-primary font-normal', isSelected ? 'text-accent' : '')}
					>{node.name}</span
				>
			</button>

			{#if isHovered || dropdownOpen}
				{#if !noEdit}
					<DropdownV2
						items={[
							{
								displayName: 'Rename',
								icon: Pencil,
								action: handleEdit
							},
							{
								displayName: 'Delete',
								icon: Trash2,
								action: handleDelete,
								type: 'delete'
							}
						]}
						placement="bottom-end"
						class="absolute -translate-y-1/2 top-1/2 right-1"
						bind:open={dropdownOpen}
					>
						{#snippet buttonReplacement()}
							<Button
								iconOnly
								unifiedSize="xs"
								variant="subtle"
								nonCaptureEvent
								startIcon={{ icon: Ellipsis }}
							></Button>
						{/snippet}
					</DropdownV2>
				{:else}
					<Lock size={12} class="text-secondary absolute -translate-y-1/2 top-1/2 right-2" />
				{/if}
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
				{onRequestEdit}
				{onCancelEdit}
				{selectedPath}
				{pathToEdit}
				level={level + 1}
			/>
		{/each}
	{/if}
</div>
