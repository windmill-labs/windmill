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
		/** The picked project's token, handed over for the form to store once the
		 * resource is saved and its path is final. */
		onCredentialSelected?: (credential: { token: string; repoUrl: string }) => void
		onArgsUpdate?: (args: Record<string, any>) => void
	}

	let {
		resourceType,
		args = {},
		workspace = undefined,
		onCredentialSelected,
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

	function apply(close: (_: any) => void) {
		if (!project || !token) return
		const chosen = project
		const url = chosen.http_url_to_repo
		// Handed to the form instead of stored now. The credential is filed under
		// the resource's path, which is not settled until the resource is saved,
		// and writing here would outlive an edit the user then cancels: picking a
		// different project and backing out would have replaced a working token.
		onCredentialSelected?.({ token, repoUrl: url })
		onArgsUpdate?.({
			...args,
			url,
			is_github_app: false,
			// The URL carries no credential, so without this the resource is
			// indistinguishable from a public remote: it is what tells the rest of
			// the UI the token is Windmill's to keep and renew, and which host it
			// belongs to.
			managed_credential: 'gitlab',
			branch: args.branch || chosen.default_branch || undefined
		})
		token = ''
		projects = []
		selectedProject = undefined
		sendUserToast(`${chosen.path_with_namespace} selected. Its token is stored when you save.`)
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
						<div class="flex justify-end">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!project || !token}
								startIcon={{ icon: GitBranch }}
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
