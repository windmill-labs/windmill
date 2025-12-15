interface Folder {
	id: string
	name: string
	parentId: string | null
	children: Folder[]
}

// Mock folder structure
const mockFolders: Folder[] = [
	{
		id: 'f1',
		name: 'Documents',
		parentId: null,
		children: [
			{
				id: 'f8',
				name: 'Projects',
				parentId: 'f1',
				children: []
			}
		]
	},
	{
		id: 'f2',
		name: 'Images',
		parentId: null,
		children: []
	}
]

export async function main(): Promise<Folder[]> {
	return mockFolders
}
