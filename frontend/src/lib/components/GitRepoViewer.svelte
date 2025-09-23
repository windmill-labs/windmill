<script lang="ts">
	import { GitBranch, File, Folder, Loader2, AlertCircle, X, HardDrive, Database } from 'lucide-svelte'
	import GitlabIcon from './icons/GitlabIcon.svelte'
	import { Button } from './common'
	import { twMerge } from 'tailwind-merge'
	import Editor from './Editor.svelte'
	import GithubIcon from './icons/GithubIcon.svelte'
	import { gitRepoManager, type RepoInfo, type FileEntry, type CloneProgress } from '$lib/utils/gitRepoManager'
	import { onMount } from 'svelte'

	interface Props {
		class?: string | undefined
		initialUrl?: string | undefined
	}

	let { class: className = '', initialUrl = '' }: Props = $props()

	let repoUrl = $state(initialUrl)
	let loading = $state(false)
	let cloning = $state(false)
	let cloneProgress = $state<CloneProgress | null>(null)
	let error = $state<string | null>(null)
	let repoInfo = $state<RepoInfo | null>(null)
	let currentRepoId = $state<string | null>(null)
	let fileTree = $state<FileEntry[]>([])
	let selectedFile = $state<FileEntry | null>(null)
	let selectedBranch = $state('main')
	let fileContent = $state<string | null>(null)
	let expandedFolders = $state<Set<string>>(new Set())
	let cacheStats = $state<{ repoCount: number; totalSize: number; maxSize: number } | null>(null)
	let showCacheManager = $state(false)
	let storedRepos = $state<RepoInfo[]>([])

	let parsedRepo = $derived(parseGitUrl(repoUrl))
	let isValidUrl = $derived(parsedRepo.isValid)
	let repoProvider = $derived(parsedRepo.provider)

	// Initialize git repo manager on mount
	onMount(async () => {
		try {
			await gitRepoManager.init()
			await updateCacheStats()
			await loadStoredRepos()
		} catch (err: any) {
			error = `Failed to initialize: ${err.message}`
		}
	})

	// URL parsing and validation
	function parseGitUrl(url: string) {
		if (!url.trim()) return { isValid: false, provider: null, owner: '', repo: '', fullUrl: '' }

		// GitHub patterns
		const githubPatterns = [
			/github\.com[\/:]([^\/]+)\/([^\/\s.]+)(?:\/tree\/([^\/\s]+))?/,
			/github\.com\/([^\/]+)\/([^\/\s.]+)\.git/,
			/git@github\.com:([^\/]+)\/([^\/\s.]+)\.git/
		]

		for (const pattern of githubPatterns) {
			const match = url.match(pattern)
			if (match) {
				return {
					isValid: true,
					provider: 'github' as const,
					owner: match[1],
					repo: match[2].replace(/\.git$/, ''),
					branch: match[3] || 'main',
					fullUrl: url
				}
			}
		}

		// GitLab patterns
		const gitlabPatterns = [
			/gitlab\.com[\/:]([^\/]+)\/([^\/\s.]+)(?:\/-\/tree\/([^\/\s]+))?/,
			/gitlab\.com\/([^\/]+)\/([^\/\s.]+)\.git/,
			/git@gitlab\.com:([^\/]+)\/([^\/\s.]+)\.git/
		]

		for (const pattern of gitlabPatterns) {
			const match = url.match(pattern)
			if (match) {
				return {
					isValid: true,
					provider: 'gitlab' as const,
					owner: match[1],
					repo: match[2].replace(/\.git$/, ''),
					branch: match[3] || 'main',
					fullUrl: url
				}
			}
		}

		// Generic git:// protocol
		const gitMatch = url.match(/git:\/\/([^\/]+)\/(.+)/)
		if (gitMatch) {
			const host = gitMatch[1]
			const path = gitMatch[2].replace(/\.git$/, '')
			
			if (host.includes('github') || host.includes('gitlab')) {
				const pathParts = path.split('/')
				if (pathParts.length >= 2) {
					const provider = host.includes('github') ? 'github' : 'gitlab'
					return {
						isValid: true,
						provider: provider as 'github' | 'gitlab',
						owner: pathParts[0],
						repo: pathParts[1],
						branch: 'main',
						fullUrl: url,
						converted: `https://${host.replace(/^[^.]+\./, '')}/${pathParts[0]}/${pathParts[1]}`
					}
				}
			}
		}

		// If no pattern matches but URL contains github/gitlab, provide helpful error
		if (url.includes('github.com') || url.includes('gitlab.com')) {
			return {
				isValid: false,
				provider: null,
				owner: '',
				repo: '',
				branch: 'main',
				fullUrl: url,
				error: 'URL format not recognized. Please use a standard repository URL.'
			}
		}

		return { 
			isValid: false, 
			provider: null, 
			owner: '', 
			repo: '', 
			branch: 'main',
			fullUrl: url,
			error: 'Only GitHub and GitLab repositories are supported.'
		}
	}

	async function loadRepository() {
		if (!isValidUrl || !repoUrl.trim()) return

		try {
			loading = true
			cloning = true
			error = null

			// Check if repo is already cached
			const existingRepoId = await findExistingRepo(repoUrl)
			if (existingRepoId) {
				currentRepoId = existingRepoId
				repoInfo = await gitRepoManager.getRepoInfo(existingRepoId) || null
			} else {
				// Clone the repository
				currentRepoId = await gitRepoManager.cloneRepository(repoUrl, (progress) => {
					cloneProgress = progress
				})
				repoInfo = await gitRepoManager.getRepoInfo(currentRepoId) || null
			}

			if (repoInfo) {
				selectedBranch = repoInfo.defaultBranch
				await loadFileTree()
				await updateCacheStats()
				await loadStoredRepos()
			}
		} catch (err: any) {
			error = err.message || 'Failed to load repository'
			repoInfo = null
			fileTree = []
			throw err
		} finally {
			loading = false
			cloning = false
			cloneProgress = null
		}
	}

	async function findExistingRepo(url: string): Promise<string | null> {
		const repos = await gitRepoManager.getStoredRepos()
		const existing = repos.find(repo => repo.url === url)
		return existing?.id || null
	}

	async function loadFileTree(path: string = '') {
		if (!currentRepoId) return

		try {
			const files = await gitRepoManager.getFileTree(currentRepoId, path, selectedBranch)
			if (path === '') {
				fileTree = files
			}
			return files
		} catch (err: any) {
			console.error('Error loading file tree:', err)
			return []
		}
	}

	async function toggleFolder(folderPath: string) {
		if (expandedFolders.has(folderPath)) {
			expandedFolders.delete(folderPath)
			expandedFolders = new Set(expandedFolders)
		} else {
			expandedFolders.add(folderPath)
			expandedFolders = new Set(expandedFolders)

			// Load folder contents if not already loaded
			const folderItem = findItemByPath(fileTree, folderPath)
			if (folderItem && folderItem.type === 'dir') {
				const children = await loadFileTree(folderPath)
				if (children) {
					// Update the file tree with children
					updateFileTreeWithChildren(fileTree, folderPath, children)
					fileTree = [...fileTree]
				}
			}
		}
	}

	function findItemByPath(items: FileEntry[], path: string): FileEntry | null {
		for (const item of items) {
			if (item.path === path) return item
			// For nested items, we'd need to implement recursive search
			// This is simplified for the current flat structure
		}
		return null
	}

	function updateFileTreeWithChildren(items: FileEntry[], folderPath: string, children: FileEntry[]) {
		// This is a simplified implementation
		// In a full implementation, you'd update the tree structure with nested children
		const folderIndex = items.findIndex(item => item.path === folderPath)
		if (folderIndex >= 0) {
			// Insert children after the folder
			items.splice(folderIndex + 1, 0, ...children.map(child => ({
				...child,
				path: `${folderPath}/${child.name}`,
				name: `  ${child.name}` // Indent for visual hierarchy
			})))
		}
	}

	async function selectFile(file: FileEntry) {
		if (!currentRepoId || file.type !== 'file') return

		try {
			loading = true
			selectedFile = file
			fileContent = await gitRepoManager.readFile(currentRepoId, file.path, selectedBranch)
		} catch (err: any) {
			error = `Failed to read file: ${err.message}`
			fileContent = ''
		} finally {
			loading = false
		}
	}

	async function switchBranch(branch: string) {
		if (!currentRepoId) return

		try {
			loading = true
			selectedBranch = branch
			await gitRepoManager.switchBranch(currentRepoId, branch)
			await loadFileTree()
			// Clear selected file when switching branches
			selectedFile = null
			fileContent = null
		} catch (err: any) {
			error = `Failed to switch branch: ${err.message}`
		} finally {
			loading = false
		}
	}

	function cancelClone() {
		gitRepoManager.cancelCurrentClone()
		cloning = false
		cloneProgress = null
		loading = false
	}

	async function updateCacheStats() {
		cacheStats = await gitRepoManager.getCacheStats()
	}

	async function loadStoredRepos() {
		storedRepos = await gitRepoManager.getStoredRepos()
	}

	async function loadStoredRepo(repo: RepoInfo) {
		repoUrl = repo.url
		currentRepoId = repo.id
		repoInfo = repo
		selectedBranch = repo.defaultBranch
		await loadFileTree()
		showCacheManager = false
	}

	async function removeStoredRepo(repoId: string) {
		await gitRepoManager.removeRepository(repoId)
		await updateCacheStats()
		await loadStoredRepos()
		
		// Clear current repo if it was removed
		if (currentRepoId === repoId) {
			currentRepoId = null
			repoInfo = null
			fileTree = []
			selectedFile = null
			fileContent = null
		}
	}

	async function clearAllCache() {
		await gitRepoManager.clearCache()
		await updateCacheStats()
		await loadStoredRepos()
		
		// Clear current state
		currentRepoId = null
		repoInfo = null
		fileTree = []
		selectedFile = null
		fileContent = null
	}

	function getFileLanguage(filename: string): string {
		const ext = filename.split('.').pop()?.toLowerCase()
		const languageMap: Record<string, string> = {
			js: 'javascript',
			ts: 'typescript',
			py: 'python',
			java: 'java',
			go: 'go',
			rs: 'rust',
			cpp: 'cpp',
			c: 'c',
			css: 'css',
			html: 'html',
			json: 'json',
			xml: 'xml',
			yaml: 'yaml',
			yml: 'yaml',
			md: 'markdown',
			sh: 'shell',
			sql: 'sql'
		}
		return languageMap[ext || ''] || 'text'
	}

	function formatSize(bytes: number): string {
		const units = ['B', 'KB', 'MB', 'GB']
		let size = bytes
		let unitIndex = 0
		
		while (size >= 1024 && unitIndex < units.length - 1) {
			size /= 1024
			unitIndex++
		}
		
		return `${size.toFixed(1)} ${units[unitIndex]}`
	}
</script>

{#snippet treeItem(item: FileEntry, depth: number)}
	<div style="padding-left: {depth * 12}px">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="flex items-center gap-2 p-2 hover:bg-surface-hover rounded cursor-pointer text-sm"
			class:bg-surface-secondary={selectedFile?.path === item.path}
			onclick={() => (item.type === 'dir' ? toggleFolder(item.path) : selectFile(item))}
		>
			{#if item.type === 'dir'}
				<div class="flex items-center gap-1">
					{#if expandedFolders.has(item.path)}
						<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
						</svg>
					{:else}
						<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
						</svg>
					{/if}
					<Folder size={16} class="text-blue-500" />
				</div>
			{:else}
				<div style="width: 16px;"></div>
				<File size={16} class="text-gray-500" />
			{/if}
			<span class="truncate">{item.name}</span>
		</div>
	</div>
{/snippet}

<div class={twMerge('h-full flex flex-col bg-surface', className)}>
	<!-- Header with URL input -->
	<div class="p-4 border-b border-gray-200 dark:border-gray-700 bg-surface-secondary">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-lg font-semibold text-primary">Git Repository Viewer</h3>
			<div class="flex items-center gap-2">
				{#if cacheStats}
					<Button
						size="sm"
						variant="border"
						on:click={() => showCacheManager = !showCacheManager}
						btnClasses="px-3 flex items-center gap-2"
					>
						<Database size={14} />
						Cache ({cacheStats.repoCount})
					</Button>
				{/if}
			</div>
		</div>

		<div class="flex gap-2">
			<input
				type="text"
				placeholder="Enter GitHub, GitLab, or git:// URL..."
				bind:value={repoUrl}
				class="flex-1 px-3 py-2 border rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 bg-surface border-gray-300 dark:border-gray-600"
				onkeydown={(e) => e.key === 'Enter' && loadRepository()}
				disabled={cloning}
			/>
			<Button
				size="sm"
				disabled={!isValidUrl || loading || cloning}
				on:click={loadRepository}
				btnClasses="px-4"
			>
				{#if cloning}
					<Loader2 class="animate-spin" size={16} />
				{:else}
					Load
				{/if}
			</Button>
			{#if cloning}
				<Button
					size="sm"
					variant="border"
					on:click={cancelClone}
					btnClasses="px-3"
				>
					<X size={14} />
				</Button>
			{/if}
		</div>

		{#if repoUrl && !isValidUrl}
			<div class="text-red-500 text-sm mt-2 flex items-center gap-2">
				<AlertCircle size={16} />
				{parsedRepo.error || 'Invalid repository URL format'}
			</div>
		{:else if parsedRepo.converted}
			<div class="text-blue-600 dark:text-blue-400 text-sm mt-2">
				Converted git:// URL to: {parsedRepo.converted}
			</div>
		{/if}

		{#if cloneProgress}
			<div class="mt-3">
				<div class="flex items-center justify-between text-sm text-secondary mb-1">
					<span>{cloneProgress.phase}...</span>
					{#if cloneProgress.lengthComputable}
						<span>{Math.round((cloneProgress.loaded / cloneProgress.total) * 100)}%</span>
					{/if}
				</div>
				<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
					<div
						class="bg-blue-600 h-2 rounded-full transition-all duration-300"
						style="width: {cloneProgress.lengthComputable ? (cloneProgress.loaded / cloneProgress.total) * 100 : 50}%"
					></div>
				</div>
			</div>
		{/if}
	</div>

	<!-- Cache Manager Modal -->
	{#if showCacheManager}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface rounded-lg shadow-lg max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
				<div class="p-4 border-b border-gray-200 dark:border-gray-700">
					<div class="flex items-center justify-between">
						<h4 class="text-lg font-semibold text-primary">Cache Manager</h4>
						<Button size="sm" variant="border" on:click={() => showCacheManager = false}>
							<X size={16} />
						</Button>
					</div>
					{#if cacheStats}
						<p class="text-sm text-secondary mt-2">
							{cacheStats.repoCount} repositories • {formatSize(cacheStats.totalSize)} / {formatSize(cacheStats.maxSize)} used
						</p>
					{/if}
				</div>
				
				<div class="p-4">
					{#if storedRepos.length === 0}
						<p class="text-secondary text-center py-8">No cached repositories</p>
					{:else}
						<div class="space-y-2">
							{#each storedRepos as repo}
								<div class="flex items-center justify-between p-3 border border-gray-200 dark:border-gray-700 rounded">
									<div class="flex items-center gap-3">
										{#if repo.provider === 'github'}
											<GithubIcon width="20px" height="20px" />
										{:else if repo.provider === 'gitlab'}
											<GitlabIcon width="20px" height="20px" />
										{/if}
										<div>
											<p class="font-medium text-primary">{repo.owner}/{repo.repo}</p>
											<p class="text-xs text-secondary">
												{formatSize(repo.size)} • Last accessed {new Date(repo.lastAccessed).toLocaleDateString()}
											</p>
										</div>
									</div>
									<div class="flex gap-2">
										<Button size="sm" variant="border" on:click={() => loadStoredRepo(repo)}>
											Load
										</Button>
										<Button size="sm" variant="border" on:click={() => removeStoredRepo(repo.id)}>
											<X size={14} />
										</Button>
									</div>
								</div>
							{/each}
						</div>
						
						<div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
							<Button size="sm" variant="border" on:click={clearAllCache} btnClasses="w-full">
								Clear All Cache
							</Button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}

	<!-- Repository Info -->
	{#if repoInfo}
		<div class="p-4 border-b border-gray-200 dark:border-gray-700 bg-surface">
			<div class="flex items-center gap-3 mb-2">
				{#if repoProvider === 'github'}
					<GithubIcon width="20px" height="20px" />
				{:else if repoProvider === 'gitlab'}
					<GitlabIcon width="20px" height="20px" />
				{/if}
				<h4 class="font-semibold text-primary">{repoInfo.owner}/{repoInfo.repo}</h4>
				<span class="text-xs text-secondary px-2 py-1 bg-blue-100 dark:bg-blue-900/30 rounded">
					<HardDrive size={12} class="inline mr-1" />
					Cached
				</span>
			</div>

			<!-- Branch selector -->
			{#if repoInfo.branches.length > 0}
				<div class="flex items-center gap-2">
					<GitBranch size={16} class="text-gray-500" />
					<select
						bind:value={selectedBranch}
						onchange={() => switchBranch(selectedBranch)}
						class="text-sm border rounded px-2 py-1 bg-surface border-gray-300 dark:border-gray-600"
						disabled={loading}
					>
						{#each repoInfo.branches as branch}
							<option value={branch}>{branch}</option>
						{/each}
					</select>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Main content area -->
	<div class="flex-1 flex overflow-hidden">
		<!-- File tree -->
		{#if fileTree.length > 0}
			<div class="w-1/3 border-r border-gray-200 dark:border-gray-700 overflow-y-auto bg-surface">
				<div class="p-2">
					{#each fileTree as item}
						{@render treeItem(item, 0)}
					{/each}
				</div>
			</div>
		{/if}

		<!-- File content viewer -->
		<div class="flex-1 overflow-hidden bg-surface">
			{#if selectedFile && fileContent != null}
				<div class="h-full flex flex-col">
					<div class="p-2 border-b border-gray-200 dark:border-gray-700 bg-surface-secondary">
						<span class="font-medium text-sm text-primary">{selectedFile.name}</span>
					</div>
					<div class="flex-1 overflow-hidden">
						<Editor
							code={fileContent}
							scriptLang={getFileLanguage(selectedFile.name) as any}
							class="h-full"
						/>
					</div>
				</div>
			{:else if loading}
				<div class="h-full flex items-center justify-center text-secondary">
					<Loader2 class="animate-spin mr-2" size={20} />
					{cloning ? 'Cloning repository...' : 'Loading...'}
				</div>
			{:else if !repoInfo}
				<div class="h-full flex items-center justify-center text-center text-secondary p-8">
					<div>
						<File class="mx-auto mb-4 opacity-50" size={48} />
						<p class="text-lg mb-2">No repository loaded</p>
						<p class="text-sm">Enter a repository URL above to get started</p>
						{#if storedRepos.length > 0}
							<Button 
								size="sm" 
								variant="border" 
								btnClasses="mt-4"
								on:click={() => showCacheManager = true}
							>
								<Database size={14} class="mr-2" />
								View Cached Repositories
							</Button>
						{/if}
					</div>
				</div>
			{:else}
				<div class="h-full flex items-center justify-center text-center text-secondary p-8">
					<div>
						<File class="mx-auto mb-4 opacity-50" size={48} />
						<p class="text-lg mb-2">Select a file to view</p>
						<p class="text-sm">Choose a file from the tree to see its contents</p>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Error display -->
	{#if error}
		<div class="p-4 bg-red-50 dark:bg-red-900/20 border-t border-red-200 dark:border-red-800">
			<div class="flex items-center gap-2 text-red-600 dark:text-red-400 text-sm">
				<AlertCircle size={16} />
				{error}
			</div>
		</div>
	{/if}
</div>
