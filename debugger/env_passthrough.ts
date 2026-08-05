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

/**
 * Read by `windmill prepare-deps` only. These must never be added to the list above: index URLs
 * routinely embed registry credentials, and a debug runtime executes user-supplied code, which can
 * read anything its process holds. `prepare-deps` is spawned from the service process, which never
 * runs user code and so already has them from the container.
 */
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

function collect(keys: string[]): Record<string, string> {
	const env: Record<string, string> = {}
	for (const key of keys) {
		const value = process.env[key]
		// An unset var and an empty one are equivalent here, and forwarding "" would shadow a
		// default the child would otherwise pick up.
		if (value !== undefined && value !== '') {
			env[key] = value
		}
	}
	return env
}

/** Safe to expose to debugged user code. */
export function runtimeEnv(): Record<string, string> {
	return collect(RUNTIME_ENV_VARS)
}

/**
 * The extra settings only the dependency installer may see. Spread over a spawn that already
 * carries {@link runtimeEnv}, and only for a process that never executes user code.
 */
export function installerEnv(): Record<string, string> {
	return collect(INSTALLER_ENV_VARS)
}
