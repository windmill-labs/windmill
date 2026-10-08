/** A directory level of an item's path, with the full path down to it. */
export type PathDir = { name: string; fullPath: string }

/**
 * Splits `f/demo/weather_report` into the levels a breadcrumb walks: the scope (`f/<folder>` or
 * `u/<user>`), then each subfolder, then the item itself. Each level carries the cumulative path,
 * which is what the picker opens and highlights.
 *
 * Returns undefined before there is even a scope. The leaf is undefined while the path names a
 * folder but no item yet (`f/demo/`) — the levels it does name still come back.
 */
export function splitItemPath(
	path: string
): { dirs: PathDir[]; leaf: PathDir | undefined } | undefined {
	const parts = path.split('/')
	if (parts.length < 3) return undefined
	const scope = parts.slice(0, 2).join('/')
	const slug = parts.slice(2)
	const dirs: PathDir[] = [{ name: scope, fullPath: scope }]
	let acc = scope
	for (let i = 0; i < slug.length - 1; i++) {
		acc = `${acc}/${slug[i]}`
		dirs.push({ name: slug[i], fullPath: acc })
	}
	// A leaf with an empty name draws a blank segment. The editor binds the path live, so the
	// breadcrumb sees every keystroke: the folders stay put while the name is being typed rather
	// than the whole trail dropping out and coming back.
	const name = slug[slug.length - 1]
	return { dirs, leaf: name ? { name, fullPath: path } : undefined }
}
