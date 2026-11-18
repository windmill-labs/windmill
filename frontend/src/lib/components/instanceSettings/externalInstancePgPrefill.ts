import { writable } from 'svelte/store'

/** The admin login fields of the `external_instance_pg` setting that a Postgres resource can fill. */
export type ExternalInstancePgPrefill = {
	/** The resource it was read from, so the form can say where the values came from. */
	source: string
	host?: string
	port?: number
	user?: string
	password?: string
	dbname?: string
	sslmode?: string
	root_certificate_pem?: string
}

const SSL_MODES = ['verify-full', 'verify-ca', 'require', 'prefer', 'disable']

/**
 * A connection handed from the data table wizard to the external cluster form it opens. A store,
 * not a prop: the form lives in the instance settings drawer, which stays mounted between visits.
 */
export const externalInstancePgPrefill = writable<ExternalInstancePgPrefill | undefined>(undefined)

/** Reads the fields the setting shares with a `postgresql` resource value; the rest is dropped. */
export function prefillFromResourceValue(
	source: string,
	value: Record<string, any>
): ExternalInstancePgPrefill {
	const port = Number(value.port)
	return {
		source,
		host: value.host || undefined,
		port: Number.isInteger(port) && port > 0 ? port : undefined,
		user: value.user || undefined,
		password: typeof value.password === 'string' ? value.password : undefined,
		dbname: value.dbname || undefined,
		// A mode the setting does not offer (`allow`) is left for the form's own default.
		sslmode: SSL_MODES.includes(value.sslmode) ? value.sslmode : undefined,
		root_certificate_pem: value.root_certificate_pem || undefined
	}
}
