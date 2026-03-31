import type { Schema } from '$lib/common'
import { VariableService } from '$lib/gen'
import { get } from 'svelte/store'
import { userStore, workspaceStore } from '$lib/stores'
import { generateRandomString } from '$lib/utils'

/**
 * Process args before job submission: for non-string fields marked as password/sensitive,
 * create ephemeral secret variables and replace values with $jsonvar:path references.
 * String password fields are already handled by PasswordArgInput (uses $var:).
 */
export async function processSecretArgs(
	args: Record<string, any>,
	schema: Schema | undefined
): Promise<Record<string, any>> {
	if (!schema?.properties) return args

	const workspace = get(workspaceStore)
	const user = get(userStore)
	if (!workspace || !user) return args

	const username = (user.username ?? user.email)?.split('@')[0]
	if (!username) return args
	const userPrefix = `u/${username}/secret_arg/`

	const result = { ...args }

	for (const [key, prop] of Object.entries(schema.properties)) {
		if (!prop.password) continue
		if (prop.type !== 'object') continue // only object types; strings handled by PasswordArgInput
		if (result[key] == null || result[key] === undefined) continue
		if (typeof result[key] === 'string' && result[key].startsWith('$jsonvar:')) continue // already processed

		const path = userPrefix + generateRandomString(12)
		await VariableService.createVariable({
			workspace,
			requestBody: {
				value: JSON.stringify(result[key]),
				is_secret: true,
				path,
				description: 'Ephemeral secret variable',
				expires_at: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7).toISOString()
			}
		})
		result[key] = '$jsonvar:' + path
	}

	return result
}
