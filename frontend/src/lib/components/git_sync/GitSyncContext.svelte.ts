import { getContext, setContext } from 'svelte'
import { JobService, WorkspaceService, ResourceService } from '$lib/gen'
import type {
	GitRepositorySettings as BackendGitRepositorySettings,
	GitSyncObjectType
} from '$lib/gen'
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
	// Cached target branch from git resource
	_targetBranch?: string
	// Detection timestamp to avoid race conditions
	_detectionTimestamp?: number
}

export type GitSyncTestJob = {
	jobId: string
	status: 'running' | 'success' | 'failure' | undefined
	error?: string
}

export type GitSyncSettings = {
	repositories: GitSyncRepository[]
}

export type ModalState = {
	push: { idx: number; repo: GitSyncRepository; open: boolean } | null
	pull: { idx: number; repo: GitSyncRepository; open: boolean; settingsOnly?: boolean } | null
	success: { open: boolean; savedWithoutInit?: boolean } | null
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
	const activeModals = $state<ModalState>({ push: null, pull: null, success: null })

	// Legacy workspace-level settings state
	const legacyWorkspaceIncludePath = $state<string[]>([])
	const legacyWorkspaceIncludeType = $state<GitSyncObjectType[]>([])

	// Derived state for legacy detection
	const hasWorkspaceLevelSettings = $derived(
		legacyWorkspaceIncludePath.length > 0 || legacyWorkspaceIncludeType.length > 0
	)

	// Watch for changes to git repository paths and reset detection state
	$effect(() => {
		repositories.forEach((repo) => {
			if (repo.isUnsavedConnection) {
				const currentPath = repo.git_repo_resource_path

				if (repo._trackedPath && repo._trackedPath !== currentPath) {
					// Clear cached branch when resource path changes
					repo._targetBranch = undefined

					// Reset detection state for any non-idle state (including errors)
					if (repo.detectionState && repo.detectionState !== 'idle') {
						// Cancel any running detection job by resetting immediately
						_resetRepoDetectionState(repo)
					}
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
		const individualChanges = validationStates.some((v) => v.hasChanges)

		// Check if any legacy repos were imported
		const anyLegacyImported = repositories.some((r) => r.legacyImported)

		// Check if the set of repositories has changed (added/removed repos)
		const repositorySetChanged = (() => {
			if (loading) {
				return false
			}

			if (!initialRepositories || initialRepositories.length === 0) {
				return repositories.filter((_, i) => validationStates[i]?.isValid).length > 0
			}

			const initialValidPaths = new Set(
				initialRepositories
					.filter((r) => r.git_repo_resource_path && r.git_repo_resource_path.trim() !== '')
					.map((r) => r.git_repo_resource_path)
			)
			const currentValidPaths = new Set(
				repositories
					.filter((_, i) => validationStates[i]?.isValid)
					.map((r) => r.git_repo_resource_path)
			)

			// Check if sets are different (repos added or removed)
			return (
				initialValidPaths.size !== currentValidPaths.size ||
				[...initialValidPaths].some((path) => !currentValidPaths.has(path)) ||
				[...currentValidPaths].some((path) => !initialValidPaths.has(path))
			)
		})()

		return individualChanges || anyLegacyImported || repositorySetChanged
	}

	const getAllRepositoriesValid = () => getValidationStates().every((v) => v.isValid)
	const getHasUnsavedConnections = () => repositories.some((repo) => repo.isUnsavedConnection)

	function validateRepository(repo: GitSyncRepository, idx: number): boolean {
		if (!repo.git_repo_resource_path) return false
		return !checkDuplicate(repo, idx)
	}

	function checkDuplicate(repo: GitSyncRepository, idx: number): boolean {
		if (!repo.git_repo_resource_path) return false
		const firstIdx = repositories.findIndex(
			(r) => r.git_repo_resource_path === repo.git_repo_resource_path
		)
		return firstIdx !== -1 && firstIdx < idx
	}

	function checkChanges(repo: GitSyncRepository, idx: number): boolean {
		const initial = initialRepositories[idx]
		if (!initial) return true

		// Legacy repositories always have "changes" because they need migration
		if (repo.legacyImported) return true

		return (
			JSON.stringify(serializeRepository(repo)) !== JSON.stringify(serializeRepository(initial))
		)
	}

	function serializeRepository(repo: GitSyncRepository) {
		return {
			git_repo_resource_path: repo.git_repo_resource_path,
			script_path: repo.script_path,
			use_individual_branch: repo.use_individual_branch,
			group_by_folder: repo.group_by_folder,
			force_branch: repo.force_branch,
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
			(initialRepo) => initialRepo.git_repo_resource_path === repo.git_repo_resource_path
		)

		// Only call backend API if repository exists in the saved state
		if (existsInInitialState) {
			await WorkspaceService.deleteGitSyncRepository({
				workspace,
				requestBody: {
					git_repo_resource_path: `$res:${repo.git_repo_resource_path}`
				}
			})

			// Update initial state to remove the deleted repository
			const initialIdx = initialRepositories.findIndex(
				(initialRepo) => initialRepo.git_repo_resource_path === repo.git_repo_resource_path
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

	function showPullModal(idx: number, settingsOnly = false) {
		const repo = repositories[idx]
		if (repo) {
			activeModals.pull = { idx, repo, open: true, settingsOnly }
		}
	}

	function closeModal(type: 'push' | 'pull' | 'success') {
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

	function showSuccessModal(savedWithoutInit?: boolean) {
		activeModals.success = { open: true, savedWithoutInit }
	}

	function closeSuccessModal() {
		closeModal('success')
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

		// Track the detection timestamp to avoid race conditions from old jobs
		const detectionTimestamp = Date.now()

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
					settings_json: JSON.stringify(repo.settings),
					use_promotion_overrides: repo.use_individual_branch
				},
				skipPreprocessor: true
			})

			repo.detectionJobId = jobId
			repo.detectionJobStatus = 'running'
			repo._detectionTimestamp = detectionTimestamp

			// Use JobManager for polling - result will be the actual job response
			await jobManager.runWithProgress(() => Promise.resolve(jobId), {
				workspace,
				timeout: 60000,
				timeoutMessage: 'Detection job timed out after 60s',
				onProgress: (status) => {
					// Only update state if this detection is still current
					if (repo._detectionTimestamp !== detectionTimestamp) {
						return
					}

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
								repo.settings = {
									...response.local,
									exclude_path: response.local.exclude_path || [],
									extra_include_path: response.local.extra_include_path || []
								}
							}
						}
					} else if (status.status === 'failure') {
						repo.detectionState = 'error'
						repo.detectionError = status.error || 'Detection failed'
					}
				}
			})
		} catch (error: any) {
			// Only set error if this detection is still current
			if (repo._detectionTimestamp !== detectionTimestamp) {
				return
			}

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

			if (settings.git_sync !== undefined && settings.git_sync !== null) {
				// Detect workspace-level legacy settings (outside repositories)
				const workspaceLegacyIncludePath: string[] = (settings.git_sync as any)?.include_path ?? []
				const workspaceLegacyIncludeTypeRaw: GitSyncObjectType[] =
					(settings.git_sync as any)?.include_type ?? []
				const workspaceLegacyIncludeType: GitSyncObjectType[] = [...workspaceLegacyIncludeTypeRaw]

				// Update legacy workspace state
				legacyWorkspaceIncludePath.splice(
					0,
					legacyWorkspaceIncludePath.length,
					...workspaceLegacyIncludePath
				)
				legacyWorkspaceIncludeType.splice(
					0,
					legacyWorkspaceIncludeType.length,
					...workspaceLegacyIncludeType
				)

				if (settings.git_sync.repositories) {
					repositories.splice(
						0,
						repositories.length,
						...settings.git_sync.repositories.map((repo) => {
							// Check if this is a legacy repo (no nested settings object)
							const isRepoLegacy = !repo.settings
							const repoExcludeTypesOverride = repo.exclude_types_override ?? []

							// Determine default types - use workspace legacy or fallback
							const defaultTypes: GitSyncObjectType[] =
								workspaceLegacyIncludeType.length > 0
									? [...workspaceLegacyIncludeType]
									: ['script', 'flow', 'app', 'folder']

							let repoSettings: SettingsObject
							if (isRepoLegacy) {
								// Legacy repo: inherit from workspace-level settings and apply exclude_types_override
								const inheritedIncludeType = repo.settings?.include_type ?? [...defaultTypes]
								const effectiveIncludeType =
									repoExcludeTypesOverride.length > 0
										? inheritedIncludeType.filter(
												(type) => !repoExcludeTypesOverride.includes(type)
											)
										: inheritedIncludeType

								repoSettings = {
									include_path: repo.settings?.include_path ?? [...workspaceLegacyIncludePath],
									exclude_path: repo.settings?.exclude_path ?? [],
									extra_include_path: repo.settings?.extra_include_path ?? [],
									include_type: effectiveIncludeType
								}
							} else {
								// New format: use repo's own settings
								repoSettings = {
									include_path: repo.settings?.include_path ?? ['f/**'],
									exclude_path: repo.settings?.exclude_path ?? [],
									extra_include_path: repo.settings?.extra_include_path ?? [],
									include_type: repo.settings?.include_type ?? ['script', 'flow', 'app']
								}
							}

							return {
								...repo,
								git_repo_resource_path: repo.git_repo_resource_path.replace('$res:', ''),
								settings: repoSettings,
								exclude_types_override: repoExcludeTypesOverride,
								// Mark legacy repos for UI handling
								legacyImported: isRepoLegacy
							}
						})
					)
				}
			}

			// Store initial state for change tracking
			initialRepositories.splice(
				0,
				initialRepositories.length,
				...repositories.map((repo) => ({ ...repo }))
			)
		} finally {
			loading = false
		}
	}

	// Migration utility for legacy repositories
	function migrateLegacyRepository(repo: GitSyncRepository): GitSyncRepository {
		if (!repo.legacyImported) {
			return repo // Already migrated or not legacy
		}

		// Create migrated repository - exclude_types_override should already be applied in settings.include_type
		// from the loadSettings logic, so we just need to clear the override and mark as migrated
		return {
			...repo,
			exclude_types_override: [], // Clear the override since it's now integrated into include_type
			legacyImported: false // Mark as migrated
		}
	}

	async function saveRepository(idx: number, savedWithoutInit = false) {
		const repo = repositories[idx]
		if (!repo || !validateRepository(repo, idx)) {
			throw new Error('Cannot save invalid repository')
		}

		// Migrate legacy repository if needed
		const repoToSave = repo.legacyImported ? migrateLegacyRepository(repo) : repo

		// Use the new individual repository API instead of saving all repositories
		await WorkspaceService.editGitSyncRepository({
			workspace,
			requestBody: {
				git_repo_resource_path: `$res:${repoToSave.git_repo_resource_path}`,
				repository: {
					git_repo_resource_path: `$res:${repoToSave.git_repo_resource_path}`,
					script_path: repoToSave.script_path,
					use_individual_branch: repoToSave.use_individual_branch,
					group_by_folder: repoToSave.group_by_folder,
					force_branch: repoToSave.force_branch,
					settings: repoToSave.settings,
					exclude_types_override: repoToSave.exclude_types_override
				}
			}
		})

		// Update local state with migrated repository
		repositories[idx] = repoToSave
		initialRepositories[idx] = { ...repoToSave }

		// Update local state
		if (repoToSave.isUnsavedConnection) {
			repoToSave.isUnsavedConnection = false
			repoToSave.detectionState = undefined
			repoToSave.extractedSettings = undefined
			// Show success modal for new connections
			showSuccessModal(savedWithoutInit)
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
					repo_url_resource_path: repo.git_repo_resource_path,
					init: repo.isUnsavedConnection || false
				},
				skipPreprocessor: true
			})

			gitSyncTestJobs[idx] = {
				jobId: jobId,
				status: 'running'
			}

			// Use JobManager for polling
			await jobManager.runWithProgress(() => Promise.resolve(jobId), {
				workspace,
				timeout: 5000,
				timeoutMessage: 'Git sync test job timed out after 5s',
				onProgress: (status) => {
					gitSyncTestJobs[idx].status =
						status.status === 'success'
							? 'success'
							: status.status === 'failure'
								? 'failure'
								: 'running'
					if (status.status === 'failure') {
						gitSyncTestJobs[idx].error = status.error
					}
				}
			})
		} catch (error: any) {
			// Initialize the job entry if it doesn't exist (e.g., job creation failed)
			const errorMessage =
				(typeof error?.body === 'string' ? error.body : error?.body?.message) ||
				error?.message ||
				error?.toString() ||
				'Failed to run test job'
			if (!gitSyncTestJobs[idx]) {
				gitSyncTestJobs[idx] = {
					jobId: '',
					status: 'failure',
					error: errorMessage
				}
			} else {
				gitSyncTestJobs[idx].status = 'failure'
				gitSyncTestJobs[idx].error = errorMessage
			}
		}
	}

	function getPrimarySyncRepository(): { repo: GitSyncRepository; idx: number } | null {
		const idx = repositories.findIndex((r) => !r.use_individual_branch)
		return idx !== -1 ? { repo: repositories[idx], idx } : null
	}

	function getPrimaryPromotionRepository(): { repo: GitSyncRepository; idx: number } | null {
		const idx = repositories.findIndex((r) => r.use_individual_branch)
		return idx !== -1 ? { repo: repositories[idx], idx } : null
	}

	function getSecondarySyncRepositories(): { repo: GitSyncRepository; idx: number }[] {
		const result: { repo: GitSyncRepository; idx: number }[] = []
		let foundFirst = false
		repositories.forEach((repo, idx) => {
			if (!repo.use_individual_branch) {
				if (foundFirst) {
					result.push({ repo, idx })
				} else {
					foundFirst = true
				}
			}
		})
		return result
	}

	function getSecondaryPromotionRepositories(): { repo: GitSyncRepository; idx: number }[] {
		const result: { repo: GitSyncRepository; idx: number }[] = []
		let foundFirst = false
		repositories.forEach((repo, idx) => {
			if (repo.use_individual_branch) {
				if (foundFirst) {
					result.push({ repo, idx })
				} else {
					foundFirst = true
				}
			}
		})
		return result
	}

	async function removeRepositoryByPath(resourcePath: string) {
		const idx = repositories.findIndex((r) => r.git_repo_resource_path === resourcePath)
		if (idx !== -1) {
			await removeRepository(idx)
		}
	}

	function addSyncRepository() {
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

	function addPromotionRepository() {
		repositories.push({
			git_repo_resource_path: '',
			script_path: hubPaths.gitSync,
			use_individual_branch: true,
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

	// Helper to get target branch from git resource
	async function getTargetBranch(repo: GitSyncRepository): Promise<string | undefined> {
		if (!repo.git_repo_resource_path) {
			return undefined
		}

		if (repo._targetBranch) {
			if (repo._targetBranch === '') return undefined
			return repo._targetBranch
		}

		try {
			const resource = await ResourceService.getResource({
				workspace,
				path: repo.git_repo_resource_path
			})

			// Extract branch from git resource value
			const resourceValue = resource.value as any
			const targetBranch = resourceValue?.branch

			// Cache the result
			repo._targetBranch = targetBranch
			if (targetBranch === '') return undefined
			return targetBranch
		} catch (error) {
			console.warn('Failed to fetch git resource for branch info:', error)
			return undefined
		}
	}

	// Return context object
	return {
		// State (read-only access)
		get repositories() {
			return repositories
		},
		get loading() {
			return loading
		},
		get activeModals() {
			return activeModals
		},
		get gitSyncTestJobs() {
			return gitSyncTestJobs
		},
		get initialRepositories() {
			return initialRepositories
		},
		get legacyWorkspaceIncludePath() {
			return legacyWorkspaceIncludePath
		},
		get legacyWorkspaceIncludeType() {
			return legacyWorkspaceIncludeType
		},

		// Computed states - use getter functions that compute on access
		get validationStates() {
			return getValidationStates()
		},
		get hasAnyChanges() {
			return getHasAnyChanges()
		},
		get allRepositoriesValid() {
			return getAllRepositoriesValid()
		},
		get hasUnsavedConnections() {
			return getHasUnsavedConnections()
		},
		get hasWorkspaceLevelSettings() {
			return hasWorkspaceLevelSettings
		},

		// Methods
		addRepository,
		addSyncRepository,
		addPromotionRepository,
		removeRepository,
		removeRepositoryByPath,
		getRepository,
		getValidation,
		revertRepository,
		runTestJob,
		resetDetectionState,
		detectRepository,
		migrateLegacyRepository,
		showPushModal,
		showPullModal,
		closePushModal,
		closePullModal,
		showSuccessModal,
		closeSuccessModal,
		loadSettings,
		saveRepository,

		// Repository categorization methods
		getPrimarySyncRepository,
		getPrimaryPromotionRepository,
		getSecondarySyncRepositories,
		getSecondaryPromotionRepositories,

		// Helper methods
		getTargetBranch
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
