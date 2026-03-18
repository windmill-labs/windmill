<script lang="ts">
	import { WorkspaceService, WorkspaceDependenciesService } from '$lib/gen'
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import Select from './select/Select.svelte'
	import Badge from './common/badge/Badge.svelte'
	import { sendUserToast } from '$lib/toast'
	import { superadmin, devopsRole } from '$lib/stores'

	interface RunnerGroup {
		workspace_id: string
		dep_name: string
		language: string
	}

	interface Props {
		/** Runner groups in format "workspace_id:dep_name:language" */
		selectedGroups: string[]
		disabled?: boolean
		onchange?: (groups: string[]) => void
	}

	let { selectedGroups = [], disabled = false, onchange }: Props = $props()

	let workspaces: { label: string; value: string }[] = $state([])
	let dependencies: { label: string; value: string; language: string }[] = $state([])
	let loadingDeps = $state(false)

	let selectedWorkspace: string | undefined = $state(undefined)
	let selectedDep: string | undefined = $state(undefined)
	let showAddForm = $state(false)

	let parsedGroups: RunnerGroup[] = $derived(
		selectedGroups.map((g) => {
			const parts = g.split(':')
			return {
				workspace_id: parts[0] ?? '',
				dep_name: parts[1] ?? '',
				language: parts[2] ?? ''
			}
		})
	)

	async function loadWorkspaces() {
		try {
			const ws = await WorkspaceService.listWorkspacesAsSuperAdmin()
			workspaces = ws.map((w) => ({ label: `${w.name} (${w.id})`, value: w.id }))
		} catch (e) {
			sendUserToast(`Failed to load workspaces: ${e}`, true)
		}
	}

	async function loadDependencies(workspace: string) {
		loadingDeps = true
		dependencies = []
		try {
			const deps = await WorkspaceDependenciesService.listWorkspaceDependencies({
				workspace
			})
			dependencies = (deps ?? [])
				.filter((d) => !d.archived)
				.filter((d) => ['bun', 'python3', 'deno'].includes(d.language ?? ''))
				.map((d) => ({
					label: `${d.name} (${d.language})`,
					value: `${d.name}:${d.language}`,
					language: d.language ?? ''
				}))
		} catch (e) {
			sendUserToast(`Failed to load dependencies: ${e}`, true)
		} finally {
			loadingDeps = false
		}
	}

	function addGroup() {
		if (!selectedWorkspace || !selectedDep) return
		const [depName, language] = selectedDep.split(':')
		const tag = `${selectedWorkspace}:${depName}:${language}`
		if (selectedGroups.includes(tag)) {
			sendUserToast('This runner group is already added', true)
			return
		}
		const newGroups = [...selectedGroups, tag]
		onchange?.(newGroups)
		selectedDep = undefined
		showAddForm = false
	}

	function removeGroup(index: number) {
		const newGroups = selectedGroups.filter((_, i) => i !== index)
		onchange?.(newGroups)
	}

	function langIcon(lang: string): string {
		switch (lang) {
			case 'python3':
				return 'Py'
			case 'bun':
			case 'bunnative':
				return 'Bun'
			case 'deno':
				return 'Deno'
			default:
				return lang
		}
	}

	$effect(() => {
		if (($superadmin || $devopsRole) && workspaces.length === 0) {
			loadWorkspaces()
		}
	})

	$effect(() => {
		if (selectedWorkspace) {
			loadDependencies(selectedWorkspace)
		}
	})
</script>

<div class="flex flex-col gap-3">
	{#if parsedGroups.length > 0}
		<div class="flex flex-col gap-2">
			{#each parsedGroups as group, i (selectedGroups[i])}
				<div
					class="flex items-center justify-between rounded-md border border-surface-secondary p-3"
				>
					<div class="flex items-center gap-2">
						<Badge color="indigo" small>{langIcon(group.language)}</Badge>
						<span class="text-sm font-medium">{group.dep_name}</span>
						<span class="text-xs text-tertiary">({group.workspace_id})</span>
					</div>
					{#if !disabled}
						<Button
							variant="subtle"
							iconOnly
							startIcon={{ icon: X }}
							unifiedSize="sm"
							onclick={() => removeGroup(i)}
						/>
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		<p class="text-xs text-tertiary">No runner groups configured.</p>
	{/if}

	{#if !disabled}
		{#if showAddForm}
			<div class="flex flex-col gap-2 rounded-md border border-surface-secondary p-3">
				<div class="flex items-center justify-between">
					<span class="text-sm font-medium">Add Runner Group</span>
					<Button
						variant="subtle"
						iconOnly
						startIcon={{ icon: X }}
						unifiedSize="sm"
						onclick={() => {
							showAddForm = false
						}}
					/>
				</div>
				<Select
					items={workspaces}
					bind:value={selectedWorkspace}
					placeholder="Select workspace"
					size="sm"
				/>
				{#if selectedWorkspace}
					<Select
						items={dependencies}
						bind:value={selectedDep}
						placeholder={loadingDeps ? 'Loading dependencies...' : 'Select workspace dependency'}
						disabled={loadingDeps}
						size="sm"
					/>
				{/if}
				<div class="flex justify-end gap-2">
					<Button
						variant="subtle"
						unifiedSize="sm"
						onclick={() => {
							showAddForm = false
						}}>Cancel</Button
					>
					<Button
						variant="accent"
						unifiedSize="sm"
						disabled={!selectedWorkspace || !selectedDep}
						onclick={addGroup}>Add</Button
					>
				</div>
			</div>
		{:else}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: Plus }}
				onclick={() => {
					showAddForm = true
				}}
			>
				Add Runner Group
			</Button>
		{/if}
	{/if}

	<p class="text-xs text-tertiary">
		One long-lived process per runner group. Scripts sharing the same workspace dependency execute
		in the same process.
	</p>
</div>
