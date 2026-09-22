<script lang="ts">
	import { GitBranch, Loader2 } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { GitSyncService, type GitlabProject } from '$lib/gen'
	import { apiErrorMessage } from '$lib/utils'
	import type { RepoConnection } from './repoConnection'

	type Props = {
		workspace: string
		connection: RepoConnection | undefined
	}

	let { workspace, connection = $bindable() }: Props = $props()

	let baseUrl = $state('https://gitlab.com')
	let token = $state('')
	let projects: GitlabProject[] = $state([])
	let selectedProject: string | undefined = $state(undefined)
	let loading = $state(false)
	let listError: string | undefined = $state(undefined)
	/** The token the current listing was made with: a project chosen under another
	 * token must not be stored with this one. */
	let listedToken = $state('')

	const staleListing = $derived(projects.length > 0 && token !== listedToken)
	const project = $derived(projects.find((p) => p.path_with_namespace === selectedProject))

	$effect(() => {
		connection =
			project && !staleListing
				? { url: project.http_url_to_repo, heldToken: listedToken }
				: undefined
	})

	async function listProjects() {
		loading = true
		listError = undefined
		const listedWith = token
		try {
			projects = await GitSyncService.listGitlabProjects({
				workspace,
				requestBody: { base_url: baseUrl, token: listedWith }
			})
			listedToken = listedWith
			selectedProject = projects[0]?.path_with_namespace
			if (projects.length === 0) {
				listError = 'The token can see no project with at least the Developer role'
			}
		} catch (e) {
			listError = apiErrorMessage(e)
			projects = []
			selectedProject = undefined
		} finally {
			loading = false
		}
	}
</script>

<div class="flex flex-col gap-4">
	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">GitLab instance</span>
		<TextInput bind:value={baseUrl} />
	</label>
	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">Project access token</span>
		<span class="text-xs text-secondary">
			Create it on the project you are syncing, with the <code>api</code> scope and the
			<code>Maintainer</code> role.
		</span>
		<TextInput bind:value={token} inputProps={{ type: 'password', autocomplete: 'off' }} />
		<span class="text-2xs text-hint">
			Windmill stores it and renews it before it expires. Use one token per project.
		</span>
	</label>
	<div class="flex">
		<Button
			variant="default"
			unifiedSize="sm"
			disabled={!token.trim() || !baseUrl.trim() || loading}
			startIcon={{ icon: loading ? Loader2 : GitBranch, classes: loading ? 'animate-spin' : '' }}
			onClick={listProjects}
		>
			List projects
		</Button>
	</div>
	{#if listError}
		<Alert type="error" title="Could not list projects" size="xs">{listError}</Alert>
	{/if}
	{#if staleListing}
		<span class="text-xs text-secondary">
			The token changed. List the projects again to choose one it can reach.
		</span>
	{:else if projects.length > 0}
		<label class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Project</span>
			<Select
				items={projects.map((p) => ({
					label: p.path_with_namespace,
					value: p.path_with_namespace
				}))}
				bind:value={selectedProject}
				clearable={false}
			/>
		</label>
	{/if}
</div>
