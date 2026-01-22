<script lang="ts">
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Runnable } from '../apps/inputType'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'
	import FileTreeNode from './FileTreeNode.svelte'
	import { buildFileTree } from './fileTreeUtils'
	import { Plus, File, Folder, Camera } from 'lucide-svelte'
	import type { Modules } from './RawAppModules.svelte'
	import RawAppModules from './RawAppModules.svelte'
	import RawAppHistoryList from './RawAppHistoryList.svelte'
	import type { RawAppHistoryManager } from './RawAppHistoryManager.svelte'
	import Button from '../common/button/Button.svelte'
	import RawAppDataTableList from './RawAppDataTableList.svelte'
	import type { DataTableRef } from './dataTableRefUtils'
	import RawAppDataTableDrawer from './RawAppDataTableDrawer.svelte'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		files: Record<string, string> | undefined
		modules?: Modules
		onSelectFile?: (path: string) => void
		selectedDocument: string | undefined
		historyManager?: RawAppHistoryManager
		historySelectedId?: number | undefined
		onHistorySelect?: (id: number) => void
		onHistorySelectCurrent?: () => void
		onManualSnapshot?: () => void
		dataTableRefs?: DataTableRef[]
		onDataTableRefsChange?: (refs: DataTableRef[]) => void
		/** Default datatable for new tables */
		defaultDatatable?: string | undefined
		/** Default schema for new tables */
		defaultSchema?: string | undefined
		onDefaultChange?: (datatable: string | undefined, schema: string | undefined) => void
	}

	let {
		runnables,
		selectedRunnable = $bindable(),
		files = $bindable(),
		modules,
		onSelectFile,
		selectedDocument = $bindable(),
		historyManager,
		historySelectedId,
		onHistorySelect,
		onHistorySelectCurrent,
		onManualSnapshot,
		dataTableRefs = [],
		onDataTableRefsChange,
		defaultDatatable = undefined,
		defaultSchema = undefined,
		onDefaultChange
	}: Props = $props()

	let dataTableDrawer: RawAppDataTableDrawer | undefined = $state()
	let selectedDataTableIndex: number | undefined = $state(undefined)

	function handleAddDataTable(ref: DataTableRef) {
		onDataTableRefsChange?.([...dataTableRefs, ref])
	}

	function handleRemoveDataTable(index: number) {
		onDataTableRefsChange?.(dataTableRefs.filter((_, i) => i !== index))
		if (selectedDataTableIndex === index) {
			selectedDataTableIndex = undefined
		}
	}

	function handleSelectDataTable(ref: DataTableRef, index: number) {
		selectedDataTableIndex = selectedDataTableIndex === index ? undefined : index
		// Open the drawer in manage mode when selecting a data table
		if (selectedDataTableIndex === index) {
			dataTableDrawer?.openDrawerWithRef(ref)
		}
	}

	// Track pending new file/folder that hasn't been confirmed yet
	let pendingNewFilePath = $state<string | undefined>(undefined)

	const fileTree = $derived(
		buildFileTree([
			...Object.keys(files ?? {}),
			...(pendingNewFilePath ? [pendingNewFilePath] : [])
		])
	)

	let pathToEdit = $state<string | undefined>(undefined)

	// Helper to find a unique path by appending numbers if needed
	function getUniquePath(basePath: string): string {
		const existingPaths = new Set([...Object.keys(files ?? {}), pendingNewFilePath].filter(Boolean))

		if (!existingPaths.has(basePath)) {
			return basePath
		}

		// Split path into name and extension (for files) or handle folders
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
			if (dotIndex > 0) {
				nameWithoutExt = fileName.substring(0, dotIndex)
				ext = fileName.substring(dotIndex)
			} else {
				nameWithoutExt = fileName
				ext = ''
			}
		}

		// Try incrementing numbers until we find a unique path
		let counter = 1
		let candidatePath: string
		do {
			const newName = `${nameWithoutExt} (${counter})${ext}`
			candidatePath = isFolder ? `${parentPath}${newName}/` : `${parentPath}${newName}`
			counter++
		} while (existingPaths.has(candidatePath))

		return candidatePath
	}

	function handleFileClick(path: string) {
		console.log('File clicked:', path)
		selectedDocument = path
		// Only open files in the editor, not folders
		if (!path.endsWith('/')) {
			onSelectFile?.(path)
		}
	}

	function handleAddFile(folderPath: string) {
		console.log('Add file to:', folderPath)
		// Ensure folderPath ends with /
		const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
		const basePath = normalizedFolder + 'newfile.txt'
		const newPath = getUniquePath(basePath)
		// Don't update files yet - just mark as pending and enter edit mode
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	function handleRename(oldPath: string, newName: string) {
		const pathParts = oldPath.split('/').filter(Boolean)
		const parentPath = '/' + pathParts.slice(0, -1).join('/')
		let newPath = parentPath === '/' ? '/' + newName : parentPath + '/' + newName

		// Check if this is a folder (ends with /)
		const isFolder = oldPath.endsWith('/')

		// For folders, ensure new path also ends with /
		if (isFolder && !newPath.endsWith('/')) {
			newPath = newPath + '/'
		}

		// Check if this is a pending new file/folder being created
		const isPendingNew = pendingNewFilePath === oldPath

		// For existing items, skip if name didn't change
		if (!isPendingNew && oldPath === newPath) {
			pathToEdit = undefined
			return
		}

		if (!files) {
			files = {}
		}
		const nfiles = { ...files }

		if (isFolder) {
			if (isPendingNew) {
				// Creating a new folder - just add it with the final name
				nfiles[newPath] = ''
				pendingNewFilePath = undefined
			} else {
				// Renaming existing folder
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
			}

			selectedDocument = newPath
		} else {
			if (isPendingNew) {
				// Creating a new file - just add it with the final name
				nfiles[newPath] = ''
				pendingNewFilePath = undefined
			} else {
				// Renaming existing file
				nfiles[newPath] = nfiles[oldPath]
				delete nfiles[oldPath]
			}
			selectedDocument = newPath
		}

		files = nfiles
		pathToEdit = undefined

		// Select the new file in the editor (only for files, not folders)
		if (!isFolder) {
			onSelectFile?.(newPath)
		}
	}

	function handleAddFolder(folderPath: string) {
		console.log('Add folder to:', folderPath)
		// Ensure folderPath ends with /
		const normalizedFolder = folderPath.endsWith('/') ? folderPath : folderPath + '/'
		const basePath = normalizedFolder + 'newfolder/'
		const newPath = getUniquePath(basePath)
		// Don't update files yet - just mark as pending and enter edit mode
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	function handleAddRootFile() {
		console.log('Add file to root or selected folder')
		let basePath: string

		if (selectedDocument) {
			// If a folder is selected, add the file inside it
			if (selectedDocument.endsWith('/')) {
				basePath = selectedDocument + 'newfile.txt'
			} else {
				// If a file is selected, add the new file in the same folder
				const pathParts = selectedDocument.split('/').filter(Boolean)
				if (pathParts.length > 1) {
					// File is in a subfolder
					const parentPath = '/' + pathParts.slice(0, -1).join('/') + '/'
					basePath = parentPath + 'newfile.txt'
				} else {
					// File is at root
					basePath = '/newfile.txt'
				}
			}
		} else {
			basePath = '/newfile.txt'
		}

		const newPath = getUniquePath(basePath)
		// Don't update files yet - just mark as pending and enter edit mode
		pendingNewFilePath = newPath
		pathToEdit = newPath
	}

	function handleAddRootFolder() {
		let basePath: string

		if (selectedDocument) {
			// If a folder is selected, add the folder inside it
			if (selectedDocument.endsWith('/')) {
				basePath = selectedDocument + 'newfolder/'
			} else {
				// If a file is selected, add the new folder in the same folder
				const pathParts = selectedDocument.split('/').filter(Boolean)
				if (pathParts.length > 1) {
					// File is in a subfolder
					const parentPath = '/' + pathParts.slice(0, -1).join('/') + '/'
					basePath = parentPath + 'newfolder/'
				} else {
					// File is at root
					basePath = '/newfolder/'
				}
			}
		} else {
			basePath = '/newfolder/'
		}

		const newPath = getUniquePath(basePath)
		// Don't update files yet - just mark as pending and enter edit mode
		pendingNewFilePath = newPath
		pathToEdit = newPath
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

			// Clear selection if deleted item was selected
			if (selectedDocument === path || (isFolder && selectedDocument?.startsWith(path))) {
				selectedDocument = undefined
			}
		}
	}
</script>

<PanelSection size="sm" fullHeight={false} title="frontend" id="app-editor-frontend-panel">
	{#snippet action()}
		<div class="flex gap-1">
			<div class="flex gap-1">
				<Button
					onClick={handleAddRootFile}
					title="Add file to root"
					unifiedSize="xs"
					variant="subtle"
					btnClasses="px-1 gap-0.5"
				>
					<Plus size={12} />
					<File size={12} />
				</Button>
				<Button
					onClick={handleAddRootFolder}
					title="Add folder to root"
					unifiedSize="xs"
					variant="subtle"
					btnClasses="px-1 gap-0.5"
				>
					<Plus size={12} />
					<Folder size={12} />
				</Button>
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
				{pathToEdit}
				onRequestEdit={(path) => (pathToEdit = path)}
				onCancelEdit={() => {
					pathToEdit = undefined
					pendingNewFilePath = undefined
				}}
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

<div class="py-4"></div>
<RawAppInlineScriptPanelList
	bind:selectedRunnable
	{runnables}
	onSelect={() => {
		selectedDocument = undefined
	}}
/>

<div class="py-4"></div>
<RawAppDataTableList
	{dataTableRefs}
	{defaultDatatable}
	{defaultSchema}
	onAdd={() => dataTableDrawer?.openDrawer()}
	onRemove={handleRemoveDataTable}
	onSelect={handleSelectDataTable}
	{onDefaultChange}
	selectedIndex={selectedDataTableIndex}
/>
<RawAppDataTableDrawer
	bind:this={dataTableDrawer}
	onAdd={handleAddDataTable}
	existingRefs={dataTableRefs}
/>

{#if historyManager && onHistorySelect && onManualSnapshot}
	<div class="py-4"></div>
	<PanelSection fullHeight={false} size="sm" title="history" id="app-editor-history-panel">
		{#snippet action()}
			<div class="flex items-center gap-2">
				<span class="text-2xs text-tertiary">{historyManager.allEntries.length}/50</span>
				<Button
					unifiedSize="xs"
					variant="subtle"
					on:click={onManualSnapshot}
					btnClasses="px-1 gap-0.5"
					title="Create a new snapshot"
				>
					<Plus size={12} />
					<Camera size={12} />
				</Button>
			</div>
		{/snippet}
		<RawAppHistoryList
			entries={historyManager.allEntries}
			branches={historyManager.allBranches}
			selectedId={historySelectedId}
			onSelect={onHistorySelect}
			onSelectCurrent={onHistorySelectCurrent}
		/>
	</PanelSection>
{/if}
