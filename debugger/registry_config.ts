/**
 * Dependency-registry settings for a debug session's install.
 *
 * `windmill prepare-deps` installs a session's imports with no database connection, so the
 * instance settings that point at a private npm or pip registry cannot be read there. They
 * are fetched here instead, from the backend that signed the session's launch token, and
 * passed down to the CLI over its stdin request.
 *
 * They stop at the installer. A registry URL usually embeds credentials and a debugged
 * script can read whatever the process running it holds, so none of these values are ever
 * put in a session's environment (see README.md, "Registry configuration").
 */

export interface RegistryConfig {
	npm_config_registry?: string
	npmrc?: string
	bunfig_install_scopes?: string
	pip_index_url?: string
	pip_extra_index_url?: string
	uv_index_strategy?: string
	/** Why the instance's settings are not in this response, for the user to see. */
	message?: string
}

const WINDMILL_BASE_URL = process.env.WINDMILL_BASE_URL || process.env.BASE_INTERNAL_URL

/**
 * Bounds how long a launch waits on the backend. The session can still start without the
 * settings, it just installs from the public registries, so an unreachable backend must
 * not hold it up for longer than the install itself would take.
 */
const FETCH_TIMEOUT_MS = Number(process.env.DAP_REGISTRY_CONFIG_TIMEOUT_MS) || 10_000

/**
 * Fetch the registry settings for a session, authorized by its launch token.
 *
 * Never throws and never blocks a launch: on any failure it returns a config carrying only
 * a `message`, so the session starts against the public registries and the user is told why
 * instead of being left with an unexplained "package not found".
 */
export async function fetchRegistryConfig(
	token: string | undefined,
	logger: { info: (...args: unknown[]) => void; warn: (...args: unknown[]) => void }
): Promise<RegistryConfig> {
	if (!token || !WINDMILL_BASE_URL) {
		return {}
	}

	const url = `${WINDMILL_BASE_URL.replace(/\/$/, '')}/api/debug/registry_config`
	try {
		const response = await fetch(url, {
			headers: { authorization: `Bearer ${token}` },
			signal: AbortSignal.timeout(FETCH_TIMEOUT_MS)
		})
		if (response.status === 401 || response.status === 403 || response.status === 404) {
			// Expected answers, not something the user can act on: a session that may not read
			// the settings (an operator's) is refused, and a backend older than this image has
			// no such route at all. Both install from the public registries.
			logger.info(`Registry configuration not served for this session (${response.status})`)
			return {}
		}
		if (!response.ok) {
			const detail = (await response.text().catch(() => '')).trim()
			return {
				message: `Could not read the registry configuration (${response.status}): ${detail || response.statusText}`
			}
		}

		const config: RegistryConfig = await response.json()
		// The values carry registry credentials, so only their names are logged.
		const configured = Object.entries(config)
			.filter(([key, value]) => key !== 'message' && value)
			.map(([key]) => key)
		logger.info(
			configured.length > 0
				? `Registry configuration from instance settings: ${configured.join(', ')}`
				: 'No registry configuration set on the instance'
		)
		return config
	} catch (error) {
		logger.warn(`Failed to fetch registry configuration: ${error}`)
		return { message: `Could not read the registry configuration: ${error}` }
	}
}
