/**
 * Parsing for `postgres://user:password@host:5432/dbname?sslmode=require`.
 *
 * Shared by the resource form and the data table wizard: both turn a pasted
 * connection string into a `postgresql` resource value, and the two drifting
 * apart would mean the same string produced two different resources.
 */

const CONNECTION_STRING =
	/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?(?:\?.*sslmode=(?<sslmode>[^&]+))?/

export type PostgresConnectionParts = {
	user: string
	password?: string
	host: string
	port?: number
	dbname?: string
	sslmode?: string
}

/** Undefined when the string is not a postgres URI. */
export function parsePostgresConnectionString(
	connectionString: string
): PostgresConnectionParts | undefined {
	const match = connectionString.match(CONNECTION_STRING)
	if (!match?.groups) return undefined
	const { user, password, host, port, dbname, sslmode } = match.groups
	return {
		user,
		password: password || undefined,
		host,
		port: port ? Number(port) : undefined,
		dbname: dbname || undefined,
		sslmode: sslmode || undefined
	}
}
