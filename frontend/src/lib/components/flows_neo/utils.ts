export function getIndexes(parentIndex: number | undefined, childIndex: number): number[] {
	const indexes: number[] = []

	if (parentIndex !== undefined) {
		indexes.push(parentIndex)
	}
	indexes.push(childIndex)

	return indexes
}
