<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Debounced, resource } from 'runed'

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

	let selectedRepo = $state<string>(
		externalData?.owner && externalData?.repo
			? `${externalData.owner}/${externalData.repo}`
			: serviceConfig.owner && serviceConfig.repo
				? `${serviceConfig.owner}/${serviceConfig.repo}`
				: ''
	)
	let selectedEvents = $state<string[]>(externalData?.events ?? serviceConfig.events ?? ['push'])

	let filterText = $state('')
	const debouncedQuery = new Debounced(() => filterText.trim(), 300)

	type RepoEntry = { full_name: string; owner: string; name: string; private: boolean }

	const reposResource = resource(
		[() => debouncedQuery.current, () => $workspaceStore],
		async ([query, workspace]) => {
			if (!workspace) return [] as RepoEntry[]
			const url = query
				? `/api/w/${workspace}/native_triggers/github/repos?q=${encodeURIComponent(query)}`
				: `/api/w/${workspace}/native_triggers/github/repos`
			const response = await fetch(url, { credentials: 'include' })
			if (!response.ok) {
				sendUserToast('Failed to load repositories', true)
				return [] as RepoEntry[]
			}
			return (await response.json()) as RepoEntry[]
		}
	)

	$effect(() => {
		loading = reposResource.loading
	})

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

	let repoItems = $derived.by(() => {
		const repos = reposResource.current ?? []
		const items = repos.map((r) => ({
			label: `${r.full_name}${r.private ? ' (private)' : ''}`,
			value: r.full_name
		}))
		// If the currently-selected repo isn't in the fetched list (e.g. editing an
		// existing trigger), inject it so the Select can display its label.
		if (selectedRepo && !items.some((i) => i.value === selectedRepo)) {
			items.unshift({ label: selectedRepo, value: selectedRepo })
		}
		return items
	})

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
			<Select
				items={repoItems}
				bind:value={selectedRepo}
				bind:filterText
				placeholder="Search repositories..."
				loading={reposResource.loading}
				{disabled}
			/>
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
