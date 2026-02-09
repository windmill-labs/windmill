<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Filter, ChevronDown } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import FilterList from './FilterList.svelte'
	import { Tabs, Tab, Button, Section } from '$lib/components/common'
	import type { GitSyncObjectType } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'

	type GitSyncTypeMap = {
		scripts: boolean
		flows: boolean
		apps: boolean
		folders: boolean
		resourceTypes: boolean
		resources: boolean
		variables: boolean
		secrets: boolean
		schedules: boolean
		users: boolean
		groups: boolean
		triggers: boolean
		settings: boolean
		key: boolean
	}

	let {
		git_repo_resource_path = $bindable(''),
		include_path = $bindable(['f/**']),
		include_type = $bindable(['script', 'flow', 'app', 'folder'] as GitSyncObjectType[]),
		exclude_types_override = $bindable([] as GitSyncObjectType[]),
		isLegacyRepo = false,
		excludes = $bindable([] as string[]),
		extraIncludes = $bindable([] as string[]),
		isInitialSetup = false,
		requiresMigration = false,
		actions = undefined,
		useIndividualBranch = false
	} = $props()

	// Component state
	let collapsed = $state(false)

	// Determine if component should be editable or read-only
	const isEditable = $derived(isInitialSetup || requiresMigration)

	// Compute effective include types (include_type minus exclude_types_override for legacy repos only)
	const effectiveIncludeTypes = $derived(
		isLegacyRepo
			? include_type.filter((type) => !exclude_types_override.includes(type))
			: include_type
	)

	// Compute type toggles from effective include types
	const typeToggles = $derived({
		scripts: effectiveIncludeTypes.includes('script'),
		flows: effectiveIncludeTypes.includes('flow'),
		apps: effectiveIncludeTypes.includes('app'),
		folders: effectiveIncludeTypes.includes('folder'),
		resourceTypes: effectiveIncludeTypes.includes('resourcetype'),
		resources: effectiveIncludeTypes.includes('resource'),
		variables: effectiveIncludeTypes.includes('variable'),
		secrets: effectiveIncludeTypes.includes('secret'),
		schedules: effectiveIncludeTypes.includes('schedule'),
		users: effectiveIncludeTypes.includes('user'),
		groups: effectiveIncludeTypes.includes('group'),
		triggers: effectiveIncludeTypes.includes('trigger'),
		settings: effectiveIncludeTypes.includes('settings'),
		key: effectiveIncludeTypes.includes('key')
	})

	// Tab selection for filter kinds
	let filtersTab = $state<'includes' | 'excludes'>('includes')

	function updateIncludeType(key: keyof GitSyncTypeMap, value: boolean) {
		const newTypes = new Set(include_type)
		const typeMap: Record<keyof GitSyncTypeMap, GitSyncObjectType> = {
			scripts: 'script',
			flows: 'flow',
			apps: 'app',
			folders: 'folder',
			resourceTypes: 'resourcetype',
			resources: 'resource',
			variables: 'variable',
			secrets: 'secret',
			schedules: 'schedule',
			users: 'user',
			groups: 'group',
			triggers: 'trigger',
			settings: 'settings',
			key: 'key'
		}

		if (value) {
			newTypes.add(typeMap[key])
		} else {
			newTypes.delete(typeMap[key])
			if (key === 'variables') {
				newTypes.delete('secret')
			}
		}

		include_type = Array.from(newTypes)
	}

	function capitalize(str: string) {
		return str.charAt(0).toUpperCase() + str.slice(1)
	}
</script>

<div class="rounded-lg shadow-sm border p-0 w-full">
	<!-- Card Header -->
	<div class="flex items-center justify-between min-h-10 px-4 py-1 border-b">
		<div class="flex items-center gap-2">
			<Filter size={14} class="text-primary" />
			<span class="font-semibold text-xs text-emphasis">Git Sync filter settings</span>
			{#if isLegacyRepo}
				<Tooltip>
					This repository uses legacy configuration format and inherits settings from
					workspace-level defaults. Excluded types are filtered out from inherited types. Save to
					migrate to the new format.
				</Tooltip>
			{:else if !isEditable}
				<Tooltip documentationLink="https://www.windmill.dev/docs/advanced/cli/sync#wmillyaml">
					These settings are controlled by the wmill.yaml file in your git repository. Click "Pull
					from repo" to check for settings drift and pull settings from repo.
				</Tooltip>
			{/if}
		</div>
		<Button
			unifiedSize="sm"
			variant="subtle"
			startIcon={{
				icon: ChevronDown,
				classes: twMerge('transition duration-150', collapsed ? '' : 'rotate-180')
			}}
			onClick={() => (collapsed = !collapsed)}
			iconOnly
		/>
	</div>
	{#if !collapsed}
		{#if isEditable}
			<!-- Editable mode -->
			<div class="px-4 py-2" transition:slide={{ duration: 150 }}>
				<div class="grid grid-cols-1 md:grid-cols-2 md:gap-32">
					<div class="flex flex-col gap-2">
						<Tabs bind:selected={filtersTab}>
							<Tab value="includes" label="Includes"></Tab>
							<Tab value="excludes" label="Excludes"></Tab>
						</Tabs>

						{#if filtersTab === 'includes'}
							<FilterList
								title="Include path filters"
								bind:items={include_path}
								placeholder="Add filter (e.g. f/**)"
							>
								{#snippet tooltip()}
									<Tooltip>
										Only scripts, flows and apps with their path matching one of those filters will
										be synced to the Git repositories below. The filters allow '*' and '**'
										characters, with '*' matching any character allowed in paths until the next
										slash (/) and '**' matching anything including slashes. By default everything in
										folders will be synced.
									</Tooltip>
								{/snippet}
							</FilterList>
						{:else if filtersTab === 'excludes'}
							<FilterList
								title="Exclude path filters"
								bind:items={excludes}
								placeholder="Add filter (e.g. f/**)"
							>
								{#snippet tooltip()}
									<Tooltip>
										After the include / extra include checks, if a file matches any of these
										patterns it will be skipped.
									</Tooltip>
								{/snippet}
							</FilterList>
						{/if}
					</div>
					<!-- Type Filters Section (Right) -->
					<div>
						<div class="flex items-center gap-2 mb-3">
							<h4 class="font-semibold text-sm">Type filters</h4>
							<Tooltip>
								On top of the filter path above, you can include only certain type of object to be
								synced with the Git repository. By default everything is synced.
							</Tooltip>
						</div>
						<div class="grid grid-cols-2 gap-x-4 gap-y-2">
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.scripts}
									on:change={(e) => updateIncludeType('scripts', e.detail)}
									options={{ right: capitalize('scripts') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.flows}
									on:change={(e) => updateIncludeType('flows', e.detail)}
									options={{ right: capitalize('flows') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.apps}
									on:change={(e) => updateIncludeType('apps', e.detail)}
									options={{ right: capitalize('apps') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.folders}
									on:change={(e) => updateIncludeType('folders', e.detail)}
									options={{ right: capitalize('folders') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.resourceTypes}
									on:change={(e) => updateIncludeType('resourceTypes', e.detail)}
									options={{ right: capitalize('resourceTypes') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.resources}
									on:change={(e) => updateIncludeType('resources', e.detail)}
									options={{ right: capitalize('resources') }}
								/>
							</div>
							<div class="col-span-2 flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.variables}
									on:change={(e) => updateIncludeType('variables', e.detail)}
									options={{ right: 'Variables' }}
								/>
								<span class="text-gray-400">-</span>
								<Toggle
									size="xs"
									disabled={!typeToggles.variables}
									checked={typeToggles.secrets}
									on:change={(e) => updateIncludeType('secrets', e.detail)}
									options={{ left: 'Include secrets' }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.schedules}
									on:change={(e) => updateIncludeType('schedules', e.detail)}
									options={{ right: capitalize('schedules') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.users}
									on:change={(e) => updateIncludeType('users', e.detail)}
									options={{ right: capitalize('users') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.groups}
									on:change={(e) => updateIncludeType('groups', e.detail)}
									options={{ right: capitalize('groups') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.triggers}
									on:change={(e) => updateIncludeType('triggers', e.detail)}
									options={{ right: capitalize('triggers') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.settings}
									on:change={(e) => updateIncludeType('settings', e.detail)}
									options={{ right: 'Workspace settings' }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									checked={typeToggles.key}
									on:change={(e) => updateIncludeType('key', e.detail)}
									options={{ right: 'Encryption key' }}
								/>
							</div>
						</div>
					</div>
				</div>
			</div>
			<div class="mt-6 p-2 border-t">
				<div class="text-xs text-primary mb-2">
					{isInitialSetup ? 'Configure initial sync settings' : 'Review migration settings'}
				</div>
			</div>
		{:else}
			<!-- Read-only view -->
			<div class="px-4 py-2">
				<div class="grid grid-cols-1 md:grid-cols-2 md:gap-8">
					<div class="flex flex-col gap-3">
						<div>
							<h4 class="font-semibold text-xs text-emphasis mb-1">Include Paths</h4>
							{#if include_path.length > 0}
								<div class="flex flex-wrap gap-1 text-xs">
									{#each include_path as path}
										<span class="bg-surface-secondary text-primary rounded-full px-2 py-1"
											>{path}</span
										>
									{/each}
								</div>
							{:else}
								<div class="text-primary text-xs">No include paths configured</div>
							{/if}
						</div>

						<div>
							<h4 class="font-semibold text-xs text-emphasis mb-1">Exclude Paths</h4>
							{#if excludes.length > 0}
								<div class="flex flex-wrap gap-1 text-xs">
									{#each excludes as path}
										<span class="bg-red-100 text-red-800 rounded-full px-2 py-1">{path}</span>
									{/each}
								</div>
							{:else}
								<div class="text-primary text-xs">No exclude paths configured</div>
							{/if}
						</div>
					</div>

					<div class="flex flex-col gap-2">
						<h4 class="font-semibold text-xs text-emphasis">Included Types</h4>
						<div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
							{#each Object.entries(typeToggles) as [key, enabled]}
								<div class="flex items-center gap-1">
									<div class={enabled ? 'text-green-600' : 'text-gray-400'}>
										{enabled ? '✓' : '✗'}
									</div>
									<span class={enabled ? 'text-primary' : 'text-primary'}>
										{capitalize(key)}
									</span>
								</div>
							{/each}
						</div>
					</div>
				</div>

				<!-- Actions slot for custom buttons -->
				{#if actions}
					<div class="flex justify-start mt-4">
						{@render actions()}
					</div>
				{/if}

				<!-- CLI Instructions (collapsible) -->
				<div class="border-t pt-2 mt-4">
					<Section label="Update settings with CLI" collapsable={true} collapsed={true}>
						<div class="mt-3 bg-surface-secondary rounded-lg p-3">
							<div class="text-xs text-primary mb-2">
								These filter settings are sourced from the <code
									class="bg-surface px-1 py-0.5 rounded">wmill.yaml</code
								>
								file in your git repository. To modify them, edit the file in your repository, commit
								the changes, and sync using the commands below. Learn more about
								<a
									href="https://www.windmill.dev/docs/advanced/cli/sync#wmillyaml"
									target="_blank"
									rel="noopener noreferrer">the wmill.yaml format</a
								>
							</div>
							<pre
								class="text-xs bg-surface p-3 rounded overflow-x-auto whitespace-pre-wrap break-all">
# Make sure your repo is up to date
git pull

# Edit wmill.yaml file
vim wmill.yaml

# Commit changes
git add wmill.yaml
git commit
git push

# Push changes to workspace or click the pull settings button above{#if useIndividualBranch}
									wmill gitsync-settings push --workspace {$workspaceStore} --repository {git_repo_resource_path} --promotion main{:else}
									wmill gitsync-settings push --workspace {$workspaceStore} --repository {git_repo_resource_path}{/if}</pre
							>
							{#if useIndividualBranch}
								<div class="text-xs text-primary mt-3">
									<div class="font-medium mb-1">Promotion Mode Configuration:</div>
									<div
										>You can add promotion-specific overrides in your <code
											class="bg-surface px-1 py-0.5 rounded">wmill.yaml</code
										> file:</div
									>
									<pre class="text-xs bg-surface p-2 rounded mt-2 overflow-x-auto"
										>gitBranches:
  main:
    promotionOverrides:
      # Add your promotion-specific settings here</pre
									>
								</div>
							{/if}
						</div>
					</Section>
				</div>
			</div>
		{/if}
	{/if}
</div>
