<script lang="ts">
	import { workspaceStore, userStore, enterpriseLicense } from '$lib/stores'
	import { GitSyncService, VariableService, type GitlabProject } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Alert } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import { GitBranch, Loader2 } from 'lucide-svelte'

	interface Props {
		resourceType: string
		args?: Record<string, any>
		/** The workspace the resource is being edited in, which is not always the
		 * one being navigated: the variable has to land where the resource will
		 * look for it. */
		workspace?: string
		onArgsUpdate?: (args: Record<string, any>) => void
	}

	let { resourceType, args = {}, workspace = undefined, onArgsUpdate }: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let baseUrl = $state('https://gitlab.com')
	let token = $state('')
	let search = $state('')
	let projects: GitlabProject[] = $state([])
	let selectedProject: string | undefined = $state(undefined)
	let variablePath = $state('')
	let loading = $state(false)
	let applying = $state(false)
	let listError: string | undefined = $state(undefined)

	// The project listing is served by an enterprise-only route, so on a build
	// without it the button would open a form whose first request 404s.
	let show = $derived(
		resourceType === 'git_repository' &&
			!!ws &&
			!!$enterpriseLicense &&
			($userStore?.is_admin || $userStore?.is_super_admin)
	)

	let project = $derived(projects.find((p) => p.path_with_namespace === selectedProject))

	// A path the user has not overridden tracks the selected project, so picking a
	// different one does not silently overwrite the first project's variable. The
	// host is part of it because the same project path exists on more than one
	// GitLab, and separators collapse, so `grp/a-b` and `grp/a_b` would otherwise
	// land on one name.
	function slug(value: string): string {
		return value
			.replace(/[^a-zA-Z0-9]+/g, '_')
			.replace(/^_+|_+$/g, '')
			.toLowerCase()
	}
	let suggestedVariablePath = $derived(
		project
			? `u/${$userStore?.username ?? 'admin'}/gitlab_${slug(
					new URL(project.http_url_to_repo).host
				)}_${slug(project.path_with_namespace)}_url`
			: ''
	)
	let variablePathTouched = $state(false)
	$effect(() => {
		if (!variablePathTouched) {
			variablePath = suggestedVariablePath
		}
	})

	// Host and path of a git remote, lowercased with `.git` dropped: the same
	// identity the backend compares, so "is this the same repository?" gets one
	// answer on both sides.
	function repoIdentity(url: string): string | undefined {
		try {
			const u = new URL(url.trim())
			const path = u.pathname
				.toLowerCase()
				.replace(/^\/+|\/+$/g, '')
				.replace(/\.git$/, '')
			return path ? `${u.host.toLowerCase()}/${path}` : undefined
		} catch {
			return undefined
		}
	}

	// What already lives at this path decides whether applying is safe. A
	// suggested path can collide with another project's, and the field is free
	// text, so the name alone proves nothing: only the repository the stored
	// value points at does.
	// `checking` exists so a path that has just changed is never treated as the
	// previous path's verdict: the answer is asynchronous, and applying against a
	// stale one is how an occupied variable gets overwritten anyway.
	type Occupant = 'checking' | 'free' | 'same-repo' | 'other'
	let occupant: Occupant = $state('free')
	$effect(() => {
		const path = variablePath
		const workspace = ws
		const target = project ? repoIdentity(project.http_url_to_repo) : undefined
		if (!workspace || !path || !target) {
			occupant = 'free'
			return
		}
		occupant = 'checking'
		let cancelled = false
		VariableService.existsVariable({ workspace, path })
			.then(async (exists) => {
				if (cancelled) return
				if (!exists) {
					occupant = 'free'
					return
				}
				// Reading it decrypts a secret, which is why this is admin-only.
				const current = await VariableService.getVariableValue({ workspace, path }).catch(
					() => undefined
				)
				if (cancelled) return
				// Unreadable counts as occupied: it is someone else's until proven otherwise.
				occupant = current && repoIdentity(current) === target ? 'same-repo' : 'other'
			})
			.catch(() => {
				if (!cancelled) occupant = 'other'
			})
		return () => {
			cancelled = true
		}
	})

	async function listProjects() {
		if (!ws) return
		loading = true
		listError = undefined
		try {
			projects = await GitSyncService.listGitlabProjects({
				workspace: ws,
				requestBody: { base_url: baseUrl, token, search: search || undefined }
			})
			selectedProject = projects[0]?.path_with_namespace
			if (projects.length === 0) {
				listError = 'The token can see no project with at least the Developer role'
			}
		} catch (err) {
			listError = err?.body ?? err?.message ?? String(err)
			projects = []
			selectedProject = undefined
		} finally {
			loading = false
		}
	}

	// The credential travels in the remote URL, and the whole URL lives in one
	// secret variable: that is the shape Windmill can rewrite when it renews the
	// token, and it keeps the credential out of the resource itself.
	function repositoryUrl(p: GitlabProject): string {
		const url = new URL(p.http_url_to_repo)
		url.username = 'oauth2'
		url.password = token
		return url.toString()
	}

	async function apply(close: (_: any) => void) {
		// The token is cleared once stored, and re-applying without one would
		// overwrite the stored credential with an empty password.
		const pathIsWritable = occupant === 'free' || occupant === 'same-repo'
		if (!ws || !project || !token || !pathIsWritable) return
		// Everything this writes is read once, here, before the first await. The
		// selector and the path field stay live while the requests are in flight,
		// so re-reading them later could store one project's URL under another's
		// path, or set the wrong default branch.
		const workspace = ws
		const chosen = project
		const path = variablePath
		const value = repositoryUrl(chosen)
		const target = repoIdentity(chosen.http_url_to_repo)
		applying = true
		try {
			const exists = await VariableService.existsVariable({
				workspace,
				path
			})
			// Re-read rather than trust the state the button was enabled from: the
			// path can change between the check and the click, and another writer
			// can take the path in between. A path holding anything but this same
			// repository is never written over — that would repoint every resource
			// using it, silently, at this project.
			if (exists) {
				const current = await VariableService.getVariableValue({
					workspace,
					path
				}).catch(() => undefined)
				if (!current || !target || repoIdentity(current) !== target) {
					occupant = 'other'
					sendUserToast(`${path} holds something else. Choose another path.`, true)
					return
				}
				await VariableService.updateVariable({
					workspace,
					path,
					requestBody: { value, is_secret: true }
				})
			} else {
				await VariableService.createVariable({
					workspace,
					requestBody: {
						path,
						value,
						is_secret: true,
						description: `Git remote for ${chosen.path_with_namespace}, including its GitLab token`
					}
				})
			}
			onArgsUpdate?.({
				...args,
				url: `$var:${path}`,
				is_github_app: false,
				branch: args.branch || chosen.default_branch || undefined
			})
			token = ''
			projects = []
			selectedProject = undefined
			variablePathTouched = false
			sendUserToast(`Repository URL stored in the secret variable ${path}`)
			close(null)
		} catch (err) {
			sendUserToast(`Could not store the repository URL: ${err?.body ?? err?.message}`, true)
		} finally {
			applying = false
		}
	}
</script>

{#if show}
	<Popover
		documentationLink="https://www.windmill.dev/docs/integrations/git_repository"
		contentClasses="overflow-auto"
	>
		{#snippet trigger()}
			<Button variant="default" unifiedSize="xs" startIcon={{ icon: GitBranch }} nonCaptureEvent>
				GitLab
			</Button>
		{/snippet}
		{#snippet content({ close })}
			<div class="block text-primary p-4">
				<div class="flex flex-col gap-4 w-[600px]">
					<div class="flex flex-col gap-y-1">
						<div class="text-xs font-semibold text-emphasis">GitLab instance</div>
						<TextInput bind:value={baseUrl} size="sm" />
					</div>
					<div class="flex flex-col gap-y-1">
						<div class="text-xs font-semibold text-emphasis">Project access token</div>
						<div class="text-xs font-normal text-secondary">
							Create it on the project you are syncing, with the api scope and at least the
							Developer role. Maintainer also lets Windmill manage the webhook and merge requests.
							Its name becomes the author of the commits and merge requests Windmill creates.
						</div>
						<div class="text-xs font-normal text-secondary">
							Use a separate token per repository. A group token works and covers every project in
							the group, but renewal updates one repository at a time, so the others keep the
							replaced token.
						</div>
						<TextInput bind:value={token} size="sm" inputProps={{ type: 'password' }} />
						<div class="text-2xs font-normal text-hint">
							Used to list projects now, then stored in a secret variable
						</div>
					</div>
					<div class="flex flex-col gap-y-1">
						<div class="text-xs font-semibold text-emphasis">Filter projects</div>
						<TextInput bind:value={search} size="sm" inputProps={{ placeholder: 'Optional' }} />
					</div>
					<div>
						<Button
							variant="default"
							unifiedSize="sm"
							disabled={!token || !baseUrl || loading}
							startIcon={{
								icon: loading ? Loader2 : GitBranch,
								classes: loading ? 'animate-spin' : ''
							}}
							onclick={listProjects}
						>
							List projects
						</Button>
					</div>
					{#if listError}
						<Alert type="error" title="Could not list projects" size="xs">{listError}</Alert>
					{/if}
					{#if projects.length > 0}
						<div class="flex flex-col gap-y-1">
							<div class="text-xs font-semibold text-emphasis">Project</div>
							<Select
								items={projects.map((p) => ({
									label: p.path_with_namespace,
									value: p.path_with_namespace
								}))}
								bind:value={selectedProject}
								clearable={false}
								disabled={applying}
							/>
						</div>
						<div class="flex flex-col gap-y-1">
							<div class="text-xs font-semibold text-emphasis">Secret variable</div>
							<div class="text-xs font-normal text-secondary">
								Where the repository URL and its token are kept. Windmill rewrites this variable
								when it renews the token, so every consumer keeps working.
							</div>
							<TextInput
								bind:value={variablePath}
								size="sm"
								inputProps={{
									oninput: () => (variablePathTouched = true),
									disabled: applying
								}}
							/>
							{#if occupant === 'checking'}
								<div class="text-2xs font-normal text-hint">Checking this path...</div>
							{:else if occupant === 'other'}
								<div class="text-2xs font-normal text-red-600 dark:text-red-400">
									This variable already holds something else. Choose another path, or anything using
									it would start pointing at this project.
								</div>
							{:else if occupant === 'same-repo'}
								<div class="text-2xs font-normal text-hint">
									This variable already points at this project, and its token will be replaced.
								</div>
							{/if}
						</div>
						<div class="flex justify-end">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!project ||
									!variablePath ||
									!token ||
									!(occupant === 'free' || occupant === 'same-repo') ||
									applying}
								startIcon={{
									icon: applying ? Loader2 : GitBranch,
									classes: applying ? 'animate-spin' : ''
								}}
								onclick={() => apply(close)}
							>
								Use this project
							</Button>
						</div>
					{/if}
				</div>
			</div>
		{/snippet}
	</Popover>
{/if}
