export function processItems<Item extends { label?: string; value: any }>({
	items,
	filterText,
	groupBy,
	sortBy,
	onCreateItem,
	createText
}: {
	items?: Item[]
	filterText?: string
	groupBy?: (item: Item) => string
	sortBy?: (a: Item, b: Item) => number
	onCreateItem?: (value: string) => void
	createText?: string
}): ProcessedItem<Item['value']>[] {
	let items2 =
		items?.map((item) => ({
			...item,
			label: getLabel(item)
		})) ?? []
	if (groupBy) {
		items2 =
			items2?.map((item) => ({
				...item,
				__select_group: groupBy(item)
			})) ?? []
	}
	if (sortBy) {
		items2 = items2?.sort(sortBy)
	}
	if (onCreateItem && filterText && !items2.some((item) => item.label === filterText)) {
		items2.push({
			label: createText ?? `Add new: "${filterText}"`,
			value: filterText,
			__is_create: true
		} as any)
	}
	return items2
}

export type ProcessedItem<T> = {
	__select_group?: string
	__is_create?: true
	label: string
	value: T
	subtitle?: string
}

export function getLabel<T>(item: { label?: string; value: T } | undefined): string {
	if (!item) return ''
	if (item.label) return item.label
	if (typeof item.value === 'string') return item.value
	if (typeof item.value == 'number' || typeof item.value == 'boolean') return item.value.toString()

	return JSON.stringify(item.value)
}

export function safeSelectItems<T>(
	list: (T | { value: T; label?: string } | undefined | null)[] | undefined | null
): { value: T; label?: string }[] {
	if (!list) return []
	return list
		.filter((item) => item !== undefined && item !== null)
		.map((item) => {
			if (typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean') {
				return { value: item }
			}
			if (typeof item === 'object' && 'value' in item) {
				return item
			}
			return { value: null as any, label: 'UNKNOWN_ITEM' }
		})
}
