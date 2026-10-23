export function computeSecretUrl(secretUrl: string, workspace: string | undefined) {
	return `${window.location.origin}/public/${workspace}/${secretUrl}`
}
