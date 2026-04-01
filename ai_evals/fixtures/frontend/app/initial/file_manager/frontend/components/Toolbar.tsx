import React, { useState } from 'react'

interface ToolbarProps {
	onCreateFolder: (name: string) => void
}

export const Toolbar: React.FC<ToolbarProps> = ({ onCreateFolder }) => {
	const [isCreating, setIsCreating] = useState(false)
	const [folderName, setFolderName] = useState('')

	const handleCreate = () => {
		if (folderName.trim()) {
			onCreateFolder(folderName.trim())
			setFolderName('')
			setIsCreating(false)
		}
	}

	return (
		<div className="px-4 py-3 bg-white border-b flex items-center gap-4">
			{isCreating ? (
				<div className="flex items-center gap-2">
					<input
						type="text"
						value={folderName}
						onChange={(e) => setFolderName(e.target.value)}
						onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
						placeholder="Folder name"
						className="border rounded px-3 py-1"
						autoFocus
					/>
					<button
						onClick={handleCreate}
						className="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
					>
						Create
					</button>
					<button
						onClick={() => {
							setIsCreating(false)
							setFolderName('')
						}}
						className="px-3 py-1 text-gray-600 hover:text-gray-800"
					>
						Cancel
					</button>
				</div>
			) : (
				<button
					onClick={() => setIsCreating(true)}
					className="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 flex items-center gap-2"
				>
					<span>+</span>
					New Folder
				</button>
			)}
		</div>
	)
}
