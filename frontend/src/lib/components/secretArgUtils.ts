import type { Schema } from '$lib/common'
import { VariableService } from '$lib/gen'
import { get } from 'svelte/store'
import { userStore, workspaceStore } from '$lib/stores'
import { generateRandomString } from '$lib/utils'
import { isSecretProp, mapArgLeaves } from './job_args'

/** Where a caller's own ephemeral secrets live, so a field can tell one it minted from a
 * workspace variable someone linked by hand. */
export function ephemeralSecretPrefix(username: string): string {
	return `u/${username}/secret_arg/`
}

/**
 * Mint the ephemeral secret variable a sensitive argument is submitted as, and return its path.
 * It expires on its own, so a run that is abandoned leaves no permanent secret behind.
 */
export async function mintEphemeralSecret(
	workspace: string,
	username: string,
	value: string
): Promise<string> {
	const path = ephemeralSecretPrefix(username) + generateRandomString(12)
	await VariableService.createVariable({
		workspace,
		requestBody: {
			value,
			is_secret: true,
			path,
			description: 'Ephemeral secret variable',
			expires_at: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7).toISOString()
		}
	})
	return path
}

/** `$var:` hands the job the variable's text and `$jsonvar:` hands it the parsed value, so a
 * field that cannot hold a string needs the second one whichever the caller named. */
function referencePrefix(prop: any, value: unknown): '$var:' | '$jsonvar:' {
	return typeof value === 'string' && prop?.type !== 'object' && prop?.type !== 'array'
		? '$var:'
		: '$jsonvar:'
}

/**
 * Turn every sensitive argument into a reference before the job is submitted: a plaintext value
 * is minted into an ephemeral secret variable, so what is stored on the job — and readable by
 * anyone who can see its run — names a secret instead of holding one.
 *
 * The single place that decides how a secret reaches a job: {@link PasswordArgInput} mints
 * through it while the user types, and a run the autonomy posture starts without a form calls it
 * in the widget's stead.
 */
export async function processSecretArgs(
	args: Record<string, any>,
	schema: Schema | undefined,
	// Workspace the ephemeral secret variable is created in — must match the
	// workspace the preview job runs in, else $jsonvar: resolves to a missing var.
	forceWorkspace?: string
): Promise<Record<string, any>> {
	if (!schema?.properties) return args

	const workspace = forceWorkspace ?? get(workspaceStore)
	const user = get(userStore)
	if (!workspace || !user) return args

	const username = (user.username ?? user.email)?.split('@')[0]
	if (!username) return args

	// A value that already names a variable is one; anything else is the secret itself. An empty
	// field holds nothing to mint, and ArgInput synthesises '' for every untouched string.
	const holdsSecret = (value: unknown) =>
		value != null &&
		value !== '' &&
		!(
			typeof value === 'string' &&
			(value.startsWith('$var:') || value.startsWith('$jsonvar:') || value.startsWith('$res:'))
		)

	// Collected first and substituted after, because the walk is synchronous and minting is not.
	// Keyed by the whole path the walk reports, which is what tells two same-named leaves apart.
	const pending: { key: string; prop: any; value: unknown }[] = []
	mapArgLeaves(args, schema as any, isSecretProp, (value, prop, path) => {
		if (holdsSecret(value)) pending.push({ key: JSON.stringify(path), prop, value })
		return value
	})

	const minted = new Map<string, string>()
	for (const { key, prop, value } of pending) {
		const reference = referencePrefix(prop, value)
		const variable = await mintEphemeralSecret(
			workspace,
			username,
			reference === '$var:' ? String(value) : JSON.stringify(value)
		)
		minted.set(key, reference + variable)
	}

	return mapArgLeaves(args, schema as any, isSecretProp, (value, prop, path) => {
		const replacement = minted.get(JSON.stringify(path))
		if (replacement !== undefined) return replacement
		// A plain variable named for a field that cannot hold a string: the caller meant that
		// variable's contents, which is the same secret read the way the field needs it.
		if (
			typeof value === 'string' &&
			value.startsWith('$var:') &&
			referencePrefix(prop, value) === '$jsonvar:'
		) {
			return '$jsonvar:' + value.slice('$var:'.length)
		}
		return value
	})
}
