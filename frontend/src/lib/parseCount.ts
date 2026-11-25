/**
 * A whole number within `1..=max` stored as a JSON number or as a string of digits, `undefined`
 * for anything else. Must accept exactly what `parse_optional_count` (windmill-common) does.
 */
export function parseCount(value: unknown, max: number): number | undefined {
	const count =
		typeof value === 'number'
			? value
			: typeof value === 'string' && /^[+-]?\d+$/.test(value.trim())
				? Number(value.trim())
				: undefined
	return count !== undefined && Number.isInteger(count) && count >= 1 && count <= max
		? count
		: undefined
}
