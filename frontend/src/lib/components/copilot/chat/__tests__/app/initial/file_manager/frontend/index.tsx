import React, { useState, useEffect } from 'react'
import { backend } from 'wmill'
import { FolderTree } from './components/FolderTree'
import { FileList } from './components/FileList'
import { Breadcrumb } from './components/Breadcrumb'
import { Toolbar } from './components/Toolbar'

export interface FileItem {
	id: string
	name: string
	type: 'file' | 'folder'
	size?: number
	modifiedAt: string
	parentId: string | null
}

export interface Folder {
	id: string
	name: string
	parentId: string | null
	children: Folder[]
}

const App = () => {
	const [folders, setFolders] = useState<Folder[]>([])
	const [files, setFiles] = useState<FileItem[]>([])
	const [currentFolderId, setCurrentFolderId] = useState<string | null>(null)
	const [path, setPath] = useState<{ id: string | null; name: string }[]>([
		{ id: null, name: 'Root' }
	])
	const [loading, setLoading] = useState(true)

	useEffect(() => {
		loadFolders()
	}, [])

	useEffect(() => {
		loadFiles(currentFolderId)
	}, [currentFolderId])

	const loadFolders = async () => {
		const data = await backend.listFolders()
		setFolders(data)
	}

	const loadFiles = async (folderId: string | null) => {
		setLoading(true)
		const data = await backend.listFiles({ folderId })
		setFiles(data)
		setLoading(false)
	}

	const handleFolderSelect = (folderId: string | null, folderName: string) => {
		setCurrentFolderId(folderId)

		if (folderId === null) {
			setPath([{ id: null, name: 'Root' }])
		} else {
			// Find if folder is already in path
			const existingIndex = path.findIndex((p) => p.id === folderId)
			if (existingIndex >= 0) {
				setPath(path.slice(0, existingIndex + 1))
			} else {
				setPath([...path, { id: folderId, name: folderName }])
			}
		}
	}

	const handleCreateFolder = async (name: string) => {
		await backend.createFolder({ name, parentId: currentFolderId })
		await loadFolders()
		await loadFiles(currentFolderId)
	}

	const handleDeleteItem = async (item: FileItem) => {
		await backend.deleteItem({ id: item.id, type: item.type })
		await loadFolders()
		await loadFiles(currentFolderId)
	}

	const handleRenameItem = async (item: FileItem, newName: string) => {
		await backend.renameItem({ id: item.id, type: item.type, newName })
		await loadFolders()
		await loadFiles(currentFolderId)
	}

	return (
		<div className="flex h-screen bg-gray-100">
			<div className="w-64 bg-white border-r overflow-auto">
				<div className="p-4 font-semibold border-b">Folders</div>
				<FolderTree
					folders={folders}
					currentFolderId={currentFolderId}
					onFolderSelect={handleFolderSelect}
				/>
			</div>

			<div className="flex-1 flex flex-col">
				<Toolbar onCreateFolder={handleCreateFolder} />
				<Breadcrumb path={path} onNavigate={handleFolderSelect} />

				<div className="flex-1 p-4 overflow-auto">
					{loading ? (
						<div className="text-center text-gray-500 py-8">Loading...</div>
					) : (
						<FileList
							files={files}
							onDelete={handleDeleteItem}
							onRename={handleRenameItem}
							onFolderOpen={(folder) => handleFolderSelect(folder.id, folder.name)}
						/>
					)}
				</div>
			</div>
		</div>
	)
}

export default App
