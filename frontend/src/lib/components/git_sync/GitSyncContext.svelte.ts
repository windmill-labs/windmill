import { getContext, setContext } from 'svelte'
import { JobService, WorkspaceService } from '$lib/gen'
import type { GitRepositorySettings as BackendGitRepositorySettings, GitSyncObjectType } from '$lib/gen'
import { jobManager } from '$lib/services/JobManager'
import hubPaths from '$lib/hubPaths.json'
import type { SettingsObject } from '$lib/git-sync'

export type GitSyncRepository = BackendGitRepositorySettings & {
	settings: SettingsObject
	exclude_types_override: GitSyncObjectType[]
	legacyImported?: boolean
	isUnsavedConnection?: boolean
	collapsed?: boolean
	// Repository detection state for new connections
	detectionState?: 'idle' | 'loading' | 'no-wmill' | 'has-wmill' | 'error'
	extractedSettings?: SettingsObject
	detectionError?: string
	// Job tracking for detection
	detectionJobId?: string
	detectionJobStatus?: 'running' | 'success' | 'failure'
	// Internal tracking for resource path changes
	_trackedPath?: string
}

export type GitSyncTestJob = {
	jobId: string
	status: 'running' | 'success' | 'failure' | undefined
}

export type GitSyncSettings = {
	repositories: GitSyncRepository[]
}

export type ModalState = {
	push: { idx: number, repo: GitSyncRepository, open: boolean } | null
	pull: { idx: number, repo: GitSyncRepository, open: boolean } | null
}

export type ValidationState = {
	isValid: boolean
	isDuplicate: boolean
	hasChanges: boolean
}

const GIT_SYNC_CONTEXT_KEY = Symbol('git-sync-context')

// Context implementation
export function createGitSyncContext(workspace: string) {
	const repositories = $state<GitSyncRepository[]>([])
	const initialRepositories = $state<GitSyncRepository[]>([])
	const gitSyncTestJobs = $state<GitSyncTestJob[]>([])
	let loading = $state(false)
	const activeModals = $state<ModalState>({ push: null, pull: null })

	// Watch for changes to git repository paths and reset detection state
	$effect(() => {
		repositories.forEach((repo) => {
			if (repo.isUnsavedConnection) {
				const currentPath = repo.git_repo_resource_path

				if (repo._trackedPath && repo._trackedPath !== currentPath && repo.detectionState && repo.detectionState !== 'idle') {
					_resetRepoDetectionState(repo)
				}

				repo._trackedPath = currentPath
			}
		})
	})

	const getValidationStates = () => {
		return repositories.map((repo, idx) => ({
			isValid: validateRepository(repo, idx),
			isDuplicate: checkDuplicate(repo, idx),
			hasChanges: checkChanges(repo, idx)
		}))
	}

	const getHasAnyChanges = () => {
		const validationStates = getValidationStates()

		// Check if any individual repositories have changes
		const individualChanges = validationStates.some(v => v.hasChanges)

		// Check if any legacy repos were imported
		const anyLegacyImported = repositories.some(r => r.legacyImported)

		// Check if the set of repositories has changed (added/removed repos)
		const repositorySetChanged = (() => {
			if (loading) {
				return false
			}

			if (!initialRepositories || initialRepositories.length === 0) {
				return repositories.filter((_,i) => validationStates[i]?.isValid).length > 0
			}

			const initialValidPaths = new Set(
				initialRepositories
					.filter(r => r.git_repo_resource_path && r.git_repo_resource_path.trim() !== '')
					.map(r => r.git_repo_resource_path)
			)
			const currentValidPaths = new Set(
				repositories
					.filter((_,i) => validationStates[i]?.isValid)
					.map(r => r.git_repo_resource_path)
			)

			// Check if sets are different (repos added or removed)
			return initialValidPaths.size !== currentValidPaths.size ||
				   [...initialValidPaths].some(path => !currentValidPaths.has(path)) ||
				   [...currentValidPaths].some(path => !initialValidPaths.has(path))
		})()

		return individualChanges || anyLegacyImported || repositorySetChanged
	}

	const getAllRepositoriesValid = () => getValidationStates().every(v => v.isValid)
	const getHasUnsavedConnections = () => repositories.some(repo => repo.isUnsavedConnection)

	function validateRepository(repo: GitSyncRepository, idx: number): boolean {
		if (!repo.git_repo_resource_path) return false
		return !checkDuplicate(repo, idx)
	}

	function checkDuplicate(repo: GitSyncRepository, idx: number): boolean {
		if (!repo.git_repo_resource_path) return false
		const firstIdx = repositories.findIndex(r => r.git_repo_resource_path === repo.git_repo_resource_path)
		return firstIdx !== -1 && firstIdx < idx
	}

	function checkChanges(repo: GitSyncRepository, idx: number): boolean {
		const initial = initialRepositories[idx]
		if (!initial) return true
		return JSON.stringify(serializeRepository(repo)) !== JSON.stringify(serializeRepository(initial))
	}

	function serializeRepository(repo: GitSyncRepository) {
		return {
			git_repo_resource_path: repo.git_repo_resource_path,
			script_path: repo.script_path,
			use_individual_branch: repo.use_individual_branch,
			group_by_folder: repo.group_by_folder,
			settings: repo.settings,
			exclude_types_override: repo.exclude_types_override
		}
	}

	function addRepository() {
		repositories.push({
			git_repo_resource_path: '',
			script_path: hubPaths.gitSync,
			use_individual_branch: false,
			group_by_folder: false,
			settings: {
				include_path: ['f/**'],
				exclude_path: [],
				extra_include_path: [],
				include_type: ['script', 'flow', 'app', 'folder']
			},
			exclude_types_override: [],
			legacyImported: false,
			isUnsavedConnection: true,
			collapsed: false
		})
		gitSyncTestJobs.push({
			jobId: '',
			status: undefined
		})
	}

	async function removeRepository(idx: number) {
		const repo = repositories[idx]
		if (!repo) return

		// Check if this repository exists in the initial (saved) state
		const existsInInitialState = initialRepositories.some(
			initialRepo => initialRepo.git_repo_resource_path === repo.git_repo_resource_path
		)

		// Only call backend API if repository exists in the saved state
		if (existsInInitialState && repo.git_repo_resource_path) {
			await WorkspaceService.deleteGitSyncRepository({
				workspace,
				requestBody: {
					git_repo_resource_path: `$res:${repo.git_repo_resource_path}`
				}
			})

			// Update initial state to remove the deleted repository
			const initialIdx = initialRepositories.findIndex(
				initialRepo => initialRepo.git_repo_resource_path === repo.git_repo_resource_path
			)
			if (initialIdx !== -1) {
				initialRepositories.splice(initialIdx, 1)
			}
		}

		// Remove from local state
		repositories.splice(idx, 1)
		gitSyncTestJobs.splice(idx, 1)
	}

	function getRepository(idx: number) {
		return repositories[idx]
	}

	function showPushModal(idx: number) {
		const repo = repositories[idx]
		if (repo) {
			activeModals.push = { idx, repo, open: true }
		}
	}

	function showPullModal(idx: number) {
		const repo = repositories[idx]
		if (repo) {
			activeModals.pull = { idx, repo, open: true }
		}
	}

	function closeModal(type: 'push' | 'pull') {
		if (activeModals[type]) {
			activeModals[type]!.open = false
		}
		setTimeout(() => {
			activeModals[type] = null
		}, 200)
	}

	function closePushModal() {
		closeModal('push')
	}

	function closePullModal() {
		closeModal('pull')
	}

	function getValidation(idx: number): ValidationState {
		const states = getValidationStates()
		return states[idx] || { isValid: false, isDuplicate: false, hasChanges: false }
	}

	async function detectRepository(idx: number) {
		const repo = repositories[idx]
		if (!repo || !repo.git_repo_resource_path) {
			throw new Error('Repository not found or no resource path')
		}

		repo.detectionState = 'loading'
		repo.detectionError = undefined
		repo.detectionJobId = undefined
		repo.detectionJobStatus = undefined

		try {
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: {
					workspace_id: workspace,
					repo_url_resource_path: repo.git_repo_resource_path,
					dry_run: true,
					pull: false,
					only_wmill_yaml: true,
					settings_json: JSON.stringify(repo.settings)
				},
				skipPreprocessor: true
			})

			repo.detectionJobId = jobId
			repo.detectionJobStatus = 'running'

			// Use JobManager for polling - result will be the actual job response
			await jobManager.runWithProgress(
				() => Promise.resolve(jobId),
				{
					workspace,
					timeout: 30000,
					timeoutMessage: 'Detection job timed out after 30s',
					onProgress: (status) => {
						repo.detectionJobStatus = status.status

						// Process successful detection result
						if (status.status === 'success' && status.result) {
							const response = status.result as any
							if (response.isInitialSetup) {
								repo.detectionState = 'no-wmill'
							} else {
								repo.detectionState = 'has-wmill'
								// Apply extracted settings from the git repository
								if (response.local) {
									repo.extractedSettings = response.local
									// Auto-apply the extracted settings
									repo.settings = { ...response.local }
								}
							}
						} else if (status.status === 'failure') {
							repo.detectionState = 'error'
							repo.detectionError = status.error || 'Detection failed'
						}
					}
				}
			)
		} catch (error: any) {
			repo.detectionState = 'error'
			repo.detectionError = error?.message || error?.toString() || 'Failed to detect repository'
			repo.detectionJobStatus = 'failure'
		}
	}

	// Settings management
	async function loadSettings() {
		loading = true
		try {
			const settings = await WorkspaceService.getSettings({ workspace })

			if (settings.git_sync?.repositories) {
				repositories.splice(0, repositories.length, ...settings.git_sync.repositories.map(repo => ({
					...repo,
					git_repo_resource_path: repo.git_repo_resource_path.replace('$res:', ''),
					settings: {
						include_path: repo.settings?.include_path || ['f/**'],
						exclude_path: repo.settings?.exclude_path || [],
						extra_include_path: repo.settings?.extra_include_path || [],
						include_type: repo.settings?.include_type || ['script', 'flow', 'app']
					},
					exclude_types_override: repo.exclude_types_override || []
				})))
			}

			// Store initial state for change tracking
			initialRepositories.splice(0, initialRepositories.length, ...repositories.map(repo => ({ ...repo })))
		} finally {
			loading = false
		}
	}

	// Centralized save utility that handles the $res: prefix correctly
	async function saveGitSyncSettings(repositoriesToSave: GitSyncRepository[]) {
		const gitSyncConfig = {
			repositories: repositoriesToSave.map(repo => ({
				git_repo_resource_path: `$res:${repo.git_repo_resource_path}`,
				script_path: repo.script_path,
				use_individual_branch: repo.use_individual_branch,
				group_by_folder: repo.group_by_folder,
				settings: repo.settings,
				exclude_types_override: repo.exclude_types_override
			}))
		}

		await WorkspaceService.editWorkspaceGitSyncConfig({
			workspace,
			requestBody: { git_sync_settings: gitSyncConfig }
		})

		// Update initial state for change tracking
		initialRepositories.splice(0, initialRepositories.length, ...repositories.map(repo => ({ ...repo })))
	}

	async function saveRepository(idx: number) {
		const repo = repositories[idx]
		if (!repo || !validateRepository(repo, idx)) {
			throw new Error('Cannot save invalid repository')
		}

		// Use the new individual repository API instead of saving all repositories
		await WorkspaceService.editGitSyncRepository({
			workspace,
			requestBody: {
				git_repo_resource_path: `$res:${repo.git_repo_resource_path}`,
				repository: {
					git_repo_resource_path: `$res:${repo.git_repo_resource_path}`,
					script_path: repo.script_path,
					use_individual_branch: repo.use_individual_branch,
					group_by_folder: repo.group_by_folder,
					settings: repo.settings,
					exclude_types_override: repo.exclude_types_override
				}
			}
		})

		// Update initial state for this specific repository only
		initialRepositories[idx] = { ...repo }

		// Update local state
		if (repo.isUnsavedConnection) {
			repo.isUnsavedConnection = false
			repo.detectionState = undefined
			repo.extractedSettings = undefined
		}
	}



	// Helper functions for original functionality

	function revertRepository(idx: number) {
		const initial = initialRepositories[idx]
		if (initial) {
			repositories[idx] = JSON.parse(JSON.stringify(initial))
		}
	}



	// Reset detection state for a repository
	function resetDetectionState(idx: number) {
		const repo = repositories[idx]
		if (!repo || !repo.isUnsavedConnection) return

		_resetRepoDetectionState(repo)
	}

	// Helper function to reset detection state on a repository object
	function _resetRepoDetectionState(repo: GitSyncRepository) {
		repo.detectionState = 'idle'
		repo.extractedSettings = undefined
		repo.detectionError = undefined
		repo.detectionJobId = undefined
		repo.detectionJobStatus = undefined
	}


	async function runTestJob(idx: number) {
		const repo = repositories[idx]
		if (!repo?.git_repo_resource_path || !repo?.script_path) {
			return
		}

		try {
			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitSyncTest,
				requestBody: {
					repo_url_resource_path: repo.git_repo_resource_path
				},
				skipPreprocessor: true
			})

			gitSyncTestJobs[idx] = {
				jobId: jobId,
				status: 'running'
			}

			// Use JobManager for polling
			await jobManager.runWithProgress(
				() => Promise.resolve(jobId),
				{
					workspace,
					timeout: 5000,
					timeoutMessage: 'Git sync test job timed out after 5s',
					onProgress: (status) => {
						gitSyncTestJobs[idx].status = status.status === 'success' ? 'success' :
							status.status === 'failure' ? 'failure' : 'running'
					}
				}
			)

			// If we get here, the job completed successfully
			gitSyncTestJobs[idx].status = 'success'
		} catch (error) {
			gitSyncTestJobs[idx].status = 'failure'
		}
	}

	// Return context object
	return {
		// State (read-only access)
		get repositories() { return repositories },
		get loading() { return loading },
		get activeModals() { return activeModals },
		get gitSyncTestJobs() { return gitSyncTestJobs },
		get initialRepositories() { return initialRepositories },

		// Computed states - use getter functions that compute on access
		get validationStates() { return getValidationStates() },
		get hasAnyChanges() { return getHasAnyChanges() },
		get allRepositoriesValid() { return getAllRepositoriesValid() },
		get hasUnsavedConnections() { return getHasUnsavedConnections() },

		// Methods
		addRepository,
		removeRepository,
		getRepository,
		getValidation,
		revertRepository,
		runTestJob,
		resetDetectionState,
		detectRepository,
		showPushModal,
		showPullModal,
		closePushModal,
		closePullModal,
		loadSettings,
		saveRepository,
		saveGitSyncSettings,
	}
}

export type GitSyncContextType = ReturnType<typeof createGitSyncContext>

export function setGitSyncContext(workspace: string): GitSyncContextType {
	const context = createGitSyncContext(workspace)
	setContext(GIT_SYNC_CONTEXT_KEY, context)
	return context
}

export function getGitSyncContext(): GitSyncContextType {
	const context = getContext<GitSyncContextType>(GIT_SYNC_CONTEXT_KEY)
	if (!context) {
		throw new Error('Git sync context not found. Make sure to call setGitSyncContext first.')
	}
	return context
}
