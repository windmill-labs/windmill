function canonicalize(path: string): { path: string } | { error: string } {
	if (path.startsWith('/')) {
		return { error: `File path must be relative, without a leading /` }
	}
	const segments = path.split('/').filter((s) => s !== '' && s !== '.')
	if (segments.includes('..')) {
		return { error: `File path cannot contain ..` }
	}
	if (segments.length === 0) {
		return { error: `File name cannot be empty` }
	}
	return { path: segments.join('/') }
}

/**
 * The canonical spelling of a script module's path (the key of the module
 * bundle), or the reason it cannot be one.
 *
 * The worker resolves `.` and `//` away when it materialises the bundle into
 * the job directory, so `./dbt_project.yml` and `dbt_project.yml` are two keys
 * for one file on disk; the bundle is a Rust `HashMap`, so which content lands
 * there is undefined. Canonicalising before the duplicate and reserved-name
 * checks is what keeps them from being walked past.
 *
 * Redundant spellings are rewritten rather than refused: they name the file the
 * user meant, and the tree shows the canonical form regardless. `..` and
 * absolute paths name a file OUTSIDE the bundle, which has no canonical form
 * inside it, so they are refused here — the worker drops them, which would
 * otherwise show up as a file that was added and then silently never written.
 *
 * Surrounding whitespace is dropped because this is what someone typed into a
 * text box. A key already in a bundle gets no such courtesy — see
 * `findModulePathClash`.
 */
export function canonicalModulePath(path: string): { path: string } | { error: string } {
	return canonicalize(path.trim())
}

/**
 * The existing module key that would land on the same file as `canonicalPath`,
 * spelled as the bundle holds it (which is what the file tree shows).
 *
 * Keys already in the bundle are not canonical either: nothing on the push path
 * rewrites them, so a project imported with `./dbt_project.yml` in it must still
 * refuse a second `dbt_project.yml`. They are matched WITHOUT trimming, because
 * the worker does not trim path components: an imported `x.sql ` is its own file
 * on disk and must not stand in the way of adding `x.sql`.
 *
 * `ignoreKey` is the module being renamed. It has to be skipped inside the
 * search rather than compared against the result: a bundle can hold BOTH
 * spellings, and stopping at the renamed one would hide the other and let the
 * rename overwrite it.
 */
export function findModulePathClash(
	modules: Record<string, unknown> | null | undefined,
	canonicalPath: string,
	ignoreKey?: string
): string | undefined {
	return Object.keys(modules ?? {}).find((key) => {
		if (key === ignoreKey) return false
		const canonical = canonicalize(key)
		return 'path' in canonical && canonical.path === canonicalPath
	})
}
