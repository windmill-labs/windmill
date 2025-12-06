<script lang="ts">
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Runnable } from '../apps/inputType'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'
	import FileTreeNode from './FileTreeNode.svelte'
	import { buildFileTree } from './fileTreeUtils'
	import { Plus, File, Folder, Undo2, Redo2 } from 'lucide-svelte'
	import type { Modules } from './RawAppModules.svelte'
	import RawAppModules from './RawAppModules.svelte'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		files: Record<string, string> | undefined
		modules?: Modules
		onSelectFile?: (path: string) => void
		selectedDocument: string | undefined
	}

	let {
		runnables,
		selectedRunnable = $bindable(),
		files = $bindable(),
		modules,
		onSelectFile,
		selectedDocument = $bindable()
	}: Props = $props()

	const fileTree = $derived(buildFileTree(Object.keys(files ?? {})))

	let pathToRename = $state<string | undefined>(undefined)
	let pathToExpand = $state<string | undefined>(undefined)

	// History management for undo/redo
	const MAX_HISTORY = 5
	let history = $state<Record<string, string>[]>([])
	let historyIndex = $state(-1)

	const canUndo = $derived(historyIndex > 0)
	const canRedo = $derived(historyIndex < history.length - 1)

	function addToHistory(newFiles: Record<string, string>) {
		// Remove any future history if we're not at the end
		if (historyIndex < history.length - 1) {
			history = history.slice(0, historyIndex + 1)
		}

		// Add new state
		history = [...history, $state.snapshot(newFiles)]

		// Keep only last MAX_HISTORY items
		if (history.length > MAX_HISTORY) {
			history = history.slice(-MAX_HISTORY)
		} else {
			historyIndex++
		}
	}

	function undo() {
		if (canUndo && files) {
			historyIndex--
			files = $state.snapshot(history[historyIndex])
		}
	}

	function redo() {
		if (canRedo && files) {
			historyIndex++
			files = $state.snapshot(history[historyIndex])
		}
	}

	// Initialize history with current state
	$effect(() => {
		if (files && history.length === 0) {
			history = [$state.snapshot(files)]
			historyIndex = 0
		}
	})

	function handleFileClick(path: string) {
		console.log('File clicked:', path)
		selectedDocument = path
		onSelectFile?.(path)
	}

	function handleAddFile(folderPath: string) {
		console.log('Add file to:', folderPath)
		if (files) {
			const nfiles = { ...files }
			// Ensure folderPath ends with /
			const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
			const newPath = normalizedFolder + 'newfile.txt'
			nfiles[newPath] = ''
			files = nfiles
			addToHistory(nfiles)
			pathToRename = newPath
			pathToExpand = normalizedFolder
		}
	}

	function handleRename(oldPath: string, newName: string) {
		if (files) {
			const nfiles = { ...files }
			const pathParts = oldPath.split('/').filter(Boolean)
			const parentPath = '/' + pathParts.slice(0, -1).join('/')
			let newPath = parentPath === '/' ? '/' + newName : parentPath + '/' + newName

			// Check if this is a folder (ends with /)
			const isFolder = oldPath.endsWith('/')

			if (isFolder) {
				// For folders, ensure new path also ends with /
				if (!newPath.endsWith('/')) {
					newPath = newPath + '/'
				}

				const oldFolderPath = oldPath
				const newFolderPath = newPath

				// Collect all paths to rename (including the folder itself and all children)
				const pathsToRename: Array<{ old: string; new: string }> = []

				Object.keys(nfiles).forEach((filePath) => {
					if (filePath === oldFolderPath) {
						// The folder itself
						pathsToRename.push({ old: filePath, new: newFolderPath })
					} else if (filePath.startsWith(oldFolderPath)) {
						// Children of the folder
						const relativePath = filePath.substring(oldFolderPath.length)
						const updatedPath = newFolderPath + relativePath
						pathsToRename.push({ old: filePath, new: updatedPath })
					}
				})

				// Apply all renames
				pathsToRename.forEach(({ old, new: newPath }) => {
					nfiles[newPath] = nfiles[old]
					delete nfiles[old]
				})

				selectedDocument = newFolderPath
			} else {
				// For files, simple rename
				nfiles[newPath] = nfiles[oldPath]
				delete nfiles[oldPath]
				selectedDocument = newPath
			}

			files = nfiles
			addToHistory(nfiles)
			pathToRename = undefined
		}
	}

	function handleAddFolder(folderPath: string) {
		console.log('Add folder to:', folderPath)
		if (files) {
			const nfiles = { ...files }
			// Ensure folderPath ends with /
			const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
			const newPath = normalizedFolder + 'newfolder/'
			nfiles[newPath] = ''
			files = nfiles
			addToHistory(nfiles)
			pathToRename = newPath
			pathToExpand = normalizedFolder
		}
	}

	function handleAddRootFile() {
		console.log('Add file to root or selected folder')
		if (files) {
			const nfiles = { ...files }
			let newPath: string
			let targetFolder: string | undefined

			if (selectedDocument) {
				// If a folder is selected, add the file inside it
				if (selectedDocument.endsWith('/')) {
					newPath = selectedDocument + 'newfile.txt'
					targetFolder = selectedDocument
				} else {
					// If a file is selected, add the new file in the same folder
					const pathParts = selectedDocument.split('/').filter(Boolean)
					if (pathParts.length > 1) {
						// File is in a subfolder
						const parentPath = '/' + pathParts.slice(0, -1).join('/') + '/'
						newPath = parentPath + 'newfile.txt'
						targetFolder = parentPath
					} else {
						// File is at root
						newPath = '/newfile.txt'
					}
				}
			} else {
				newPath = '/newfile.txt'
			}

			nfiles[newPath] = ''
			files = nfiles
			addToHistory(nfiles)
			pathToRename = newPath
			if (targetFolder) {
				pathToExpand = targetFolder
			}
		}
	}

	function handleAddRootFolder() {
		if (files) {
			const nfiles = { ...files }
			let newPath: string
			let targetFolder: string | undefined

			if (selectedDocument) {
				// If a folder is selected, add the folder inside it
				if (selectedDocument.endsWith('/')) {
					newPath = selectedDocument + 'newfolder/'
					targetFolder = selectedDocument
				} else {
					// If a file is selected, add the new folder in the same folder
					const pathParts = selectedDocument.split('/').filter(Boolean)
					if (pathParts.length > 1) {
						// File is in a subfolder
						const parentPath = '/' + pathParts.slice(0, -1).join('/') + '/'
						newPath = parentPath + 'newfolder/'
						targetFolder = parentPath
					} else {
						// File is at root
						newPath = '/newfolder/'
					}
				}
			} else {
				newPath = '/newfolder/'
			}

			nfiles[newPath] = ''
			files = nfiles
			addToHistory(nfiles)
			pathToRename = newPath
			if (targetFolder) {
				pathToExpand = targetFolder
			}
		}
	}

	function handleDelete(path: string) {
		if (files) {
			console.log('Delete:', path)
			const nfiles = { ...files }

			// Check if this is a folder (ends with /)
			const isFolder = path.endsWith('/')

			if (isFolder) {
				// Delete the folder and all its children
				Object.keys(nfiles).forEach((filePath) => {
					if (filePath === path || filePath.startsWith(path)) {
						delete nfiles[filePath]
					}
				})
			} else {
				// Delete single file
				delete nfiles[path]
			}

			files = nfiles
			console.log(nfiles)
			addToHistory(nfiles)

			// Clear selection if deleted item was selected
			if (selectedDocument === path || (isFolder && selectedDocument?.startsWith(path))) {
				selectedDocument = undefined
			}
		}
	}
</script>

<!-- {JSON.stringify(history)} -->
<PanelSection size="lg" fullHeight={false} title="Frontend" id="app-editor-frontend-panel">
	{#snippet action()}
		<div class="flex gap-1">
			<div class="flex gap-0.5 border-r border-gray-200 dark:border-gray-700 pr-1">
				<button
					onclick={undo}
					disabled={!canUndo}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
					title="Undo (Ctrl+Z)"
				>
					<Undo2 size={12} class="text-secondary" />
				</button>
				<button
					onclick={redo}
					disabled={!canRedo}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
					title="Redo (Ctrl+Y)"
				>
					<Redo2 size={12} class="text-secondary" />
				</button>
			</div>
			<div class="flex gap-0.5">
				<button
					onclick={handleAddRootFile}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
					title="Add file to root"
				>
					<Plus size={12} class="text-secondary" />
					<File size={12} class="text-tertiary" />
				</button>
				<button
					onclick={handleAddRootFolder}
					class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
					title="Add folder to root"
				>
					<Plus size={12} class="text-secondary" />
					<Folder size={12} class="text-tertiary" />
				</button>
			</div>
		</div>
	{/snippet}
	<div class="flex flex-col gap-0.5 w-full">
		{#each fileTree as node (node.path)}
			<FileTreeNode
				{node}
				onFileClick={handleFileClick}
				onAddFile={handleAddFile}
				onAddFolder={handleAddFolder}
				onRename={handleRename}
				onDelete={handleDelete}
				selectedPath={selectedDocument}
				{pathToRename}
				{pathToExpand}
			/>
		{/each}
		<FileTreeNode
			node={{
				name: 'wmill.ts',
				path: '/wmill.ts',
				isFolder: false
			}}
			noEdit={true}
			onFileClick={handleFileClick}
			onAddFile={handleAddFile}
			onAddFolder={handleAddFolder}
			selectedPath={selectedDocument}
		/>
	</div>
</PanelSection>

<RawAppModules {modules} />

<div class="py-10"></div>
<RawAppInlineScriptPanelList bind:selectedRunnable {runnables} />
