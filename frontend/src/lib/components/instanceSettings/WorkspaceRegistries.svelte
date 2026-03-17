<script lang="ts">
	import { WORKSPACE_REGISTRY_SETTINGS } from '../instanceSettings'
	import { Button } from '../common'
	import SettingCard from './SettingCard.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '../Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import type { Writable } from 'svelte/store'
	import { ChevronDown, ChevronRight, Plus, Trash2 } from 'lucide-svelte'
	import Password from '../Password.svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { untrack } from 'svelte'

	interface Props {
		values: Writable<Record<string, any>>
		loading: boolean
	}

	let { values, loading }: Props = $props()

	let workspaces: { id: string; name: string }[] = $state([])
	let expandedWorkspaces = new SvelteSet<string>()
	let showAddWorkspace = $state(false)
	let newWorkspaceId = $state('')

	const registries = $derived(
		($values['workspace_registries'] ?? {}) as Record<string, Record<string, any>>
	)
	const workspaceIds = $derived(Object.keys(registries).sort())

	async function loadWorkspaces() {
		try {
			const result = await WorkspaceService.listWorkspacesAsSuperAdmin({ perPage: 1000 })
			workspaces = result.map((w) => ({ id: w.id, name: w.name }))
		} catch (e) {
			console.error('Failed to load workspaces', e)
		}
	}

	$effect(() => {
		if (!loading) {
			untrack(() => loadWorkspaces())
		}
	})

	function addWorkspace(wsId: string) {
		if (!wsId || registries[wsId]) {
			sendUserToast('Workspace already has overrides or invalid ID', true)
			return
		}
		if (!$values['workspace_registries']) {
			$values['workspace_registries'] = {}
		}
		$values['workspace_registries'] = { ...$values['workspace_registries'], [wsId]: {} }
		expandedWorkspaces.add(wsId)
		showAddWorkspace = false
		newWorkspaceId = ''
	}

	function removeWorkspace(wsId: string) {
		if ($values['workspace_registries']) {
			const copy = { ...$values['workspace_registries'] }
			delete copy[wsId]
			$values['workspace_registries'] = Object.keys(copy).length > 0 ? copy : undefined
			expandedWorkspaces.delete(wsId)
		}
	}

	function toggleExpand(wsId: string) {
		if (expandedWorkspaces.has(wsId)) {
			expandedWorkspaces.delete(wsId)
		} else {
			expandedWorkspaces.add(wsId)
		}
	}

	function updateSetting(wsId: string, key: string, value: any, isBool = false) {
		if (!$values['workspace_registries']) {
			$values['workspace_registries'] = {}
		}
		if (!$values['workspace_registries'][wsId]) {
			$values['workspace_registries'][wsId] = {}
		}
		const ws = { ...$values['workspace_registries'][wsId] }
		if (!isBool && (value === undefined || value === null || value === '')) {
			delete ws[key]
		} else if (isBool && (value === undefined || value === null)) {
			delete ws[key]
		} else {
			ws[key] = value
		}
		$values['workspace_registries'] = {
			...$values['workspace_registries'],
			[wsId]: ws
		}
	}

	const availableWorkspaces = $derived(workspaces.filter((w) => !registries[w.id]))
</script>

<SettingCard
	label="Workspace-Specific Registry Overrides"
	ee_only=""
	description="Override global registry settings for specific workspaces. Workspace overrides fully replace the global value for each configured key."
>
	{#if workspaceIds.length === 0}
		<p class="text-secondary text-xs">No workspace overrides configured.</p>
	{/if}

	{#each workspaceIds as wsId (wsId)}
		{@const wsName = workspaces.find((w) => w.id === wsId)?.name}
		{@const isExpanded = expandedWorkspaces.has(wsId)}
		{@const wsSettings = registries[wsId] ?? {}}
		<div class="border border-border rounded-md mb-2">
			<button
				class="w-full flex items-center justify-between p-3 hover:bg-surface-hover text-left"
				onclick={() => toggleExpand(wsId)}
			>
				<div class="flex items-center gap-2">
					{#if isExpanded}
						<ChevronDown size={16} />
					{:else}
						<ChevronRight size={16} />
					{/if}
					<span class="font-semibold text-sm text-primary">{wsId}</span>
					{#if wsName}
						<span class="text-secondary text-xs">({wsName})</span>
					{/if}
					{#if Object.keys(wsSettings).length > 0}
						<span class="text-tertiary text-xs"
							>{Object.keys(wsSettings).length} override{Object.keys(wsSettings).length !== 1
								? 's'
								: ''}</span
						>
					{/if}
				</div>
				<Button
					startIcon={{ icon: Trash2 }}
					iconOnly
					variant="subtle"
					unifiedSize="sm"
					onclick={(e) => {
						e.stopPropagation()
						removeWorkspace(wsId)
					}}
				/>
			</button>

			{#if isExpanded}
				<div class="px-3 pb-3 flex flex-col gap-3 border-t border-border pt-3">
					{#each WORKSPACE_REGISTRY_SETTINGS as setting (setting.key)}
						{@const currentValue = wsSettings[setting.key]}
						<div class="flex flex-col gap-1">
							<span class="text-xs font-medium text-emphasis">{setting.label}</span>
							{#if setting.fieldType === 'boolean'}
								<Toggle
									bind:checked={
										() => currentValue ?? false, (v) => updateSetting(wsId, setting.key, v, true)
									}
								/>
							{:else if setting.fieldType === 'codearea'}
								<SimpleEditor
									lang={setting.codeAreaLang ?? 'text'}
									bind:code={() => currentValue ?? '', (v) => updateSetting(wsId, setting.key, v)}
									autoHeight
									fixedOverflowWidgets={false}
								/>
							{:else if setting.fieldType === 'password'}
								<Password
									bind:password={
										() => currentValue ?? '', (v) => updateSetting(wsId, setting.key, v)
									}
									placeholder={setting.placeholder}
								/>
							{:else}
								<input
									type="text"
									class="input-base text-sm"
									value={currentValue ?? ''}
									placeholder={setting.placeholder}
									oninput={(e) => updateSetting(wsId, setting.key, e.currentTarget.value)}
								/>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/each}

	{#if showAddWorkspace}
		<div class="flex items-center gap-2 mt-2">
			{#if availableWorkspaces.length > 0}
				<select class="input-base text-sm" bind:value={newWorkspaceId}>
					<option value="">Select a workspace...</option>
					{#each availableWorkspaces as ws (ws.id)}
						<option value={ws.id}>{ws.id} ({ws.name})</option>
					{/each}
				</select>
			{:else}
				<input
					type="text"
					class="input-base text-sm"
					placeholder="Workspace ID"
					bind:value={newWorkspaceId}
				/>
			{/if}
			<Button
				variant="default"
				unifiedSize="sm"
				disabled={!newWorkspaceId}
				onclick={() => addWorkspace(newWorkspaceId)}
			>
				Add
			</Button>
			<Button
				variant="subtle"
				unifiedSize="sm"
				onclick={() => {
					showAddWorkspace = false
					newWorkspaceId = ''
				}}
			>
				Cancel
			</Button>
		</div>
	{:else}
		<Button
			startIcon={{ icon: Plus }}
			variant="default"
			unifiedSize="sm"
			onclick={() => (showAddWorkspace = true)}
			class="mt-2"
		>
			Add workspace override
		</Button>
	{/if}
</SettingCard>
