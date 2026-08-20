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
	/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>\[[^\]]+\]|[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?/

/**
 * The query parameters, read the way libpq reads them: names are case-sensitive — `SslMode` is
 * rejected outright as an invalid URI query parameter, not folded to `sslmode` — and a name
 * repeated takes its last value. One reader for both the parser and the allowlist below, or
 * they disagree about what a string says and a name is refused by neither and honoured by
 * neither.
 */
function paramsOf(connectionString: string): Map<string, string> {
	const query = connectionString.split('?').slice(1).join('?')
	const params = new Map<string, string>()
	if (!query) return params
	new URLSearchParams(query).forEach((value, name) => params.set(name, value))
	return params
}

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
	const { user, password, host, port, dbname } = match.groups
	// By parameter name, never by searching the query text: `sslmode=` also occurs inside
	// another parameter's *value*, and a substring match there reads someone's
	// `application_name=sslmode=disable` as a request to turn TLS off.
	const sslmode = paramsOf(connectionString).get('sslmode')
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
	for (const name of paramsOf(connectionString).keys()) {
		if (!REPRESENTABLE_PARAMS.includes(name) && !COSMETIC_PARAMS.includes(name)) return name
	}
	return undefined
}

/**
 * Why the string cannot be saved, in the terms the reader needs. Two refusals come out of the
 * check above and they call for opposite fixes: a name Postgres does not accept at all, where
 * the parameter itself is fine and only its spelling is wrong, and a parameter this resource
 * has no field for, where respelling it changes nothing.
 */
export function connectionParamRefusal(connectionString: string): string | undefined {
	const name = unsupportedConnectionParam(connectionString)
	if (!name) return undefined
	const lower = name.toLowerCase()
	const storableWhenSpelledRight =
		REPRESENTABLE_PARAMS.includes(lower) || COSMETIC_PARAMS.includes(lower)
	return storableWhenSpelledRight
		? `Postgres does not accept ${name}: connection parameter names are case-sensitive. Write it as ${lower}.`
		: `Windmill cannot store ${name} on a Postgres resource, and ignoring it would connect differently from what this string asks for. Remove it, or set the connection with the fields.`
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
