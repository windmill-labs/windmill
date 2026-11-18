type InsertField = { name: string; datatype: string }

const Q = `["'\`]`
const NOT_Q = `[^"'\`]*`

function rejectedValue(message: string): { value: string; type: string } | undefined {
	const parse = message.match(new RegExp(`cannot parse ${Q}(${NOT_Q})${Q} as ([\\w ]+)`, 'i'))
	if (parse) return { value: parse[1], type: parse[2] }
	const syntax = message.match(
		new RegExp(`invalid input syntax for type ([\\w ]+): ${Q}(${NOT_Q})${Q}`, 'i')
	)
	if (syntax) return { value: syntax[2], type: syntax[1] }
	return undefined
}

/**
 * Best-effort guess at which form fields an insert error is about. Database errors name a column
 * ("null value in column \"x\"", "Column 'x' cannot be null"), while argument conversion errors
 * only quote the offending value ("Cannot parse '' as date"), so fall back to the fields holding
 * that value, narrowed to those whose type matches the one the message names.
 */
export function fieldsBlamedByInsertError(
	message: string,
	fields: InsertField[],
	args: Record<string, any>
): string[] {
	const quoted = [...message.matchAll(new RegExp(`${Q}(${NOT_Q})${Q}`, 'g'))].map((m) => m[1])
	const named = fields.filter((f) => quoted.includes(f.name)).map((f) => f.name)
	if (named.length > 0) return named

	const rejected = rejectedValue(message)
	if (!rejected) return []
	const holding = fields.filter((f) => {
		const v = args[f.name]
		return (
			(typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') &&
			String(v) === rejected.value
		)
	})
	const type = rejected.type.trim().toLowerCase()
	const ofType = holding.filter((f) => f.datatype.toLowerCase().startsWith(type))
	return (ofType.length > 0 ? ofType : holding).map((f) => f.name)
}
