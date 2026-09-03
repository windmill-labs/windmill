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

	// Applying replaces whatever is at this path, and anything else pointing at it
	// would silently start using a different repository.
	let variableExists = $state(false)
	$effect(() => {
		const path = variablePath
		const workspace = ws
		if (!workspace || !path) {
			variableExists = false
			return
		}
		let cancelled = false
		VariableService.existsVariable({ workspace, path })
			.then((e) => {
				if (!cancelled) variableExists = e
			})
			.catch(() => {
				if (!cancelled) variableExists = false
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
		if (!ws || !project || !token) return
		applying = true
		try {
			const value = repositoryUrl(project)
			const exists = await VariableService.existsVariable({
				workspace: ws,
				path: variablePath
			})
			if (exists) {
				await VariableService.updateVariable({
					workspace: ws,
					path: variablePath,
					requestBody: { value, is_secret: true }
				})
			} else {
				await VariableService.createVariable({
					workspace: ws,
					requestBody: {
						path: variablePath,
						value,
						is_secret: true,
						description: `Git remote for ${project.path_with_namespace}, including its GitLab token`
					}
				})
			}
			onArgsUpdate?.({
				...args,
				url: `$var:${variablePath}`,
				is_github_app: false,
				branch: args.branch || project.default_branch || undefined
			})
			token = ''
			projects = []
			selectedProject = undefined
			variablePathTouched = false
			sendUserToast(`Repository URL stored in the secret variable ${variablePath}`)
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
						<div class="text-xs font-semibold text-emphasis">Group access token</div>
						<div class="text-xs font-normal text-secondary">
							Create it in the group that owns the project, with the api scope and at least the
							Developer role. Maintainer also lets Windmill manage the webhook and merge requests.
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
								inputProps={{ oninput: () => (variablePathTouched = true) }}
							/>
							{#if variableExists}
								<div class="text-2xs font-normal text-hint">
									This variable already exists and will be replaced. Anything else using it will
									point at this repository.
								</div>
							{/if}
						</div>
						<div class="flex justify-end">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!project || !variablePath || !token || applying}
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
