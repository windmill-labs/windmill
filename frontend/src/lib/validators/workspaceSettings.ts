import { z } from 'zod'

// Deploy path filter validation
const deployPathFilterSchema = z
	.string()
	.min(1, 'Path filter cannot be empty')
	.refine(
		(val) => {
			// Allow alphanumeric characters, underscores, slashes, asterisks, and hyphens
			// But don't allow consecutive slashes or ending with slash
			const validChars = /^[a-zA-Z0-9/_*-]+$/
			const noConsecutiveSlashes = !/\/\//.test(val)
			const noEndingSlash = !val.endsWith('/')
			const noEndingDash = !val.endsWith('-')

			return validChars.test(val) && noConsecutiveSlashes && noEndingSlash && noEndingDash
		},
		{
			message:
				'Path filter contains invalid characters or format. Allowed: letters, numbers, /, _, -, *. Cannot end with / or -'
		}
	)
	.refine(
		(val) => {
			// Validate glob patterns - * and ** are allowed, but *** is not
			const invalidGlobPattern = /\*{3,}/.test(val)
			return !invalidGlobPattern
		},
		{
			message:
				'Invalid glob pattern. Use * for single level wildcard or ** for multi-level wildcard'
		}
	)

// Webhook URL validation
const webhookUrlSchema = z
	.string()
	.refine(
		(val) => {
			if (!val || val.trim() === '') return true // Allow empty
			try {
				new URL(val)
				return true
			} catch {
				return false
			}
		},
		{
			message: 'Please enter a valid URL (e.g., https://example.com/webhook)'
		}
	)
	.optional()
	.or(z.literal(''))

// Workspace encryption key validation
const encryptionKeySchema = z.string().regex(/^[a-zA-Z0-9]{64}$/, {
	message: 'Encryption key must be exactly 64 characters long and contain only letters and numbers'
})

export function validateDeployPathFilters(paths: string[]): {
	isValid: boolean
	errors: Record<number, string>
} {
	const errors: Record<number, string> = {}

	paths.forEach((path, index) => {
		const result = deployPathFilterSchema.safeParse(path)
		if (!result.success) {
			errors[index] = result.error.issues[0]?.message || 'Invalid path filter'
		}
	})

	return {
		isValid: Object.keys(errors).length === 0,
		errors
	}
}

export function validateWebhookUrl(url: string): {
	isValid: boolean
	error?: string
} {
	const result = webhookUrlSchema.safeParse(url)
	return {
		isValid: result.success,
		error: result.success ? undefined : result.error.issues[0]?.message || 'Invalid URL'
	}
}

export function validateEncryptionKey(key: string): {
	isValid: boolean
	error?: string
} {
	const result = encryptionKeySchema.safeParse(key)
	return {
		isValid: result.success,
		error: result.success ? undefined : result.error.issues[0]?.message || 'Invalid encryption key'
	}
}
