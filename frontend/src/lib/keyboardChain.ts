export type NavigationChain = Record<string, { upId?: string | null; downId?: string | null }>

// Update navigation chain: remove old chain between first/last, insert new chain
export function updateLinks(navigationChain: NavigationChain, newChain: NavigationChain) {
	if (Object.keys(newChain).length === 0) return navigationChain

	const firstLink = Object.keys(newChain).find((link) => newChain[link].upId === null)
	const lastLink = Object.keys(newChain).find((link) => newChain[link].downId === null)

	//If the chain has no end or start, return the navigationChain
	if (!firstLink || !lastLink) return navigationChain

	const cleaned = { ...navigationChain }
	const inserted = { ...newChain }

	// Check if the chain needs to be inserted
	if (!cleaned[lastLink] && cleaned[firstLink]) {
		// connect the beginning of the new chain to the beginning of the chain
		inserted[firstLink].upId = cleaned[firstLink]?.upId

		//connect the end of new the new chain to the beginning of the chain
		const nextLink = cleaned[firstLink]?.downId
		if (nextLink) {
			inserted[lastLink].downId = nextLink
			cleaned[nextLink].upId = lastLink
		}
	} else if (cleaned[lastLink] && cleaned[firstLink]) {
		// If the end of the chain matches a link in the navigationChain, and the beginning delete the chain in the middle and replace it with the new chain
		inserted[firstLink].upId = cleaned[firstLink]?.upId
		inserted[lastLink].downId = cleaned[lastLink]?.downId

		if (firstLink !== lastLink) {
			const toDelete: string[] = []

			let current = cleaned[firstLink]?.downId
			while (current && current !== lastLink && !toDelete.includes(current)) {
				toDelete.push(current)
				current = cleaned[current]?.downId
			}
			toDelete.forEach((itemId) => {
				delete cleaned[itemId]
			})
		}
	}

	return { ...cleaned, ...inserted }
}
