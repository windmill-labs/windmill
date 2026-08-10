export type FilterLogic = 'and' | 'or'

export type FilterLeaf = { key: string; value: any }
export type FilterAnyOf = { any_of: FilterNode[] }
export type FilterAllOf = { all_of: FilterNode[] }
export type FilterGroup = FilterAnyOf | FilterAllOf
export type FilterNode = FilterLeaf | FilterGroup

export function isFilterGroup(node: FilterNode): node is FilterGroup {
	return (
		node != null &&
		(Array.isArray((node as FilterAnyOf).any_of) || Array.isArray((node as FilterAllOf).all_of))
	)
}

export function groupLogic(group: FilterGroup): FilterLogic {
	return 'any_of' in group ? 'or' : 'and'
}

export function groupItems(group: FilterGroup): FilterNode[] {
	return 'any_of' in group ? group.any_of : group.all_of
}

/** Groups carry their operator in their key, so switching it rebuilds the object. */
export function makeGroup(logic: FilterLogic, items: FilterNode[]): FilterGroup {
	return logic === 'or' ? { any_of: items } : { all_of: items }
}
