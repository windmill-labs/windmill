import { GitSyncService, type GetGlobalConnectedRepositoriesResponse } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { base } from '$lib/base'

export interface GitHubAppState {
	loadingGithubInstallations: boolean
	githubInstallations: GetGlobalConnectedRepositoriesResponse
	workspaceGithubInstallations: GetGlobalConnectedRepositoriesResponse
	selectedGHAppAccountId: string | undefined
	selectedGHAppRepository: string | undefined
	githubInstallationUrl: string | undefined
	installationCheckInterval: number | undefined
	isCheckingInstallation: boolean
	importJwt: string
}

export interface GitHubRepository {
	name: string
	url: string
}

export interface GitHubAppError extends Error {
	code: 'VALIDATION_ERROR' | 'NETWORK_ERROR' | 'AUTH_ERROR' | 'UNKNOWN_ERROR'
	details?: unknown
}

/**
 * Creates a standardized GitHub App error
 */
function createGitHubAppError(
	message: string,
	code: GitHubAppError['code'],
	details?: unknown
): GitHubAppError {
	const error = new Error(message) as GitHubAppError
	error.code = code
	error.details = details
	return error
}

/**
 * Validates JWT token format
 */
function validateJwtToken(token: string): boolean {
	if (!token || typeof token !== 'string') return false

	// Basic JWT structure validation (header.payload.signature)
	const parts = token.trim().split('.')
	if (parts.length !== 3) return false

	// Check if each part is valid base64
	try {
		parts.forEach((part) => {
			if (!part) throw new Error('Empty JWT part')
			// Add padding if needed for base64 decoding
			const padded = part + '='.repeat((4 - (part.length % 4)) % 4)
			atob(padded.replace(/-/g, '+').replace(/_/g, '/'))
		})
		return true
	} catch {
		return false
	}
}

/**
 * Handles errors consistently across GitHub App operations
 */
function handleGitHubAppError(error: unknown, operation: string): GitHubAppError {
	console.error(`GitHub App ${operation} failed:`, error)

	// Check if it's already a GitHubAppError by checking for the code property
	if (error && typeof error === 'object' && 'code' in error && 'message' in error) {
		return error as GitHubAppError
	}

	if (error instanceof Error) {
		if (error.message.includes('401') || error.message.includes('403')) {
			return createGitHubAppError(`Authentication failed during ${operation}`, 'AUTH_ERROR', error)
		}
		if (error.message.includes('network') || error.message.includes('fetch')) {
			return createGitHubAppError(`Network error during ${operation}`, 'NETWORK_ERROR', error)
		}
	}

	return createGitHubAppError(`Unknown error during ${operation}`, 'UNKNOWN_ERROR', error)
}

export function createGitHubAppState(): GitHubAppState {
	return {
		loadingGithubInstallations: false,
		githubInstallations: [],
		workspaceGithubInstallations: [],
		selectedGHAppAccountId: undefined,
		selectedGHAppRepository: undefined,
		githubInstallationUrl: undefined,
		installationCheckInterval: undefined,
		isCheckingInstallation: false,
		importJwt: ''
	}
}

/**
 * Loads GitHub installations for the current workspace
 */
export async function loadGithubInstallations(
	state: GitHubAppState,
	currentWorkspace: string
): Promise<void> {
	if (!currentWorkspace) {
		throw createGitHubAppError('Workspace is required', 'VALIDATION_ERROR')
	}

	try {
		state.loadingGithubInstallations = true

		const installations = await GitSyncService.getGlobalConnectedRepositories()
		const workspaceInstallations = installations.filter(
			(installation) => installation.workspace_id === currentWorkspace
		)

		// Update state in a way that ensures Svelte 5 reactivity
		state.githubInstallations = [...installations]
		state.workspaceGithubInstallations = [...workspaceInstallations]

		const stateParam = encodeURIComponent(
			JSON.stringify({
				workspace_id: currentWorkspace,
				base_url: window.location.origin + base
			})
		)

		state.githubInstallationUrl = `https://github.com/apps/windmill-sync-helper/installations/new?state=${stateParam}`
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'load installations')
		sendUserToast(`Failed to load GitHub installations: ${githubError.message}`, true)

		// Reset state on error
		state.githubInstallations = []
		state.workspaceGithubInstallations = []

		throw githubError
	} finally {
		state.loadingGithubInstallations = false
	}
}

/**
 * Starts polling for new GitHub installations
 */
export function startInstallationCheck(
	state: GitHubAppState,
	currentWorkspace: string,
	onInstallationFound?: () => void
): void {
	if (!currentWorkspace) {
		throw createGitHubAppError('Workspace is required', 'VALIDATION_ERROR')
	}

	// Stop any existing check first
	stopInstallationCheck(state)

	// Remember initial count to detect new installations
	const initialInstallationCount = state.githubInstallations.length

	state.isCheckingInstallation = true
	let pollCount = 0
	const maxPolls = 150 // 5 minutes (150 * 2 seconds)

	state.installationCheckInterval = window.setInterval(async () => {
		pollCount++

		// Stop polling after timeout
		if (pollCount >= maxPolls) {
			stopInstallationCheck(state)
			return
		}

		try {
			const installations = await GitSyncService.getGlobalConnectedRepositories()
			// Check if we have MORE installations than when we started
			if (installations.length > initialInstallationCount) {
				stopInstallationCheck(state)
				state.githubInstallations = [...installations]
				state.workspaceGithubInstallations = [
					...installations.filter((installation) => installation.workspace_id === currentWorkspace)
				]
				// Call callback with delay to allow popover to open
				if (onInstallationFound) {
					setTimeout(onInstallationFound, 100)
				}
			}
		} catch (error) {
			const githubError = handleGitHubAppError(error, 'check installations')
			console.error('Installation check failed:', githubError)
			// Continue polling despite errors
		}
	}, 2000)
}

export function stopInstallationCheck(state: GitHubAppState): void {
	if (state.installationCheckInterval) {
		clearInterval(state.installationCheckInterval)
		state.installationCheckInterval = undefined
	}
	state.isCheckingInstallation = false
}

/**
 * Gets repositories for a specific GitHub account
 */
export function getRepositories(state: GitHubAppState, accountId: string): GitHubRepository[] {
	if (!accountId) return []

	return (
		state.githubInstallations.find((installation) => installation.account_id === accountId)
			?.repositories || []
	)
}

/**
 * Adds a GitHub installation to the current workspace
 */
export async function addInstallationToWorkspace(
	currentWorkspace: string,
	installationId: number,
	sourceWorkspaceId: string,
	onSuccess?: () => void
): Promise<void> {
	// Input validation
	if (!currentWorkspace) {
		throw createGitHubAppError('Current workspace is required', 'VALIDATION_ERROR')
	}
	if (!installationId || installationId <= 0) {
		throw createGitHubAppError('Valid installation ID is required', 'VALIDATION_ERROR')
	}
	if (!sourceWorkspaceId) {
		throw createGitHubAppError('Source workspace ID is required', 'VALIDATION_ERROR')
	}

	try {
		await GitSyncService.installFromWorkspace({
			workspace: currentWorkspace,
			requestBody: {
				source_workspace_id: sourceWorkspaceId,
				installation_id: installationId
			}
		})
		sendUserToast('Successfully added installation to workspace', false)
		onSuccess?.()
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'add installation to workspace')
		sendUserToast(`Failed to add installation: ${githubError.message}`, true)
		throw githubError
	}
}

/**
 * Deletes a GitHub installation from the current workspace
 */
export async function deleteInstallation(
	currentWorkspace: string,
	installationId: number,
	onSuccess?: () => void
): Promise<void> {
	// Input validation
	if (!currentWorkspace) {
		throw createGitHubAppError('Workspace is required', 'VALIDATION_ERROR')
	}
	if (!installationId || installationId <= 0) {
		throw createGitHubAppError('Valid installation ID is required', 'VALIDATION_ERROR')
	}

	try {
		await GitSyncService.deleteFromWorkspace({
			workspace: currentWorkspace,
			installationId: installationId
		})
		sendUserToast('Successfully deleted installation', false)
		onSuccess?.()
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'delete installation')
		sendUserToast(`Failed to delete installation: ${githubError.message}`, true)
		throw githubError
	}
}

/**
 * Exports a GitHub installation as a JWT token
 */
export async function exportInstallation(
	currentWorkspace: string,
	installationId: number
): Promise<void> {
	// Input validation
	if (!currentWorkspace) {
		throw createGitHubAppError('Workspace is required', 'VALIDATION_ERROR')
	}
	if (!installationId || installationId <= 0) {
		throw createGitHubAppError('Valid installation ID is required', 'VALIDATION_ERROR')
	}

	try {
		const response = await GitSyncService.exportInstallation({
			workspace: currentWorkspace,
			installationId: installationId
		})

		if (!response.jwt_token) {
			throw createGitHubAppError('No JWT token received from server', 'UNKNOWN_ERROR')
		}

		const jwtToken = response.jwt_token

		// Copy to clipboard with fallback for unsecure contexts
		if (navigator.clipboard && navigator.clipboard.writeText) {
			await navigator.clipboard.writeText(jwtToken)
			sendUserToast(
				'JWT token copied to clipboard. This token is sensitive and should be kept secret!',
				false,
				undefined,
				undefined,
				10000
			)
		} else {
			// Fallback: show the token in the toast for manual copying
			sendUserToast(
				`JWT token (copy manually): ${jwtToken}`,
				false,
				[
					{
						label: 'Copy',
						callback: () => {
							// Try to copy using the older execCommand method as fallback
							const textArea = document.createElement('textarea')
							textArea.value = jwtToken
							document.body.appendChild(textArea)
							textArea.select()
							try {
								document.execCommand('copy')
								sendUserToast('JWT token copied to clipboard!', false)
							} catch (err) {
								console.error('Failed to copy to clipboard:', err)
								sendUserToast('Could not copy to clipboard. Please copy manually.', true)
							}
							document.body.removeChild(textArea)
						}
					}
				],
				undefined,
				15000
			)
		}
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'export installation')
		sendUserToast(`Failed to export installation: ${githubError.message}`, true)
		throw githubError
	}
}

/**
 * Imports a GitHub installation using a JWT token
 */
export async function importInstallation(
	currentWorkspace: string,
	jwt: string,
	onSuccess?: () => void
): Promise<void> {
	// Input validation
	if (!currentWorkspace) {
		throw createGitHubAppError('Workspace is required', 'VALIDATION_ERROR')
	}
	if (!jwt || !validateJwtToken(jwt)) {
		throw createGitHubAppError('Valid JWT token is required', 'VALIDATION_ERROR')
	}

	try {
		await GitSyncService.importInstallation({
			workspace: currentWorkspace,
			requestBody: { jwt_token: jwt.trim() }
		})
		sendUserToast('Installation imported successfully', false)
		onSuccess?.()
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'import installation')
		sendUserToast(`Failed to import installation: ${githubError.message}`, true)
		throw githubError
	}
}

/**
 * Applies the selected repository URL to the form arguments
 */
export function applyRepositoryURL(
	state: GitHubAppState,
	args: Record<string, any>,
	description: string,
	onArgsUpdate: (newArgs: Record<string, any>) => void,
	onDescriptionUpdate: (newDescription: string) => void
): void {
	if (!state.selectedGHAppRepository) {
		throw createGitHubAppError('No repository selected', 'VALIDATION_ERROR')
	}

	// Validate args object
	if (!args || typeof args !== 'object') {
		throw createGitHubAppError('Invalid arguments object', 'VALIDATION_ERROR')
	}

	const newArgs = {
		...args,
		url: state.selectedGHAppRepository,
		is_github_app: true
	}

	// Check if description already contains GitHub App text to avoid duplication
	const githubAppText = `Repository ${state.selectedGHAppRepository} with permissions fetched using Windmill Github App.`
	const existingDescription = description ?? ''

	const newDescription = existingDescription.includes(
		'with permissions fetched using Windmill Github App'
	)
		? existingDescription.replace(
				/Repository [^ ]+ with permissions fetched using Windmill Github App\. ?/,
				githubAppText + ' '
			)
		: `${githubAppText} ${existingDescription}`.trim()

	try {
		onArgsUpdate(newArgs)
		onDescriptionUpdate(newDescription)
	} catch (err) {
		const githubError = handleGitHubAppError(err, 'apply repository URL')
		throw githubError
	}
}

/**
 * Handles the install button click
 */
export function handleInstallClick(
	state: GitHubAppState,
	currentWorkspace: string,
	onInstallationFound?: () => void
): void {
	if (state.githubInstallations.length === 0) {
		if (!state.isCheckingInstallation) {
			startInstallationCheck(state, currentWorkspace, onInstallationFound)
		}
	}
}
