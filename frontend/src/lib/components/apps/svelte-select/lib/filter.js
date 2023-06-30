export default function filter({
	loadOptions,
	filterText,
	items,
	value,
	itemId,
	groupBy,
	filterSelectedItems,
	itemFilter,
	convertStringItemsToObjects,
	filterGroupedItems,
	label
}) {
	if (items && loadOptions) return items
	if (!items) return []

	if (items && items.length > 0 && typeof items[0] !== 'object') {
		items = convertStringItemsToObjects(items)
	}

	let filterResults = items.filter((item) => {
		let matchesFilter = itemFilter(item[label], filterText, item)

		return matchesFilter
	})

	if (groupBy) {
		filterResults = filterGroupedItems(filterResults)
	}

	return filterResults
}
