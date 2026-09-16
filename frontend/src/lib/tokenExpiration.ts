/** Largest `max_token_expiration_days` read as a ceiling, about 2,700 years. Mirrors
 * `MAX_TOKEN_EXPIRATION_DAYS_BOUND` in windmill-common. */
export const MAX_TOKEN_EXPIRATION_DAYS_BOUND = 1_000_000

/**
 * The ceiling a stored `max_token_expiration_days` value sets, or `undefined` for none.
 *
 * Must accept exactly what `cap_token_expiration` (windmill-api-users) accepts: a whole number
 * of days within the bound, stored as a JSON number or as a string of digits. Anything looser
 * and the token form hides "No expiration" on an instance whose server caps nothing.
 */
export function parseMaxTokenExpirationDays(value: unknown): number | undefined {
	const days =
		typeof value === 'number'
			? value
			: typeof value === 'string' && /^[+-]?\d+$/.test(value.trim())
				? Number(value.trim())
				: undefined
	return days !== undefined &&
		Number.isInteger(days) &&
		days >= 1 &&
		days <= MAX_TOKEN_EXPIRATION_DAYS_BOUND
		? days
		: undefined
}
