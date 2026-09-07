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
		/** Fired once the picked project's token is stored, so a form that would
		 * otherwise file the URL as a secret knows it no longer holds one. */
		onCredentialStored?: () => void
		onArgsUpdate?: (args: Record<string, any>) => void
	}

	let {
		resourceType,
		args = {},
		workspace = undefined,
		onCredentialStored,
		onArgsUpdate
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let baseUrl = $state('https://gitlab.com')
	let token = $state('')
	let search = $state('')
	let projects: GitlabProject[] = $state([])
	let selectedProject: string | undefined = $state(undefined)
	let loading = $state(false)
	let listError: string | undefined = $state(undefined)
	let applying = $state(false)
	let applyError: string | undefined = $state(undefined)
	/** The token the current listing was made with. Editing the token afterwards
	 * leaves projects on screen that were never checked against it, and applying
	 * would store the new token for a project chosen under the old one. */
	let listedToken = $state('')

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
	let staleListing = $derived(projects.length > 0 && token !== listedToken)

	async function listProjects() {
		if (!ws) return
		loading = true
		listError = undefined
		try {
			// Captured before the await, not after: the field stays editable while the
			// request is in flight, and reading it on the way back would record a
			// token this listing was never checked against.
			const listedWith = token
			projects = await GitSyncService.listGitlabProjects({
				workspace: ws,
				requestBody: { base_url: baseUrl, token: listedWith, search: search || undefined }
			})
			listedToken = listedWith
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
		if (!project || !token || applying || staleListing) return
		const chosen = project
		const url = chosen.http_url_to_repo
		applying = true
		applyError = undefined
		try {
			// Stored against the project it was issued for, the way a GitHub App
			// installation is stored against the account it covers. Nothing waits on
			// the resource: its path is not settled while it is being created, and
			// deferring the write would tie one repository's token to another
			// repository's save succeeding.
			await GitSyncService.setGitCredential({
				workspace: ws!,
				requestBody: { repo_url: url, token }
			})
		} catch (err) {
			applyError = err?.body ?? err?.message ?? String(err)
			return
		} finally {
			applying = false
		}
		onCredentialStored?.()
		onArgsUpdate?.({
			...args,
			url,
			is_github_app: false,
			branch: args.branch || chosen.default_branch || undefined
		})
		token = ''
		listedToken = ''
		projects = []
		selectedProject = undefined
		sendUserToast(`${chosen.path_with_namespace} selected and its token stored`)
		close(null)
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
							Create it on the project you are syncing, with the <code>api</code> scope and the
							<code>Maintainer</code> role.
						</div>
						<TextInput bind:value={token} size="sm" inputProps={{ type: 'password' }} />
						<div class="text-2xs font-normal text-hint">
							Windmill stores it and renews it before it expires. Use one token per project: a group
							token covers the group, but renewal replaces it for one project at a time.
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
					{#if staleListing}
						<div class="text-xs font-normal text-secondary">
							The token changed. List the projects again to choose one it can reach.
						</div>
					{/if}
					{#if projects.length > 0 && !staleListing}
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
						{#if applyError}
							<Alert type="error" title="Could not store the token" size="xs">{applyError}</Alert>
						{/if}
						<div class="flex justify-end">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!project || !token || applying}
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
