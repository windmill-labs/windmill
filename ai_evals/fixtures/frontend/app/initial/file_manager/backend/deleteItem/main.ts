export async function main({
	id,
	type
}: {
	id: string
	type: 'file' | 'folder'
}): Promise<{ success: boolean }> {
	// In a real implementation, this would delete from storage
	console.log(`Deleting ${type} with id: ${id}`)

	return { success: true }
}
