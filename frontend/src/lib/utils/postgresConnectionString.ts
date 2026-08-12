/**
 * `postgres://user:password@host:5432/dbname?sslmode=require` in both directions.
 *
 * Shared by the resource form and the data table wizard: both turn a pasted
 * connection string into a `postgresql` resource value, and the two drifting
 * apart would mean the same string produced two different resources.
 *
 * The wizard offers the same connection as a string or as fields and lets the
 * user switch, so parse and compose have to be inverses: whatever one produces,
 * the other must read back unchanged.
 */

const CONNECTION_STRING =
	/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?(?:\?.*sslmode=(?<sslmode>[^&]+))?/

/**
 * A database someone types into Windmill is almost never localhost, so callers ask for TLS
 * where libpq would settle for `prefer`. A string that names its own `sslmode` keeps it.
 */
export const DEFAULT_SSLMODE = 'require'

export type PostgresConnectionParts = {
	user: string
	password?: string
	host: string
	port?: number
	dbname?: string
	sslmode?: string
}

/** A lone `%` is not an escape, and a password is free to contain one. */
function decode(value: string): string {
	try {
		return decodeURIComponent(value)
	} catch {
		return value
	}
}

/** Undefined when the string is not a postgres URI. */
export function parsePostgresConnectionString(
	connectionString: string
): PostgresConnectionParts | undefined {
	const match = connectionString.match(CONNECTION_STRING)
	if (!match?.groups) return undefined
	const { user, password, host, port, dbname, sslmode } = match.groups
	return {
		user: decode(user),
		password: password ? decode(password) : undefined,
		host,
		port: port ? Number(port) : undefined,
		dbname: dbname || undefined,
		sslmode: sslmode || undefined
	}
}

/**
 * `sslmode` is emitted only when it differs from libpq's own default, so the
 * string stays the short one people recognize when nothing was overridden.
 */
export function composePostgresConnectionString(parts: PostgresConnectionParts): string {
	const credentials = parts.password
		? `${encodeURIComponent(parts.user)}:${encodeURIComponent(parts.password)}`
		: encodeURIComponent(parts.user)
	const port = parts.port ? `:${parts.port}` : ''
	const query = parts.sslmode && parts.sslmode !== 'prefer' ? `?sslmode=${parts.sslmode}` : ''
	return `postgres://${credentials}@${parts.host}${port}/${parts.dbname ?? ''}${query}`
}
