<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Filter, Eye, Loader2, CheckCircle2, XCircle, Check, RefreshCw } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import FilterList from './FilterList.svelte'
	import { Tabs, Tab } from '$lib/components/common'

	type ObjectType =
		| 'script'
		| 'flow'
		| 'app'
		| 'folder'
		| 'resource'
		| 'variable'
		| 'secret'
		| 'resourcetype'
		| 'schedule'
		| 'user'
		| 'group'
		| 'trigger'
		| 'settings'
		| 'key'

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

	type PreviewResult = {
		diff?: { [key: string]: { from: any; to: any } }
		hasChanges?: boolean
		isInitialSetup?: boolean
		message?: string
		local?: {
			include_path: string[]
			exclude_path: string[]
			extra_include_path: string[]
			include_type: ObjectType[]
		}
		backend?: {
			include_path: string[]
			exclude_path: string[]
			extra_include_path: string[]
			include_type: ObjectType[]
		}
	}

	let {
		git_repo_resource_path = $bindable(''),
		include_path = $bindable(['f/**']),
		include_type = $bindable(['script', 'flow', 'app', 'folder'] as ObjectType[]),
		exclude_types_override = $bindable([] as ObjectType[]),
		isLegacyRepo = false,
		excludes = $bindable([] as string[]),
		extraIncludes = $bindable([] as string[]),
		isInitialSetup = false,
		requiresMigration = false,
		onSave = (settings: any) => {}
	} = $props()

	// Component state
	let collapsed = $state(false)

	// Determine if component should be editable or read-only
	const isEditable = $derived(isInitialSetup || requiresMigration)

	// Pull/Preview state
	let previewResult = $state<PreviewResult | null>(null)
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isPreviewLoading = $state(false)
	let previewError = $state('')
	let previewSettingsSnapshot = $state<string | null>(null)

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
		const typeMap: Record<keyof GitSyncTypeMap, ObjectType> = {
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

	// Simple JSON-based UI state helper
	function getUIState() {
		return {
			include_path,
			exclude_path: excludes,
			extra_include_path: extraIncludes,
			include_type
		}
	}

	// Apply settings from backend format (used by both local git repo and backend settings)
	function fromBackendFormat(settings: {
		include_path: string[]
		exclude_path: string[]
		extra_include_path: string[]
		include_type: ObjectType[]
	}) {
		include_path = settings.include_path || []
		excludes = settings.exclude_path || []
		extraIncludes = settings.extra_include_path || []
		include_type = settings.include_type || []
	}


	// Sync changes from git repository
	async function previewPullFromGitRepo() {
		isPreviewLoading = true
		previewError = ''
		previewResult = null
		previewJobId = null
		previewJobStatus = undefined

		// Take a snapshot of current settings
		previewSettingsSnapshot = JSON.stringify({
			include_path,
			excludes,
			extraIncludes,
			include_type
		})

		try {
			const workspace = $workspaceStore
			if (!workspace) return

			// Always pass UI state as JSON - the backend now handles this uniformly
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: git_repo_resource_path,
				only_wmill_yaml: true,
				dry_run: true,
				pull: true,
				settings_json: JSON.stringify(getUIState())
			}

			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: payloadObj,
				skipPreprocessor: true
			})

			previewJobId = jobId
			previewJobStatus = 'running'
			let jobSuccess = false
			let result: PreviewResult = {}

			await (
				await import('$lib/utils')
			).tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({ workspace, id: jobId })
					jobSuccess = !!testResult.success
					if (jobSuccess) {
						const jobResult = await JobService.getCompletedJobResult({ workspace, id: jobId })
						result = jobResult as PreviewResult
					}
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: { reason: 'Preview job timed out after 5s' }
						})
					} catch (err) {}
				},
				interval: 500,
				timeout: 10000
			})

			previewJobStatus = jobSuccess ? 'success' : 'failure'
			if (jobSuccess) {
				previewResult = result
			} else {
				previewError = 'Preview failed'
			}
		} catch (e) {
			previewJobStatus = 'failure'
			previewError = e?.message || 'Preview failed'
			previewResult = null
		} finally {
			isPreviewLoading = false
		}
	}

	// Apply pulled settings from git repository
	async function applyPulledSettings() {
		if (previewResult?.local) {
			try {
				fromBackendFormat(previewResult.local)
				if (isEditable) {
					// For editable mode, trigger save callback with new settings
					onSave({
						include_path,
						exclude_path: excludes,
						extra_include_path: extraIncludes,
						include_type
					})
				}
				sendUserToast('Changes applied from git repository')

				// Clear the preview state after applying settings
				previewResult = null
				previewJobId = null
				previewJobStatus = undefined
				previewError = ''
			} catch (e) {
				previewError = 'Failed to apply pulled settings: ' + e.message
			}
		}
	}



	// Reset preview state when settings change (making preview stale)
	$effect(() => {
		// Track all the settings that affect the preview
		const currentSettings = JSON.stringify({
			include_path,
			excludes,
			extraIncludes,
			include_type
		})

		// If we have an existing preview result and settings have changed from snapshot, clear it
		if (
			previewResult !== null &&
			previewSettingsSnapshot !== null &&
			currentSettings !== previewSettingsSnapshot
		) {
			previewResult = null
			previewJobId = null
			previewJobStatus = undefined
			previewError = ''
			previewSettingsSnapshot = null
		}
	})
</script>

<div class="rounded-lg shadow-sm border p-0 w-full">
	<!-- Card Header -->
	<div class="flex items-center justify-between min-h-10 px-4 py-1 border-b">
		<div class="flex items-center gap-2">
			<Filter size={18} class="text-primary" />
			<span class="font-semibold text-sm">Git Sync filter settings</span>
		</div>
		<div class="flex items-center gap-2">
			{#if !isEditable}
				<Button
					size="xs"
					color="light"
					on:click={previewPullFromGitRepo}
					disabled={isPreviewLoading}
					startIcon={{
						icon: isPreviewLoading ? Loader2 : RefreshCw,
						classes: isPreviewLoading ? 'animate-spin' : ''
					}}
				>
					{isPreviewLoading ? 'Syncing...' : 'Sync with repo'}
				</Button>
			{/if}
			<button
				class="text-gray-500 hover:text-primary focus:outline-none"
				onclick={() => (collapsed = !collapsed)}
				aria-label="Toggle collapse"
			>
				{#if collapsed}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-5 w-5"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M19 9l-7 7-7-7"
						/>
					</svg>
				{:else}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-5 w-5"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M5 15l7-7 7 7"
						/>
					</svg>
				{/if}
			</button>
		</div>
	</div>
	{#if !collapsed}
		{#if isEditable}
			<!-- Editable mode -->
			<div class="px-4 py-2">
				<div class="grid grid-cols-1 md:grid-cols-2 md:gap-32">
					<div class="flex flex-col gap-2">
						<Tabs bind:selected={filtersTab}>
							<Tab value="includes">Includes</Tab>
							<Tab value="excludes">Excludes</Tab>
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
				<div class="text-xs text-tertiary mb-2">
					{isInitialSetup ? 'Configure initial sync settings' : 'Review migration settings'}
				</div>
			</div>
		{:else}
			<!-- Read-only view -->
			<div class="px-4 py-2">
				<div class="grid grid-cols-1 md:grid-cols-2 md:gap-8">
					<div class="flex flex-col gap-3">
						<div>
							<h4 class="font-semibold text-sm mb-1">Include Paths</h4>
							{#if include_path.length > 0}
								<div class="flex flex-wrap gap-1 text-xs">
									{#each include_path as path}
										<span class="bg-gray-100 text-gray-800 rounded-full px-2 py-1">{path}</span>
									{/each}
								</div>
							{:else}
								<div class="text-tertiary text-xs">No include paths configured</div>
							{/if}
						</div>

						<div>
							<h4 class="font-semibold text-sm mb-1">Exclude Paths</h4>
							{#if excludes.length > 0}
								<div class="flex flex-wrap gap-1 text-xs">
									{#each excludes as path}
										<span class="bg-red-100 text-red-800 rounded-full px-2 py-1">{path}</span>
									{/each}
								</div>
							{:else}
								<div class="text-tertiary text-xs">No exclude paths configured</div>
							{/if}
						</div>
					</div>

					<div class="flex flex-col gap-2">
						<h4 class="font-semibold text-sm">Included Types</h4>
						<div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
							{#each Object.entries(typeToggles) as [key, enabled]}
								<div class="flex items-center gap-1">
									<div class={enabled ? 'text-green-600' : 'text-gray-400'}>
										{enabled ? '✓' : '✗'}
									</div>
									<span class={enabled ? 'text-primary' : 'text-tertiary'}>
										{capitalize(key)}
									</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			</div>

			{#if previewResult?.hasChanges && (previewResult?.isInitialSetup || (previewResult?.diff && Object.keys(previewResult.diff).length > 0))}
				<div class="mt-4 flex flex-col gap-2 p-2 border-t">
					<div class="flex gap-2 items-center">
						<Button
							size="sm"
							on:click={applyPulledSettings}
							disabled={isPreviewLoading}
							color="dark"
							startIcon={{ icon: Check }}
						>
							Apply Changes
						</Button>
					</div>
				</div>
			{/if}
		{/if}

		<!-- Preview/job status sections (shown for both editable and read-only modes) -->
		{#if previewError}
			<div class="text-xs text-red-600 mt-2 px-4">{previewError}</div>
		{/if}
		{#if previewJobId}
			<div class="flex items-center gap-2 text-xs text-tertiary mt-1 px-4">
				{#if previewJobStatus === 'running'}
					<Loader2 class="animate-spin" size={14} />
				{:else if previewJobStatus === 'success'}
					<CheckCircle2 size={14} class="text-green-600" />
				{:else if previewJobStatus === 'failure'}
					<XCircle size={14} class="text-red-700" />
				{/if}
				Pull job:
				<a
					target="_blank"
					class="underline"
					href={`/run/${previewJobId}?workspace=${$workspaceStore}`}>{previewJobId}</a
				>
			</div>
		{/if}
		{#if previewResult}
			<div class="border rounded p-2 text-xs max-h-40 overflow-y-auto bg-surface-secondary mt-2 mx-4">
				<div class="font-semibold text-[11px] mb-1 text-tertiary">Preview of changes:</div>
				{#if previewResult.isInitialSetup}
					<div class="mt-2 text-green-600">
						{previewResult.message || 'wmill.yaml will be created with repository settings'}
					</div>
				{:else if previewResult.hasChanges && previewResult.diff && Object.keys(previewResult.diff).length > 0}
					<div class="mt-2 space-y-1">
						{#each Object.entries(previewResult.diff) as [field, change]}
							<div class="flex items-start gap-2 text-2xs">
								<span class="font-mono text-tertiary min-w-0 flex-shrink-0">{field}:</span>
								<div class="min-w-0 flex-1">
									{#if Array.isArray(change.from) || Array.isArray(change.to)}
										<div class="space-y-0.5">
											<div class="text-red-600">- {JSON.stringify(change.from)}</div>
											<div class="text-green-600">+ {JSON.stringify(change.to)}</div>
										</div>
									{:else}
										<span class="text-red-600">{JSON.stringify(change.from)}</span>
										<span class="text-tertiary"> → </span>
										<span class="text-green-600">{JSON.stringify(change.to)}</span>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="mt-2 text-tertiary">No changes found! The file is up to date.</div>
				{/if}
			</div>
		{/if}
	{/if}
</div>
