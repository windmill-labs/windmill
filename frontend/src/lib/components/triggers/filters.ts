/** The root's operator, stored separately as the trigger's `filter_logic`. */
export type FilterLogic = 'and' | 'or'
/** A nested group's operator. Only groups can negate; the root has nowhere to store it. */
export type GroupOp = FilterLogic | 'none'

/** How a leaf addresses its field: a top-level name, or a dotted path into nested objects. */
export type FieldMode = 'key' | 'path'
export type FilterKeyLeaf = { key: string; value: any }
export type FilterPathLeaf = { path: string; value: any }
export type FilterLeaf = FilterKeyLeaf | FilterPathLeaf
export type FilterAnyOf = { any_of: FilterNode[] }
export type FilterAllOf = { all_of: FilterNode[] }
export type FilterNoneOf = { none_of: FilterNode[] }
export type FilterGroup = FilterAnyOf | FilterAllOf | FilterNoneOf
export type FilterNode = FilterLeaf | FilterGroup

export function isFilterGroup(node: FilterNode): node is FilterGroup {
	return (
		node != null &&
		(Array.isArray((node as FilterAnyOf).any_of) ||
			Array.isArray((node as FilterAllOf).all_of) ||
			Array.isArray((node as FilterNoneOf).none_of))
	)
}

export function groupOp(group: FilterGroup): GroupOp {
	if ('any_of' in group) return 'or'
	if ('none_of' in group) return 'none'
	return 'and'
}

export function groupItems(group: FilterGroup): FilterNode[] {
	if ('any_of' in group) return group.any_of
	if ('none_of' in group) return group.none_of
	return group.all_of
}

export function fieldMode(leaf: FilterLeaf): FieldMode {
	return typeof (leaf as FilterPathLeaf).path === 'string' ? 'path' : 'key'
}

export function leafField(leaf: FilterLeaf): string {
	return (
		(fieldMode(leaf) === 'path' ? (leaf as FilterPathLeaf).path : (leaf as FilterKeyLeaf).key) ?? ''
	)
}

/** A leaf names its field under one key or the other, so switching rebuilds the object. */
export function makeLeaf(mode: FieldMode, field: string, value: any): FilterLeaf {
	return mode === 'path' ? { path: field, value } : { key: field, value }
}

/** Groups carry their operator in their key, so switching it rebuilds the object. */
export function makeGroup(op: GroupOp, items: FilterNode[]): FilterGroup {
	if (op === 'or') return { any_of: items }
	if (op === 'none') return { none_of: items }
	return { all_of: items }
}
