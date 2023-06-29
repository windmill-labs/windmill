export default function filter({
	loadOptions,
	filterText,
	items,
	multiple,
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
		if (matchesFilter && multiple && value?.length) {
			matchesFilter = !value.some((x) => {
				return filterSelectedItems ? x[itemId] === item[itemId] : false
			})
		}

		return matchesFilter
	})

	if (groupBy) {
		filterResults = filterGroupedItems(filterResults)
	}

	return filterResults
}
