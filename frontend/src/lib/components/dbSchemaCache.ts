import type { DbInput } from './dbTypes'

/** What identifies a database's schema, role included: two roles on one data table may reach
 * different schemas, so they cannot share a cache entry. Never throws, since it keys derived
 * state; the connection itself is what refuses an invalid role. */
export function getDbSchemasPath(input: DbInput): string {
	switch (input.type) {
		case 'database':
			return input.role !== undefined && input.resourcePath.startsWith('datatable://')
				? `${input.resourcePath}?role=${input.role}`
				: input.resourcePath
		case 'ducklake':
			return 'ducklake://' + input.ducklake
	}
}

/** Scoped by the acting workspace: a data table of the same name can exist in both the nav and
 * the acting workspace, and one's schema must not be reused for the other. */
export function schemaCacheKey(workspace: string | undefined, input: DbInput): string {
	return `${workspace}:${getDbSchemasPath(input)}`
}
