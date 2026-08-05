/**
 * Container-level network configuration forwarded to every process the debugger spawns.
 *
 * Debug subprocesses get an allowlisted environment rather than the service's own, but the
 * debugged script and the `windmill prepare-deps` subprocess (which runs `uv venv` / `uv pip
 * install`) reach nothing behind a corporate proxy, a MITM CA or a private package index unless
 * these are handed down. The service environment is the only channel a deployment can configure
 * them through.
 */
export const PASSTHROUGH_ENV_VARS = [
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
	'NODE_EXTRA_CA_CERTS',
	// Python package index
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

export function passthroughEnv(): Record<string, string> {
	const env: Record<string, string> = {}
	for (const key of PASSTHROUGH_ENV_VARS) {
		const value = process.env[key]
		// An unset var and an empty one are equivalent here, and forwarding "" would shadow a
		// default the child would otherwise pick up.
		if (value !== undefined && value !== '') {
			env[key] = value
		}
	}
	return env
}
