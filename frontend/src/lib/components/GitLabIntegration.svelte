<script lang="ts">
	import { workspaceStore, userStore, enterpriseLicense } from '$lib/stores'
	import { GitSyncService, type GitlabProject } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Alert } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import { GitBranch, Gitlab, Loader2 } from 'lucide-svelte'

	interface Props {
		resourceType: string
		args?: Record<string, any>
		/** The workspace the resource is being edited in, which is not always the
		 * one being navigated: the credential has to land where the resource will
		 * look for it. */
		workspace?: string
		/** Path the resource is being saved at. The stored credential is keyed by
		 * it, so the picker cannot run before the resource has a path. */
		resourcePath?: string
		onArgsUpdate?: (args: Record<string, any>) => void
	}

	let {
		resourceType,
		args = {},
		workspace = undefined,
		resourcePath = undefined,
		onArgsUpdate
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let baseUrl = $state('https://gitlab.com')
	let token = $state('')
	let search = $state('')
	let projects: GitlabProject[] = $state([])
	let selectedProject: string | undefined = $state(undefined)
	let loading = $state(false)
	let applying = $state(false)
	let listError: string | undefined = $state(undefined)

	// Shown alongside the GitHub App button and on the same terms, so the two
	// read as one choice rather than one option and one absence.
	let show = $derived(
		resourceType === 'git_repository' &&
			!!ws &&
			($userStore?.is_admin || $userStore?.is_super_admin)
	)
	// The project listing is served by an enterprise-only route, so on a build
	// without it the form's first request would 404. The button still shows,
	// disabled and labelled, because a missing button reads as "GitLab is not
	// supported" rather than "this needs a licence".
	let enabled = $derived(!!$enterpriseLicense)

	let project = $derived(projects.find((p) => p.path_with_namespace === selectedProject))
	let hasPath = $derived(!!resourcePath && resourcePath !== '')

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

	async function apply(close: (_: any) => void) {
		if (!ws || !project || !token || !resourcePath) return
		// Everything this writes is read once, here, before the first await. The
		// selector stays live while the request is in flight, so re-reading it
		// later could store one project's token against another's URL.
		const workspace = ws
		const chosen = project
		const repoPath = resourcePath
		const url = chosen.http_url_to_repo
		applying = true
		try {
			await GitSyncService.setGitCredential({
				workspace,
				requestBody: { repo_path: repoPath, repo_url: url, token }
			})
			onArgsUpdate?.({
				...args,
				url,
				is_github_app: false,
				branch: args.branch || chosen.default_branch || undefined
			})
			token = ''
			projects = []
			selectedProject = undefined
			sendUserToast(`Windmill stored the token for ${chosen.path_with_namespace}`)
			close(null)
		} catch (err) {
			sendUserToast(`Could not store the token: ${err?.body ?? err?.message}`, true)
		} finally {
			applying = false
		}
	}
</script>

{#if show}
	<Popover
		documentationLink="https://www.windmill.dev/docs/integrations/git_repository"
		disabled={!enabled}
		contentClasses="overflow-auto"
	>
		{#snippet trigger()}
			<Button
				variant="default"
				unifiedSize="sm"
				disabled={!enabled}
				startIcon={{ icon: Gitlab }}
				nonCaptureEvent
			>
				{enabled ? 'GitLab' : 'GitLab (ee only)'}
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
							Windmill keeps it for this repository and hands it only to this workspace's sync jobs.
							Forks of this workspace use it without holding a copy.
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
						{#if hasPath}
							<div class="text-2xs font-normal text-hint">
								Stored for the resource at {resourcePath}. Give the resource its final path before
								applying, so the token stays with it.
							</div>
						{:else}
							<Alert type="warning" title="The resource needs a path first" size="xs">
								The token is kept against the resource's path. Name the resource, then pick the
								project.
							</Alert>
						{/if}
						<div class="flex justify-end">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!project || !token || !hasPath || applying}
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
