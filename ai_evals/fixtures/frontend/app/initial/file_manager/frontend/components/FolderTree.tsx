import React from 'react'
import type { Folder } from '../index'

interface FolderTreeProps {
	folders: Folder[]
	currentFolderId: string | null
	onFolderSelect: (folderId: string | null, folderName: string) => void
	depth?: number
}

export const FolderTree: React.FC<FolderTreeProps> = ({
	folders,
	currentFolderId,
	onFolderSelect,
	depth = 0
}) => {
	return (
		<div>
			{depth === 0 && (
				<div
					className={`px-4 py-2 cursor-pointer hover:bg-gray-100 ${
						currentFolderId === null ? 'bg-blue-100 text-blue-700' : ''
					}`}
					onClick={() => onFolderSelect(null, 'Root')}
				>
					<span className="mr-2">📁</span>
					Root
				</div>
			)}

			{folders.map((folder) => (
				<div key={folder.id}>
					<div
						className={`px-4 py-2 cursor-pointer hover:bg-gray-100 ${
							currentFolderId === folder.id ? 'bg-blue-100 text-blue-700' : ''
						}`}
						style={{ paddingLeft: `${(depth + 1) * 16 + 16}px` }}
						onClick={() => onFolderSelect(folder.id, folder.name)}
					>
						<span className="mr-2">📁</span>
						{folder.name}
					</div>

					{folder.children && folder.children.length > 0 && (
						<FolderTree
							folders={folder.children}
							currentFolderId={currentFolderId}
							onFolderSelect={onFolderSelect}
							depth={depth + 1}
						/>
					)}
				</div>
			))}
		</div>
	)
}
