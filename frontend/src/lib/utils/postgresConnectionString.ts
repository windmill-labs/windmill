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
 *
 * libpq is the arbiter of what a connection string means, so this follows it rather than
 * RFC 3986 where they differ: credentials are split at the *first* `@` -- an unencoded one
 * lands in the host for libpq too -- and percent escapes in them are decoded, so `p%40ss`
 * authenticates as `p@ss`.
 */

/**
 * The host alternation is what admits IPv6: a literal address is full of colons, so a URI
 * has to bracket it (`@[2001:db8::1]:5432/`) and the brackets are what tell the port apart
 * from the address. Brackets are stripped on the way in and added back on the way out, so
 * what is stored is the bare address a Postgres client wants.
 */
const CONNECTION_STRING =
	/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>\[[^\]]+\]|[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?(?:\?.*sslmode=(?<sslmode>[^&]+))?/

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
		host: host.startsWith('[') ? host.slice(1, -1) : host,
		port: port ? Number(port) : undefined,
		dbname: dbname ? decode(dbname) : undefined,
		sslmode: sslmode || undefined
	}
}

/** The only query parameter the `postgresql` resource has a field for. */
const REPRESENTABLE_PARAMS = ['sslmode']

/**
 * Parameters that change nothing about what the connection reaches, how it is secured, or how
 * it behaves, so losing them costs the user nothing. `connect_timeout` is deliberately not one
 * of them: the backend applies its own fixed timeout, so honouring it is not on offer.
 */
const COSMETIC_PARAMS = ['application_name']

/**
 * The name of a parameter this string carries that the resource cannot honour. An allowlist,
 * not a list of known-bad names: libpq keeps adding parameters, and the ones that matter are
 * the ones that would be missed. Dropping one silently saves a connection weaker or simply
 * other than the one pasted, behind a probe that reports success.
 */
export function unsupportedConnectionParam(connectionString: string): string | undefined {
	const query = connectionString.split('?').slice(1).join('?')
	if (!query) return undefined
	let found: string | undefined = undefined
	new URLSearchParams(query).forEach((_value, name) => {
		const known = REPRESENTABLE_PARAMS.includes(name.toLowerCase())
		const harmless = COSMETIC_PARAMS.includes(name.toLowerCase())
		if (!found && !known && !harmless) found = name
	})
	return found
}

/**
 * Every part that was set is emitted, `sslmode` included. Leaving `prefer` out because it is
 * libpq's own default would be shorter, but it does not survive the trip: a caller that
 * reparses this string gets `undefined` back and substitutes its own default, which is how an
 * explicit `prefer` silently became `require`. Whatever this produces, `parse` must read back.
 */
export function composePostgresConnectionString(parts: PostgresConnectionParts): string {
	const credentials = parts.password
		? `${encodeURIComponent(parts.user)}:${encodeURIComponent(parts.password)}`
		: encodeURIComponent(parts.user)
	const port = parts.port ? `:${parts.port}` : ''
	const query = parts.sslmode ? `?sslmode=${parts.sslmode}` : ''
	const dbname = parts.dbname ? encodeURIComponent(parts.dbname) : ''
	// A bare IPv6 address would put its own colons where the port separator goes.
	const host =
		parts.host.includes(':') && !parts.host.startsWith('[') ? `[${parts.host}]` : parts.host
	return `postgres://${credentials}@${host}${port}/${dbname}${query}`
}
