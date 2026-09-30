/** A directory level of an item's path, with the full path down to it. */
export type PathDir = { name: string; fullPath: string }

/**
 * Splits `f/demo/weather_report` into the levels a breadcrumb walks: the scope (`f/<folder>` or
 * `u/<user>`), then each subfolder, then the item itself. Each level carries the cumulative path,
 * which is what the picker opens and highlights.
 *
 * Returns undefined for anything shorter than scope + name, such as a path still being typed.
 */
export function splitItemPath(path: string): { dirs: PathDir[]; leaf: PathDir } | undefined {
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
	return { dirs, leaf: { name: slug[slug.length - 1], fullPath: path } }
}
