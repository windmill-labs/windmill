/** A git repository resource value, as far as this predicate cares. */
type GitRepositoryValue = { url?: string; managed_credential?: string } | undefined | null

/** True when the remote authenticates itself, i.e. it carries a `user@` or
 * `user:password@` userinfo component. Mirrors the server's rule, which is what
 * decides whether it attaches the stored credential at all. */
function urlCarriesCredential(url: string | undefined): boolean {
	return /:\/\/[^/@]+@/.test(url ?? '')
}

/**
 * The host whose token Windmill holds for this repository, or undefined when it
 * holds none.
 *
 * A URL that carries its own credential wins over the marker: the server skips
 * the stored credential for such a URL, so honouring a stale marker here would
 * have the UI promise renewal for a token nothing renews. That happens whenever
 * someone puts a token back in the URL without clearing the marker, which is why
 * this is checked rather than trusting the marker alone.
 */
export function managedCredentialHost(value: GitRepositoryValue): string | undefined {
	const host = value?.managed_credential
	if (!host || host === 'none') return undefined
	return urlCarriesCredential(value?.url) ? undefined : host
}
