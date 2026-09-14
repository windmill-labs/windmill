import oauthConnectRegistry from '$oauth_connect_registry'

/**
 * Reads of the static OAuth connect registry (`backend/oauth_connect.json`), shared by the
 * connect dialog and by anything deciding whether to offer connecting at all.
 *
 * Kept here rather than inside `AppConnectInner` because two places have to agree on the
 * answer: the dialog decides whether a type gets the OAuth flow, and a caller deciding
 * whether to open the dialog has to reach the same verdict — or it offers Connect where the
 * dialog would fall back to a manual form, or hides it where the dialog would have worked.
 */

const SANDBOX_SUFFIX = '_sandbox'

export function stripSandboxSuffix(name: string): string {
	return name.endsWith(SANDBOX_SUFFIX) ? name.slice(0, -SANDBOX_SUFFIX.length) : name
}

/**
 * The registry entry for the first of `names` that has one. Callers pass the client name and
 * the resource type, which differ for sandbox clients (`salesforce_sandbox` vs `salesforce`);
 * both are resolved to the parent entry so a sandbox connection sees the same metadata.
 */
export function registryEntryFor(...names: (string | undefined)[]): any {
	const reg = oauthConnectRegistry as Record<string, any>
	for (const n of names) {
		if (!n) continue
		const entry = reg[stripSandboxSuffix(n)]
		if (entry) return entry
	}
	return undefined
}

/**
 * The registry declares this provider supports client credentials — which is what makes it
 * connectable with no OAuth client configured on the instance, since the credentials are
 * entered per resource rather than held by a superadmin.
 */
export function registryCcCapableFor(...names: (string | undefined)[]): boolean {
	return registryEntryFor(...names)?.grant_types?.includes('client_credentials') ?? false
}
