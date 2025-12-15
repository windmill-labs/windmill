import React from 'react'
import { FileItem as FileItemComponent } from './FileItem'
import type { FileItem } from '../index'

interface FileListProps {
	files: FileItem[]
	onDelete: (item: FileItem) => void
	onRename: (item: FileItem, newName: string) => void
	onFolderOpen: (folder: FileItem) => void
}

export const FileList: React.FC<FileListProps> = ({ files, onDelete, onRename, onFolderOpen }) => {
	if (files.length === 0) {
		return (
			<div className="text-center text-gray-500 py-8">This folder is empty</div>
		)
	}

	// Sort: folders first, then files
	const sortedFiles = [...files].sort((a, b) => {
		if (a.type === 'folder' && b.type !== 'folder') return -1
		if (a.type !== 'folder' && b.type === 'folder') return 1
		return a.name.localeCompare(b.name)
	})

	return (
		<div className="bg-white rounded-lg border">
			<div className="grid grid-cols-12 gap-4 px-4 py-2 border-b bg-gray-50 font-medium text-sm text-gray-600">
				<div className="col-span-6">Name</div>
				<div className="col-span-2">Type</div>
				<div className="col-span-2">Modified</div>
				<div className="col-span-2">Actions</div>
			</div>

			{sortedFiles.map((file) => (
				<FileItemComponent
					key={file.id}
					item={file}
					onDelete={onDelete}
					onRename={onRename}
					onFolderOpen={onFolderOpen}
				/>
			))}
		</div>
	)
}
