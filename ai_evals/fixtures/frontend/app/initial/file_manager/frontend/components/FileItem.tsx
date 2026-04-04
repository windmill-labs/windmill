import React, { useState } from 'react'
import type { FileItem as FileItemType } from '../index'

interface FileItemProps {
	item: FileItemType
	onDelete: (item: FileItemType) => void
	onRename: (item: FileItemType, newName: string) => void
	onFolderOpen: (folder: FileItemType) => void
}

export const FileItem: React.FC<FileItemProps> = ({ item, onDelete, onRename, onFolderOpen }) => {
	const [isRenaming, setIsRenaming] = useState(false)
	const [newName, setNewName] = useState(item.name)

	const handleRename = () => {
		if (newName.trim() && newName !== item.name) {
			onRename(item, newName.trim())
		}
		setIsRenaming(false)
	}

	const handleDoubleClick = () => {
		if (item.type === 'folder') {
			onFolderOpen(item)
		}
	}

	const icon = item.type === 'folder' ? '📁' : '📄'
	const formattedDate = new Date(item.modifiedAt).toLocaleDateString()

	return (
		<div
			className="grid grid-cols-12 gap-4 px-4 py-2 border-b hover:bg-gray-50 items-center"
			onDoubleClick={handleDoubleClick}
		>
			<div className="col-span-6 flex items-center gap-2">
				<span>{icon}</span>
				{isRenaming ? (
					<input
						type="text"
						value={newName}
						onChange={(e) => setNewName(e.target.value)}
						onBlur={handleRename}
						onKeyDown={(e) => e.key === 'Enter' && handleRename()}
						className="border rounded px-2 py-1 flex-1"
						autoFocus
					/>
				) : (
					<span
						className={item.type === 'folder' ? 'cursor-pointer hover:text-blue-600' : ''}
					>
						{item.name}
					</span>
				)}
			</div>

			<div className="col-span-2 text-gray-500 text-sm">
				{item.type === 'folder' ? 'Folder' : 'File'}
			</div>

			<div className="col-span-2 text-gray-500 text-sm">{formattedDate}</div>

			<div className="col-span-2 flex gap-2">
				<button
					onClick={() => setIsRenaming(true)}
					className="text-blue-500 hover:text-blue-700 text-sm"
				>
					Rename
				</button>
				<button
					onClick={() => onDelete(item)}
					className="text-red-500 hover:text-red-700 text-sm"
				>
					Delete
				</button>
			</div>
		</div>
	)
}
