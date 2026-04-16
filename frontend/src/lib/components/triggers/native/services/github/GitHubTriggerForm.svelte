<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		serviceConfig: Record<string, any>
		errors: Record<string, string>
		disabled?: boolean
		externalData?: any
		loading?: boolean
		path?: string
		isFlow?: boolean
		token?: string
		triggerTokens?: any
		scopes?: any[]
	}

	let {
		serviceConfig = $bindable(),
		errors = $bindable(),
		loading = $bindable(false),
		disabled = false,
		externalData
	}: Props = $props()

	const GITHUB_EVENTS = [
		{ label: 'Push', value: 'push' },
		{ label: 'Pull Request', value: 'pull_request' },
		{ label: 'Issues', value: 'issues' },
		{ label: 'Issue Comment', value: 'issue_comment' },
		{ label: 'Create (branch/tag)', value: 'create' },
		{ label: 'Delete (branch/tag)', value: 'delete' },
		{ label: 'Release', value: 'release' },
		{ label: 'Workflow Run', value: 'workflow_run' },
		{ label: 'Pull Request Review', value: 'pull_request_review' },
		{ label: 'Fork', value: 'fork' },
		{ label: 'Star', value: 'star' },
		{ label: 'Deployment', value: 'deployment' },
		{ label: 'Deployment Status', value: 'deployment_status' }
	]

	let repos = $state<Array<{ full_name: string; owner: string; name: string; private: boolean }>>(
		[]
	)
	let loadingRepos = $state(false)

	let selectedRepo = $state<string>(
		externalData?.owner && externalData?.repo
			? `${externalData.owner}/${externalData.repo}`
			: serviceConfig.owner && serviceConfig.repo
				? `${serviceConfig.owner}/${serviceConfig.repo}`
				: ''
	)
	let selectedEvents = $state<string[]>(externalData?.events ?? serviceConfig.events ?? ['push'])

	async function loadRepos() {
		if (!$workspaceStore) return
		loadingRepos = true
		try {
			const response = await fetch(`/api/w/${$workspaceStore}/native_triggers/github/repos`, {
				credentials: 'include'
			})
			if (response.ok) {
				repos = await response.json()
			} else {
				sendUserToast('Failed to load repositories', true)
			}
		} catch (err: any) {
			sendUserToast(`Failed to load repositories: ${err.message}`, true)
		} finally {
			loadingRepos = false
		}
	}

	loadRepos()

	$effect(() => {
		loading = loadingRepos
	})

	// Sync form state back to serviceConfig
	$effect(() => {
		const parts = selectedRepo.split('/')
		const owner = parts[0] ?? ''
		const repo = parts.slice(1).join('/') ?? ''
		serviceConfig = {
			owner,
			repo,
			events: selectedEvents
		}
	})

	let repoItems = $derived(
		repos.map((r) => ({
			label: `${r.full_name}${r.private ? ' (private)' : ''}`,
			value: r.full_name
		}))
	)

	export function validate(): Record<string, string> {
		let serviceErrors: Record<string, string> = {}

		if (!selectedRepo?.trim()) {
			serviceErrors.repo = 'Repository is required'
		}
		if (!selectedEvents || selectedEvents.length === 0) {
			serviceErrors.events = 'At least one event is required'
		}

		return serviceErrors
	}
</script>

<Section label="GitHub Trigger Configuration">
	<div class="flex flex-col gap-4">
		<div class="flex flex-col gap-1">
			<p class="block text-xs font-semibold text-primary">Repository</p>
			{#if loadingRepos}
				<div class="flex items-center gap-2 text-secondary text-xs">
					<Loader2 class="animate-spin" size={14} />
					Loading repositories...
				</div>
			{:else}
				<Select
					items={repoItems}
					bind:value={selectedRepo}
					placeholder="Select a repository"
					{disabled}
				/>
			{/if}
			{#if errors.repo}
				<p class="text-red-500 text-xs">{errors.repo}</p>
			{/if}
		</div>

		<div class="flex flex-col gap-1">
			<p class="block text-xs font-semibold text-primary">Events</p>
			<MultiSelect
				items={GITHUB_EVENTS}
				bind:value={selectedEvents}
				placeholder="Select events"
				{disabled}
				reorderable={false}
			/>
			{#if errors.events}
				<p class="text-red-500 text-xs">{errors.events}</p>
			{/if}
		</div>
	</div>
</Section>
