export default async function getItems({
	dispatch,
	loadOptions,
	convertStringItemsToObjects,
	filterText
}) {
	let res = await loadOptions(filterText).catch((err) => {
		console.warn('svelte-select loadOptions error :>> ', err)
		dispatch('error', { type: 'loadOptions', details: err })
	})

	if (res && !res.cancelled) {
		if (res) {
			if (res && res.length > 0 && typeof res[0] !== 'object') {
				res = convertStringItemsToObjects(res)
			}

			dispatch('loaded', { items: res })
		} else {
			res = []
		}

		return {
			filteredItems: res,
			loading: false,
			focused: true,
			listOpen: true
		}
	}
}
