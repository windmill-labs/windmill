<script lang="ts">
	/**
	 * Homepage header badge for the workspace's Git Sync: the last sync and whether the connection
	 * is healthy when it is configured, and where to set it up when it is not. Admins only: the
	 * settings endpoint and the sync jobs (run as a system identity) are not readable by others.
	 */
	import { onDestroy } from 'svelte'
	import { resource } from 'runed'
	import { twMerge } from 'tailwind-merge'
	import {
		ArrowDownToLine,
		ArrowUpFromLine,
		CheckCircle2,
		ExternalLink,
		EyeOff,
		Loader2,
		Settings,
		XCircle
	} from 'lucide-svelte'
	import {
		JobService,
		ResourceService,
		VariableService,
		WorkspaceService,
		type GitRepositorySettings,
		type Job
	} from '$lib/gen'
	import GitIcon from '$lib/components/icons/GitIcon.svelte'
	import GithubIcon from '$lib/components/icons/GithubIcon.svelte'
	import GitlabIcon from '$lib/components/icons/GitlabIcon.svelte'
	import BitbucketIcon from '$lib/components/icons/BitbucketIcon.svelte'
	import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { Button, ButtonType } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { pageHeader, PHONE_BAR } from '$lib/components/pageHeaderRegistry.svelte'
	import {
		isGitSyncHomeBadgeHidden,
		setGitSyncHomeBadgeHidden
	} from './gitSyncHomeBadgeVisibility.svelte'

	type State = 'error' | 'warning' | 'running' | 'ok' | 'never'
	type Provider = 'github' | 'gitlab' | 'bitbucket' | 'azure'
	type SyncJob = Job & { pull: boolean }
	type RepoStatus = {
		path: string
		jobs: SyncJob[]
		state: State
		problem?: string
		lastSyncAt?: string
	}

	const JOBS_PER_REPO = 5
	const POLL_MS = 5000
	const EXPIRY_WARNING_DAYS = 7
	const DOCS_URL = 'https://www.windmill.dev/docs/advanced/git_sync'
	const RANK: Record<State, number> = { error: 4, warning: 3, running: 2, ok: 1, never: 0 }

	const isAdmin = $derived(Boolean($userStore?.is_admin || $superadmin))
	const workspace = $derived(
		isAdmin && $workspaceStore && !isGitSyncHomeBadgeHidden($workspaceStore)
			? $workspaceStore
			: undefined
	)
	const phone = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < PHONE_BAR)

	const config = resource(
		() => workspace,
		async (workspace, _prev, { signal }) => {
			if (!workspace) return undefined
			const [settings, eligibility] = await Promise.all([
				WorkspaceService.getSettings({ workspace }),
				WorkspaceService.getGitSyncEnabled({ workspace })
			])
			dropIfSuperseded(signal)
			return {
				workspace,
				repos: (settings.git_sync?.repositories ?? []).filter((r) => r.git_repo_resource_path),
				canEnable: eligibility.enabled
			}
		}
	)
	// The previous workspace's config stays loaded until the new one lands, and hiding must drop
	// it at once.
	const current = $derived(config.current?.workspace === workspace ? config.current : undefined)

	let pollTimer: ReturnType<typeof setTimeout> | undefined
	// A response still in flight at teardown must not schedule a poll nothing would ever clear.
	let destroyed = false
	onDestroy(() => {
		destroyed = true
		clearTimeout(pollTimer)
	})

	// resource() stores every response, even one a newer request has superseded, so a fetcher
	// drops its own result once its signal is aborted (runed ignores an AbortError).
	function dropIfSuperseded(signal: AbortSignal) {
		if (signal.aborted) throw new DOMException('Superseded', 'AbortError')
	}

	function syncQueries(repo: GitRepositorySettings) {
		// Every sync job names its repository without `$res:`: the server strips it, and the Git Sync
		// settings strip it when they load, before their Pull and Push run the hub script directly.
		const repoPath = repo.git_repo_resource_path.replace(/^\$res:/, '')
		// A PR diff check or a preview is a pull job too, with `dry_run: true`, and it applies
		// nothing, so it must not count as a sync. The list returns no args to filter on, so each
		// query pins one real kind instead: pulls, automatic (`deploymentcallback`) or manual
		// (`script_hub`), by `dry_run: false, pull: true`; manual pushes by `pull: false`; and
		// deploy pushes by `use_individual_branch`, which every push sets and no pull does.
		return [
			{
				kinds: 'deploymentcallback,script_hub',
				args: { repo_url_resource_path: repoPath, dry_run: false, pull: true },
				pull: true
			},
			{
				kinds: 'script_hub',
				args: { repo_url_resource_path: repoPath, dry_run: false, pull: false },
				pull: false
			},
			{
				kinds: 'deploymentcallback',
				args: { repo_url_resource_path: repoPath, use_individual_branch: true },
				pull: false
			},
			{
				kinds: 'deploymentcallback',
				args: { repo_url_resource_path: repoPath, use_individual_branch: false },
				pull: false
			}
		]
	}

	const jobs = resource(
		() => current,
		async (config, _prev, { signal }) => {
			clearTimeout(pollTimer)
			if (!config?.repos.length) return undefined
			const lists = await Promise.all(
				config.repos.map(async (repo) => {
					const lists = await Promise.all(
						syncQueries(repo).map(async (query) =>
							(
								await JobService.listJobs({
									workspace: config.workspace,
									jobKinds: query.kinds,
									args: JSON.stringify(query.args),
									perPage: JOBS_PER_REPO
								})
							).map((job): SyncJob => Object.assign(job, { pull: query.pull }))
						)
					)
					return lists
						.flat()
						.sort((a, b) => (b.created_at ?? '').localeCompare(a.created_at ?? ''))
						.slice(0, JOBS_PER_REPO)
				})
			)
			dropIfSuperseded(signal)
			// Followed only while something is in flight, so an idle homepage makes no requests.
			if (!destroyed && lists.some((list) => list.some((job) => job.type === 'QueuedJob'))) {
				pollTimer = setTimeout(() => jobs.refetch(), POLL_MS)
			}
			return {
				workspace: config.workspace,
				byRepo: Object.fromEntries(
					config.repos.map((repo, i) => [repo.git_repo_resource_path, lists[i]])
				)
			}
		}
	)

	type Remote = { provider?: Provider; webUrl?: string }

	function parseRemote(url: string): Remote {
		// Covers [scheme://][user[:token]@]host[:port]/path and scp-style git@host:org/repo.
		const m = url
			.trim()
			.match(/^(?:([a-z+]+):\/\/)?(?:[^@/]+@)?([^/:]+)(?::(\d+))?[:/](.+?)(?:\.git)?\/?$/i)
		if (!m) return {}
		const scheme = m[1]
		const port = m[3]
		let hostname = m[2]
		let repoPath = m[4]
		// Azure DevOps serves SSH from its own host, under a v3/ path the web does not use.
		const azureSsh = repoPath.match(/^v3\/([^/]+)\/([^/]+)\/([^/]+)$/)
		if (azureSsh && hostname.toLowerCase() === 'ssh.dev.azure.com') {
			hostname = 'dev.azure.com'
			repoPath = `${azureSsh[1]}/${azureSsh[2]}/_git/${azureSsh[3]}`
		} else if (azureSsh && hostname.toLowerCase() === 'vs-ssh.visualstudio.com') {
			hostname = `${azureSsh[1]}.visualstudio.com`
			repoPath = `${azureSsh[2]}/_git/${azureSsh[3]}`
		}
		const host = hostname.toLowerCase()
		const provider: Provider | undefined = host.includes('github')
			? 'github'
			: host.includes('gitlab')
				? 'gitlab'
				: host.includes('bitbucket')
					? 'bitbucket'
					: host === 'dev.azure.com' || host.endsWith('.visualstudio.com')
						? 'azure'
						: undefined
		// The credentials stay out of the link; an SSH port is not the web port.
		const http = scheme === 'http' || scheme === 'https'
		const webUrl = `${http ? scheme : 'https'}://${hostname}${http && port ? `:${port}` : ''}/${repoPath}`
		return { provider, webUrl }
	}

	async function repoRemote(workspace: string, repo: GitRepositorySettings): Promise<Remote> {
		const path = repo.git_repo_resource_path.replace(/^\$res:/, '')
		const value = (await ResourceService.getResource({ workspace, path })).value as
			| Record<string, unknown>
			| undefined
		let url = value?.url
		const varPath = typeof url === 'string' ? url.match(/^\$var:(.+)$/)?.[1] : undefined
		if (varPath) {
			// Never decrypted: each decrypt is an audit entry, and this runs on every homepage load
			// just to draw a logo. A secret URL leaves the repo with the generic icon and no link.
			const variable = await VariableService.getVariable({
				workspace,
				path: varPath,
				decryptSecret: false
			})
			url = variable.is_secret ? undefined : variable.value
		}
		const remote = typeof url === 'string' ? parseRemote(url) : {}
		if (value?.is_github_app === true) remote.provider = 'github'
		if (repo.credential?.provider === 'gitlab') remote.provider = 'gitlab'
		return remote
	}

	// Separate from the jobs so polling does not refetch every repository resource.
	const remotes = resource(
		() => current,
		async (config, _prev, { signal }) => {
			if (!config?.repos.length) return undefined
			const found = await Promise.all(
				config.repos.map((repo) => repoRemote(config.workspace, repo).catch(() => ({})))
			)
			dropIfSuperseded(signal)
			return {
				workspace: config.workspace,
				byRepo: Object.fromEntries(
					config.repos.map((repo, i) => [repo.git_repo_resource_path, found[i] as Remote])
				)
			}
		}
	)
	const remoteByRepo = $derived(
		remotes.current && remotes.current.workspace === current?.workspace
			? remotes.current.byRepo
			: {}
	)

	function credentialDaysLeft(repo: GitRepositorySettings): number | undefined {
		const expiresAt = repo.credential?.expires_at
		if (!expiresAt || repo.credential?.rotatable) return undefined
		// GitLab expires a token on a date, so count calendar days rather than from now.
		const expiry = new Date(`${expiresAt}T00:00:00Z`).getTime()
		const now = new Date()
		const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate())
		return Math.round((expiry - today) / 86_400_000)
	}

	function repoStatus(repo: GitRepositorySettings, repoJobs: SyncJob[]): RepoStatus {
		const path = repo.git_repo_resource_path.replace(/^\$res:/, '')
		const latest = repoJobs[0]
		const lastSuccess = repoJobs.find((j) => j.type === 'CompletedJob' && j.success)
		const lastSyncAt = lastSuccess?.type === 'CompletedJob' ? lastSuccess.completed_at : undefined
		const common = { path, jobs: repoJobs, lastSyncAt }
		const configError =
			repo.credential?.error ?? repo.open_pr_error ?? repo.auto_pull?.webhook_error
		if (configError) return { ...common, state: 'error', problem: configError }
		if (latest?.type === 'CompletedJob' && !latest.success) {
			return { ...common, state: 'error', problem: 'Last sync failed' }
		}
		const daysLeft = credentialDaysLeft(repo)
		if (daysLeft !== undefined && daysLeft <= EXPIRY_WARNING_DAYS) {
			return {
				...common,
				state: 'warning',
				problem:
					daysLeft < 0
						? 'Repository token expired'
						: `Repository token expires on ${repo.credential?.expires_at}`
			}
		}
		if (latest?.type === 'QueuedJob') return { ...common, state: 'running' }
		return { ...common, state: latest ? 'ok' : 'never' }
	}

	const jobsByRepo = $derived(
		jobs.current && jobs.current.workspace === current?.workspace ? jobs.current.byRepo : {}
	)
	const repos = $derived(
		(current?.repos ?? []).map((repo) => ({
			...repoStatus(repo, jobsByRepo[repo.git_repo_resource_path] ?? []),
			...remoteByRepo[repo.git_repo_resource_path]
		}))
	)
	// One logo only when every repository is on the same detected provider.
	const provider = $derived(
		repos.length > 0 && repos.every((r) => r.provider && r.provider === repos[0].provider)
			? repos[0].provider
			: undefined
	)
	const overall = $derived(
		repos.reduce<State>((worst, r) => (RANK[r.state] > RANK[worst] ? r.state : worst), 'never')
	)
	const lastSyncAt = $derived(
		repos
			.map((r) => r.lastSyncAt)
			.filter((at): at is string => !!at)
			.sort()
			.at(-1)
	)

	const label = $derived(
		{
			error: 'Sync failed',
			warning: 'Git Sync needs attention',
			running: 'Syncing…',
			ok: 'Synced',
			never: 'Never synced'
		}[overall]
	)

	function hide() {
		if ($workspaceStore) setGitSyncHomeBadgeHidden($workspaceStore, true)
	}

	// The Popover renders its trigger as a <button>, so this wears the design system's subtle
	// button rather than containing one — a <Button> here would nest a button inside a button.
	const triggerClass = twMerge(
		'inline-flex items-center gap-1 rounded-md whitespace-nowrap transition-all',
		ButtonType.VariantStyles.subtle,
		ButtonType.UnifiedSizingClasses.sm,
		ButtonType.UnifiedHeightClasses.sm,
		ButtonType.UnifiedFontSizes.sm,
		'text-xs'
	)
	const iconSize = ButtonType.UnifiedIconSizes.sm
	const DOT_CLASS: Record<State, string | undefined> = {
		error: 'bg-red-500',
		warning: 'bg-yellow-500',
		running: 'bg-blue-500 animate-pulse',
		ok: 'bg-green-500',
		never: undefined
	}
</script>

{#snippet stateDot(state: State)}
	{#if DOT_CLASS[state]}
		<span class="size-2 shrink-0 rounded-full {DOT_CLASS[state]}" aria-hidden="true"></span>
	{/if}
{/snippet}

{#snippet providerIcon(provider: Provider | undefined, size: number)}
	<span class="inline-flex shrink-0">
		{#if provider === 'github'}
			<GithubIcon {size} />
		{:else if provider === 'gitlab'}
			<GitlabIcon {size} />
		{:else if provider === 'bitbucket'}
			<BitbucketIcon width="{size}px" height="{size}px" />
		{:else if provider === 'azure'}
			<AzureIcon {size} />
		{:else}
			<GitIcon {size} />
		{/if}
	</span>
{/snippet}

{#if current && current.repos.length > 0}
	<div class="my-2">
		<Popover
			placement="bottom-end"
			class={triggerClass}
			triggerAttrs={{ 'aria-label': `Git Sync: ${label}`, title: `Git Sync: ${label}` }}
			onOpen={() => jobs.refetch()}
		>
			{#snippet trigger()}
				{@render providerIcon(provider, 18)}
				{@render stateDot(overall)}
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-3 p-4 text-xs w-96 max-w-[calc(100vw-2rem)]">
					<div class="flex items-baseline justify-between gap-2">
						<span class="font-semibold text-emphasis">Git Sync</span>
						<span class="text-secondary">
							{#if overall === 'ok' && lastSyncAt}
								Synced <TimeAgo date={lastSyncAt} />
							{:else}
								{label}
							{/if}
						</span>
					</div>
					{#each repos as repo (repo.path)}
						<div class="flex flex-col gap-1.5">
							<div class="flex items-center gap-1.5 min-w-0">
								{@render providerIcon(repo.provider, 14)}
								{@render stateDot(repo.state)}
								<span class="font-mono text-primary truncate" title={repo.path}>{repo.path}</span>
								{#if repo.webUrl}
									<a
										href={repo.webUrl}
										target="_blank"
										rel="noopener noreferrer"
										class="ml-auto inline-flex shrink-0 items-center gap-1 text-accent hover:underline"
										title={repo.webUrl}
									>
										Open repository <ExternalLink size={12} />
									</a>
								{/if}
							</div>
							{#if repo.problem}
								<p class={repo.state === 'error' ? 'text-red-500' : 'text-yellow-600'}>
									{repo.problem}
								</p>
							{/if}
							{#if repo.jobs.length === 0}
								<p class="text-secondary"
									>No sync job yet. The next deploy will push to this repository.</p
								>
							{:else}
								<div class="flex flex-col">
									{#each repo.jobs as job (job.id)}
										<a
											href="/run/{job.id}?workspace={$workspaceStore}"
											class="flex items-center gap-2 rounded px-1.5 py-1 hover:bg-surface-hover"
										>
											{#if job.type === 'QueuedJob'}
												<Loader2 size={12} class="shrink-0 animate-spin text-secondary" />
											{:else if job.type === 'CompletedJob' && job.success}
												<CheckCircle2 size={12} class="shrink-0 text-green-500" />
											{:else}
												<XCircle size={12} class="shrink-0 text-red-500" />
											{/if}
											{#if job.pull}
												<ArrowDownToLine size={12} class="shrink-0 text-secondary" />
												<span class="text-primary">Pull</span>
											{:else}
												<ArrowUpFromLine size={12} class="shrink-0 text-secondary" />
												<span class="text-primary">Push</span>
											{/if}
											<span class="ml-auto text-secondary whitespace-nowrap">
												{#if job.created_at}
													<TimeAgo date={job.created_at} />
												{/if}
											</span>
										</a>
									{/each}
								</div>
							{/if}
						</div>
					{/each}
					<a
						href="/runs?job_kinds=deploymentcallbacks&workspace={$workspaceStore}"
						class="text-accent hover:underline"
					>
						All sync jobs
					</a>
					<div class="flex items-center justify-between gap-2">
						<Button
							variant="subtle"
							unifiedSize="sm"
							startIcon={{ icon: EyeOff }}
							onClick={hide}
							title="Hide from the homepage. Show it again from the homepage menu or the Git Sync settings."
						>
							Hide
						</Button>
						<Button
							variant="default"
							unifiedSize="sm"
							startIcon={{ icon: Settings }}
							href="/workspace_settings?tab=git_sync"
						>
							Git Sync settings
						</Button>
					</div>
				</div>
			{/snippet}
		</Popover>
	</div>
{:else if current?.canEnable}
	<div class="my-2">
		<Popover
			placement="bottom-end"
			class={twMerge(triggerClass, 'text-hint')}
			triggerAttrs={{ 'aria-label': 'Set up Git Sync' }}
		>
			{#snippet trigger()}
				<GitIcon size={iconSize} />
				{#if !phone}Git Sync{/if}
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-3 p-4 text-xs w-80 max-w-[calc(100vw-2rem)]">
					<span class="font-semibold text-emphasis">Git Sync is not set up</span>
					<p class="text-primary">
						Commit every deploy of this workspace to a Git repository, review changes in pull
						requests, and deploy new commits back into the workspace.
					</p>
					<a href={DOCS_URL} target="_blank" class="inline-flex items-center gap-1 text-accent">
						Git Sync documentation <ExternalLink size={12} />
					</a>
					<div class="flex items-center justify-between gap-2">
						<Button variant="subtle" unifiedSize="sm" startIcon={{ icon: EyeOff }} onClick={hide}>
							Hide
						</Button>
						<Button variant="accent" unifiedSize="sm" href="/workspace_settings?tab=git_sync">
							Set up Git Sync
						</Button>
					</div>
				</div>
			{/snippet}
		</Popover>
	</div>
{/if}
