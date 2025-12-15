interface FileItem {
	id: string
	name: string
	type: 'file' | 'folder'
	size?: number
	modifiedAt: string
	parentId: string | null
}

// Mock file system data
const mockFiles: FileItem[] = [
	{ id: 'f1', name: 'Documents', type: 'folder', modifiedAt: '2024-01-15', parentId: null },
	{ id: 'f2', name: 'Images', type: 'folder', modifiedAt: '2024-01-10', parentId: null },
	{ id: 'f3', name: 'readme.txt', type: 'file', size: 1024, modifiedAt: '2024-01-20', parentId: null },
	{ id: 'f4', name: 'report.pdf', type: 'file', size: 52400, modifiedAt: '2024-01-18', parentId: 'f1' },
	{ id: 'f5', name: 'notes.txt', type: 'file', size: 256, modifiedAt: '2024-01-12', parentId: 'f1' },
	{ id: 'f6', name: 'photo1.jpg', type: 'file', size: 2048000, modifiedAt: '2024-01-08', parentId: 'f2' },
	{ id: 'f7', name: 'photo2.jpg', type: 'file', size: 1536000, modifiedAt: '2024-01-09', parentId: 'f2' },
	{ id: 'f8', name: 'Projects', type: 'folder', modifiedAt: '2024-01-05', parentId: 'f1' }
]

export async function main({ folderId }: { folderId: string | null }): Promise<FileItem[]> {
	return mockFiles.filter((file) => file.parentId === folderId)
}
