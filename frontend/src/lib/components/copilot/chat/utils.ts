import { z } from 'zod'

const errorSchema = z.object({
	error: z.any()
})
export function getStringError(result: unknown): string | undefined {
	try {
		const parsed = errorSchema.parse(result)
		if (parsed.error) {
			return JSON.stringify(parsed.error, null, 2)
		} else {
			return undefined
		}
	} catch (_) {
		return undefined
	}
}
