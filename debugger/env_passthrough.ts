/**
 * Container network configuration forwarded to a debug session, matching what a worker gives a
 * job's script. The session environment is built from an allowlist rather than inherited, so an
 * outbound proxy or a private CA is unreachable from a session unless these are passed
 * explicitly. Registry settings are deliberately absent: they carry credentials and are consumed
 * by the service itself (see PythonDebugSession.prepareDependencies).
 *
 * Lives in its own module because both session kinds build their own environment, and
 * dap_debug_service.ts already imports from dap_websocket_server_bun.ts.
 */
export const SESSION_ENV_VARS = [
	'HTTP_PROXY',
	'HTTPS_PROXY',
	'NO_PROXY',
	// The lowercase spellings take precedence in the worker, so forward both.
	'http_proxy',
	'https_proxy',
	'no_proxy',
	// Trust roots for a TLS-intercepting proxy. Installing the CA in the container's system
	// store is not enough on its own: requests carries its own bundle and Node reads only
	// NODE_EXTRA_CA_CERTS, so a debugged script's own HTTPS calls fail without these.
	'SSL_CERT_FILE',
	'SSL_CERT_DIR',
	'REQUESTS_CA_BUNDLE',
	'CURL_CA_BUNDLE',
	'NODE_EXTRA_CA_CERTS'
]

export function sessionEnv(): Record<string, string> {
	const env: Record<string, string> = {}
	for (const key of SESSION_ENV_VARS) {
		const value = process.env[key]
		if (value) {
			env[key] = value
		}
	}
	// A proxy without a bypass list would send the script's calls to BASE_INTERNAL_URL through it;
	// the worker defaults the same way (PROXY_ENVS in windmill-worker).
	if (!env.NO_PROXY && !env.no_proxy && (env.HTTP_PROXY || env.http_proxy || env.HTTPS_PROXY || env.https_proxy)) {
		env.NO_PROXY = 'localhost,127.0.0.1'
	}
	return env
}
