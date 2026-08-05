/**
 * Container-level network configuration forwarded to the processes the debugger spawns.
 *
 * Debug subprocesses get an allowlisted environment rather than the service's own, but nothing
 * reaches the network behind a corporate proxy, a MITM CA or a private package index unless these
 * are handed down, and the service environment is the only channel a deployment can set them
 * through.
 *
 * The split mirrors what the worker gives regular jobs: proxy settings and TLS trust roots reach
 * the debugged script, package-index settings do not. Index URLs routinely embed registry
 * credentials, and debug sessions run user-supplied code, so those are routed to the dependency
 * installer alone.
 */
export const RUNTIME_ENV_VARS = [
	// Proxy
	'HTTP_PROXY',
	'HTTPS_PROXY',
	'NO_PROXY',
	'http_proxy',
	'https_proxy',
	'no_proxy',
	// TLS trust roots (custom/MITM CAs)
	'SSL_CERT_FILE',
	'SSL_CERT_DIR',
	'REQUESTS_CA_BUNDLE',
	'CURL_CA_BUNDLE',
	'NODE_EXTRA_CA_CERTS'
]

/** Read by `windmill prepare-deps` only, and withheld from the debugged script. */
export const INSTALLER_ENV_VARS = [
	'PIP_INDEX_URL',
	'PY_INDEX_URL',
	'PIP_EXTRA_INDEX_URL',
	'PY_EXTRA_INDEX_URL',
	'PIP_INDEX_CERT',
	'PY_INDEX_CERT',
	'PIP_TRUSTED_HOST',
	'PY_TRUSTED_HOST',
	'UV_NATIVE_TLS',
	'PY_NATIVE_CERT',
	'UV_HTTP_TIMEOUT'
]

/**
 * Prefix under which installer variables are smuggled past a debug runtime that shares its
 * environment with the installer it spawns (the Python DAP server runs user code in-process).
 * That server strips the prefix into the `prepare-deps` environment and drops it from its own,
 * so the values never appear in the debugged script's `os.environ`.
 */
export const INSTALLER_ENV_PREFIX = 'WM_DAP_INSTALLER_'

function collect(keys: string[], keyPrefix = ''): Record<string, string> {
	const env: Record<string, string> = {}
	for (const key of keys) {
		const value = process.env[key]
		// An unset var and an empty one are equivalent here, and forwarding "" would shadow a
		// default the child would otherwise pick up.
		if (value !== undefined && value !== '') {
			env[keyPrefix + key] = value
		}
	}
	return env
}

/** Safe to expose to debugged user code. */
export function runtimeEnv(): Record<string, string> {
	return collect(RUNTIME_ENV_VARS)
}

/** Installer settings under their real names, for a process that does not run user code. */
export function installerEnv(): Record<string, string> {
	return collect(INSTALLER_ENV_VARS)
}

/** Installer settings under {@link INSTALLER_ENV_PREFIX}, for a runtime that does. */
export function prefixedInstallerEnv(): Record<string, string> {
	return collect(INSTALLER_ENV_VARS, INSTALLER_ENV_PREFIX)
}
