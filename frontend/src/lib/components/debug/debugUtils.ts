/**
 * Shared debug utility functions used across ScriptEditor, FlowModuleComponent,
 * and RawAppInlineScriptEditor.
 */

import { VariableService, UserService } from '$lib/gen'

/**
 * Fetch contextual variables (WM_WORKSPACE, WM_TOKEN, etc.) for the debugger.
 * Creates a fresh short-lived token for the debug session.
 */
export async function fetchContextualVariables(
	workspace: string
): Promise<Record<string, string>> {
	if (!workspace) {
		return {}
	}

	try {
		const variables = await VariableService.listContextualVariables({ workspace })
		const envVars: Record<string, string> = {}
		for (const v of variables) {
			envVars[v.name] = v.value
		}

		// Create a fresh token with 15-minute expiration for the debugger
		try {
			const expirationDate = new Date(Date.now() + 15 * 60 * 1000)
			const freshToken = await UserService.createToken({
				requestBody: {
					label: 'debugger-token',
					expiration: expirationDate.toISOString(),
					workspace_id: workspace
				}
			})
			envVars['WM_TOKEN'] = freshToken
		} catch (tokenError) {
			console.error('Failed to create debugger token:', tokenError)
		}

		return envVars
	} catch (error) {
		console.error('Failed to fetch contextual variables:', error)
		return {}
	}
}

/**
 * Sign a debug request with the backend. This creates an audit log entry
 * and returns a signed token that authorizes the debug session.
 */
export async function signDebugRequest(
	workspace: string,
	code: string,
	language: string
): Promise<{ token: string; code: string; job_id: string }> {
	if (!workspace) {
		throw new Error('No workspace selected')
	}

	const response = await fetch(`/api/w/${workspace}/debug/sign`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ code, language })
	})

	if (!response.ok) {
		const errorText = await response.text()
		if (errorText.includes('not initialized')) {
			throw new Error(
				'Debug signing is not configured on the server. Please contact your administrator.'
			)
		}
		throw new Error(errorText || 'Failed to authorize debug session')
	}

	return await response.json()
}

/**
 * Get a user-friendly error message for debug errors
 */
export function getDebugErrorMessage(error: unknown): string {
	const message = error instanceof Error ? error.message : String(error)

	// Handle token verification errors from debugger
	if (message.includes('Token verification failed') || message.includes('Debug token required')) {
		if (message.includes('expired')) {
			return 'Debug session expired. Please try again.'
		}
		if (message.includes('Invalid JWT signature')) {
			return 'Debug authorization failed. The signing key may be misconfigured.'
		}
		if (message.includes('Code hash mismatch')) {
			return 'Code was modified after signing. Please try again.'
		}
		if (message.includes('Public key not available')) {
			return 'Debug server cannot verify tokens. Please check WINDMILL_BASE_URL configuration.'
		}
		if (message.includes('Debug token required')) {
			return 'Debug authorization required. The debug session must be signed by the backend.'
		}
		return 'Debug authorization failed. Please try again.'
	}

	// Handle connection errors
	if (message.includes('WebSocket') || message.includes('connection failed')) {
		return 'Could not connect to debug server. Make sure the DAP server is running.'
	}

	// Handle HTTP errors
	if (message.includes('401') || message.includes('Unauthorized')) {
		return 'Debug session unauthorized. Please check your permissions.'
	}
	if (message.includes('404')) {
		return 'Debug service not available. Please check if the debugger is enabled.'
	}
	if (message.includes('timeout') || message.includes('ETIMEDOUT')) {
		return 'Debug service connection timed out. Please try again.'
	}

	// Handle signing errors
	if (message.includes('not configured on the server')) {
		return message
	}

	return message || 'An unexpected error occurred while starting the debugger.'
}

/**
 * Check if a language supports debugging
 */
export function isDebuggableLanguage(language: string | undefined): boolean {
	if (!language) return false
	return ['python3', 'bun', 'typescript', 'deno', 'nativets'].includes(language)
}

/**
 * Get the file extension for a language (used for debug file path)
 */
export function getDebugFileExtension(language: string | undefined): string {
	switch (language) {
		case 'python3':
			return '.py'
		case 'bun':
		case 'typescript':
		case 'deno':
		case 'nativets':
			return '.ts'
		default:
			return '.txt'
	}
}
