// A secret variable whose stored value is `$external:<scheme>://<path>#<key>` is resolved
// from an external secret store each time it is read; the backend parses and authorizes it
// (`secret_backend/external_ref.rs`). Only Vault is supported.
export const EXTERNAL_REF_PREFIX = '$external:'
const VAULT_PREFIX = `${EXTERNAL_REF_PREFIX}vault://`

export function isExternalRef(value: string | undefined | null): boolean {
	return !!value?.startsWith(EXTERNAL_REF_PREFIX)
}

export function parseVaultRef(value: string): { path: string; key: string } {
	const rest = value.startsWith(VAULT_PREFIX) ? value.slice(VAULT_PREFIX.length) : ''
	const i = rest.indexOf('#')
	return i < 0 ? { path: rest, key: '' } : { path: rest.slice(0, i), key: rest.slice(i + 1) }
}

export function formatVaultRef(path: string, key: string): string {
	return `${VAULT_PREFIX}${path}#${key}`
}

/** "Vault · secret/team/app#token", for lists. */
export function describeExternalRef(value: string): string {
	const { path, key } = parseVaultRef(value)
	return `Vault · ${path}#${key}`
}
