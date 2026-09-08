<script lang="ts">
	import Button from './common/button/Button.svelte'
	import { Plus, File, Folder, FolderOpen } from 'lucide-svelte'
	import FileTreeNode from './raw_apps/FileTreeNode.svelte'
	import type { TreeNode } from './raw_apps/fileTreeUtils'
	import { buildFileTree } from './raw_apps/fileTreeUtils'

	interface Props {
		/** File path → content map. Keys use / prefix (e.g. /index.html). */
		files: Record<string, string>
		/** Currently selected file (/-prefixed). Read-only; changes via onSelectPath callback. */
		selectedPath?: string | undefined
		/** Called when the user clicks a file. Folders aren't selectable — clicking
		 * one only expands it — so the only non-file paths this reports are the root
		 * row under `showRoot`, and '' when the last file is deleted. */
		onSelectPath?: (path: string) => void
		/** Extra tree nodes appended after the main tree (e.g. read-only wmill.ts). */
		extraNodes?: TreeNode[]
		/** Show a root / entry at the top of the tree. */
		showRoot?: boolean
		/** Hide the built-in header (useful when parent provides its own). */
		hideHeader?: boolean
	}

	let {
		files = $bindable({}),
		selectedPath = undefined,
		onSelectPath,
		extraNodes,
		showRoot = false,
		hideHeader = false
	}: Props = $props()

	let pendingNewFilePath: string | undefined = $state(undefined)
	let pathToEdit: string | undefined = $state(undefined)
	// Empty folders exist only in the UI until a file is created inside them
	let emptyFolders: string[] = $state([])

	const fileTree = $derived(
		buildFileTree([
			...Object.keys(files ?? {}),
			...emptyFolders,
			...(pendingNewFilePath ? [pendingNewFilePath] : [])
		])
	)

	function getUniquePath(basePath: string): string {
		const existingPaths = new Set(
			[...Object.keys(files ?? {}), ...emptyFolders, pendingNewFilePath].filter(Boolean)
		)

		if (!existingPaths.has(basePath)) return basePath

		const isFolder = basePath.endsWith('/')
		let pathWithoutTrailing = isFolder ? basePath.slice(0, -1) : basePath
		const lastSlash = pathWithoutTrailing.lastIndexOf('/')
		const parentPath = pathWithoutTrailing.substring(0, lastSlash + 1)
		const fileName = pathWithoutTrailing.substring(lastSlash + 1)

		let nameWithoutExt: string
		let ext: string
		if (isFolder) {
			nameWithoutExt = fileName
			ext = ''
		} else {
			const dotIndex = fileName.lastIndexOf('.')
			nameWithoutExt = dotIndex > 0 ? fileName.substring(0, dotIndex) : fileName
			ext = dotIndex > 0 ? fileName.substring(dotIndex) : ''
		}

		let counter = 1
		let candidate: string
		do {
			const newName = `${nameWithoutExt} (${counter})${ext}`
			candidate = isFolder ? `${parentPath}${newName}/` : `${parentPath}${newName}`
			counter++
		} while (existingPaths.has(candidate))
		return candidate
	}

	function handleFileClick(path: string) {
		onSelectPath?.(path)
	}

	function parentFolderOfSelection(): string {
		if (!selectedPath || selectedPath === '/') return '/'
		const pathParts = selectedPath.split('/').filter(Boolean)
		return pathParts.length > 1 ? '/' + pathParts.slice(0, -1).join('/') + '/' : '/'
	}

	function handleAddFile(folderPath: string) {
		const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
		const basePath = normalizedFolder + 'newfile.txt'
		const newPath = getUniquePath(basePath)
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	// New entries land beside the selected file; with nothing selected, at the
	// root. To create inside another folder, use that folder row's own menu.
	export function handleAddFileBesideSelection() {
		const basePath = parentFolderOfSelection() + 'newfile.txt'
		const newPath = getUniquePath(basePath)
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	function handleAddFolder(folderPath: string) {
		const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
		const basePath = normalizedFolder + 'newfolder/'
		const newPath = getUniquePath(basePath)
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	export function handleAddFolderBesideSelection() {
		const basePath = parentFolderOfSelection() + 'newfolder/'
		const newPath = getUniquePath(basePath)
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	function handleRename(oldPath: string, newName: string) {
		const isFolder = oldPath.endsWith('/')
		const pathParts = oldPath.split('/').filter(Boolean)
		const parentPath = '/' + pathParts.slice(0, -1).join('/')
		let newPath = parentPath === '/' ? '/' + newName : parentPath + '/' + newName
		if (isFolder && !newPath.endsWith('/')) {
			newPath = newPath + '/'
		}

		const isPendingNew = pendingNewFilePath === oldPath

		if (!isPendingNew && oldPath === newPath) {
			pathToEdit = undefined
			return
		}

		const nfiles = { ...files }

		if (isFolder) {
			if (isPendingNew) {
				// New empty folder — track in UI until a file is created inside
				emptyFolders = [...emptyFolders, newPath]
				pendingNewFilePath = undefined
			} else {
				// Rename all children under old folder path
				for (const key of Object.keys(nfiles)) {
					if (key === oldPath || key.startsWith(oldPath)) {
						const newKey = newPath + key.substring(oldPath.length)
						nfiles[newKey] = nfiles[key]
						delete nfiles[key]
					}
				}
				// Also rename in emptyFolders
				emptyFolders = emptyFolders.map((f) =>
					f === oldPath || f.startsWith(oldPath) ? newPath + f.substring(oldPath.length) : f
				)
			}
		} else {
			if (isPendingNew) {
				nfiles[newPath] = ''
				pendingNewFilePath = undefined
			} else {
				nfiles[newPath] = nfiles[oldPath]
				delete nfiles[oldPath]
			}
			// Remove empty folders that are now implicitly defined by this file path
			emptyFolders = emptyFolders.filter((f) => !newPath.startsWith(f))
		}

		files = nfiles
		pathToEdit = undefined
		if (!isFolder) {
			onSelectPath?.(newPath)
		} else if (selectedPath?.startsWith(oldPath)) {
			// A folder isn't selectable, but the selected file moved with it — follow
			// it to its new path, or the caller keeps editing a key that's now gone.
			onSelectPath?.(newPath + selectedPath.slice(oldPath.length))
		}
	}

	function handleDelete(path: string) {
		const isFolder = path.endsWith('/')
		const nfiles = { ...files }

		if (isFolder) {
			for (const key of Object.keys(nfiles)) {
				if (key === path || key.startsWith(path)) {
					delete nfiles[key]
				}
			}
			emptyFolders = emptyFolders.filter((f) => f !== path && !f.startsWith(path))
		} else {
			delete nfiles[path]
		}

		files = nfiles

		if (selectedPath === path || (isFolder && selectedPath?.startsWith(path))) {
			const remainingFile = Object.keys(nfiles).find((key) => !key.endsWith('/'))
			onSelectPath?.(remainingFile ?? (showRoot ? '/' : ''))
		}
	}
</script>

{#if !hideHeader}
	<div class="p-2 border-b flex items-center justify-between">
		<span class="text-xs font-semibold text-emphasis">Files</span>
		<div class="flex gap-1">
			<Button
				onClick={handleAddFileBesideSelection}
				title="Add file"
				unifiedSize="xs"
				variant="subtle"
				btnClasses="px-1 gap-0.5"
			>
				<Plus size={12} />
				<File size={12} />
			</Button>
			<Button
				onClick={handleAddFolderBesideSelection}
				title="Add folder"
				unifiedSize="xs"
				variant="subtle"
				btnClasses="px-1 gap-0.5"
			>
				<Plus size={12} />
				<Folder size={12} />
			</Button>
		</div>
	</div>
{/if}
<div class="flex-1 overflow-y-auto py-1 w-full">
	{#if showRoot}
		<button
			onclick={() => onSelectPath?.('/')}
			class="w-full flex items-center gap-1 px-2 py-1 text-xs hover:bg-surface-hover transition-colors rounded text-left {selectedPath ===
			'/'
				? 'bg-surface-accent-selected'
				: ''}"
		>
			<FolderOpen size={12} class="flex-shrink-0 text-secondary" />
			<span class="truncate text-primary font-normal {selectedPath === '/' ? 'text-accent' : ''}"
				>/</span
			>
		</button>
	{/if}
	{#each fileTree as node (node.path)}
		<FileTreeNode
			{node}
			onFileClick={handleFileClick}
			onAddFile={handleAddFile}
			onAddFolder={handleAddFolder}
			onRename={handleRename}
			onDelete={handleDelete}
			{selectedPath}
			{pathToEdit}
			onRequestEdit={(path) => (pathToEdit = path)}
			onCancelEdit={() => {
				pathToEdit = undefined
				pendingNewFilePath = undefined
			}}
		/>
	{/each}
	{#if extraNodes}
		{#each extraNodes as node (node.path)}
			<FileTreeNode
				{node}
				noEdit
				onFileClick={handleFileClick}
				onAddFile={handleAddFile}
				onAddFolder={handleAddFolder}
				{selectedPath}
			/>
		{/each}
	{/if}
</div>
