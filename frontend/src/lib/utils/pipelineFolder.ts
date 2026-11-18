/**
 * The bare folder name a pipeline is scoped to, from either form a caller may hold
 * (`analytics` or the owner path `f/analytics`). Every pipeline surface keys on the
 * NAME and builds `f/<folder>/<node>` paths from it, so a value that still carries
 * the prefix double-prefixes every path built from it. A folder name cannot contain
 * `/`, so a leading `f/` is unambiguously the prefix.
 */
export function normalizePipelineFolder(folder: string): string {
	return folder
		.trim()
		.replace(/^\/+|\/+$/g, '')
		.replace(/^f\//, '')
}
