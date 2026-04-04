interface Folder {
	id: string
	name: string
	parentId: string | null
}

export async function main({
	name,
	parentId
}: {
	name: string
	parentId: string | null
}): Promise<Folder> {
	// In a real implementation, this would create a folder in storage
	const newFolder: Folder = {
		id: `folder_${Date.now()}`,
		name,
		parentId
	}

	return newFolder
}
