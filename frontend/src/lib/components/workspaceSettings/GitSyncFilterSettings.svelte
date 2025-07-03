<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Filter, Save, Eye, Loader2, CheckCircle2, XCircle, Check } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import yaml from 'js-yaml'
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
	}

	let {
		git_repo_resource_path = $bindable(''),
		include_path = $bindable(['f/**']),
		include_type = $bindable(['script', 'flow', 'app', 'folder'] as ObjectType[]),
		exclude_types_override = $bindable([] as ObjectType[]),
		isLegacyRepo = false,
		yamlText = $bindable(''),
		onSettingsChange = (settings: { yaml: string }) => {},
		excludes = $bindable([] as string[]),
		extraIncludes = $bindable([] as string[])
	} = $props()

	// Component state
	let collapsed = $state(false)
	let editAsYaml = $state(false)
	let yamlError = $state('')
	let isPullMode = $state(false)

	// Preview/Push state
	let previewResult = $state<PreviewResult | null>(null)
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let pushJobId = $state<string | null>(null)
	let pushJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isPreviewLoading = $state(false)
	let isPushing = $state(false)
	let previewError = $state('')

	// Compute effective include types (include_type minus exclude_types_override for legacy repos only)
	const effectiveIncludeTypes = $derived(
		isLegacyRepo
			? include_type.filter(type => !exclude_types_override.includes(type))
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
	let filtersTab = $state<'includes' | 'excludes' | 'extra'>('includes')

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

	// Simplified YAML parsing for manual editing
	function fromYaml(yamlStr: string) {
		yamlError = ''
		try {
			const parsed = yaml.load(yamlStr)
			if (!parsed || typeof parsed !== 'object') {
				throw new Error('Invalid YAML structure')
			}

			const obj: any = parsed
			yamlText = yamlStr

			// Extract includes - reset to default if not present
			if (obj.includes && Array.isArray(obj.includes)) {
				include_path = obj.includes.map((p: any) => {
					if (typeof p !== 'string') {
						throw new Error('includes must contain only strings')
					}
					// Handle quoted strings
					if (/^['"].*['"]$/.test(p)) {
						return p.slice(1, -1).replace(/''/g, "'")
					}
					return p
				})
			} else {
				// Reset to default if includes is not present
				include_path = ['f/**']
			}

			// Build the type set based on the YAML flags
			const newTypes = new Set<ObjectType>()

			// Always include core types (these are fundamental and not controlled by flags)
			newTypes.add('script')
			newTypes.add('flow')
			newTypes.add('app')
			newTypes.add('folder')

			// Handle skip flags (if skipX is false or undefined, include the type)
			if (obj.skipResourceTypes !== true) newTypes.add('resourcetype')
			if (obj.skipResources !== true) newTypes.add('resource')
			if (obj.skipVariables !== true) newTypes.add('variable')
			if (obj.skipSecrets !== true) newTypes.add('secret')

			// Handle include flags (if includeX is true, include the type)
			if (obj.includeSchedules === true) newTypes.add('schedule')
			if (obj.includeTriggers === true) newTypes.add('trigger')
			if (obj.includeUsers === true) newTypes.add('user')
			if (obj.includeGroups === true) newTypes.add('group')
			if (obj.includeSettings === true) newTypes.add('settings')
			if (obj.includeKey === true) newTypes.add('key')

			// Apply business rule: secrets can only be included if variables are included
			// This matches the UI behavior where turning off variables also turns off secrets
			if (!newTypes.has('variable')) {
				newTypes.delete('secret')
			}

			include_type = Array.from(newTypes)

		} catch (e) {
			yamlError = e.message || 'Invalid YAML'
			console.error('Error parsing YAML:', e)
		}
	}

	// Simple YAML generation for manual editing mode
	function generateYamlFromUI() {
		try {
			const validIncludePath = include_path
			const validExcludePath = excludes
			const validExtraInclude = extraIncludes

			// Basic YAML structure - let the CLI handle the proper normalization
			let config: any = {
				defaultTs: 'bun',
				includes: validIncludePath,
				excludes: validExcludePath,
				extraIncludes: validExtraInclude,
				codebases: []
			}

			// Let the CLI handle the optimization of skip/include flags
			// Just convert the UI state directly
			if (!include_type.includes('variable')) config.skipVariables = true
			if (!include_type.includes('resource')) config.skipResources = true
			if (!include_type.includes('secret')) config.skipSecrets = true
			if (!include_type.includes('resourcetype')) config.skipResourceTypes = true

			if (include_type.includes('schedule')) config.includeSchedules = true
			if (include_type.includes('trigger')) config.includeTriggers = true
			if (include_type.includes('user')) config.includeUsers = true
			if (include_type.includes('group')) config.includeGroups = true
			if (include_type.includes('settings')) config.includeSettings = true
			if (include_type.includes('key')) config.includeKey = true

			return yaml.dump(config, {
				indent: 2,
				lineWidth: -1,
				quotingType: '"',
				forceQuotes: false,
				noRefs: true
			})
		} catch (e) {
			console.warn('Failed to generate YAML:', e)
			yamlError = e.message || 'Failed to generate YAML'
			return `defaultTs: bun
includes:
  - f/**
excludes: []
extraIncludes: []
codebases: []`
		}
	}

	function switchToYaml() {
		yamlText = generateYamlFromUI()
		yamlError = ''
		editAsYaml = true
	}

	function switchToUI() {
		fromYaml(yamlText)
		if (!yamlError) {
			editAsYaml = false
		}
	}

	// Simplified preview function - always uses JSON approach
	async function previewFiltersToGitRepo() {
		isPreviewLoading = true
		previewError = ''
		previewResult = null
		previewJobId = null
		previewJobStatus = undefined

		try {
			const workspace = $workspaceStore
			if (!workspace) return

			// Always pass UI state as JSON - the backend now handles this uniformly
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: git_repo_resource_path,
				only_wmill_yaml: true,
				dry_run: true,
				pull: isPullMode,
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

			await (await import('$lib/utils')).tryEvery({
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

	// Simplified push function - always uses JSON approach
	async function pushFiltersToGitRepo() {
		if (isPullMode) {
			// In pull mode, apply the pulled YAML to current settings
			if (previewResult?.yaml) {
				try {
					fromYaml(previewResult.yaml);
					yamlText = previewResult.yaml;
					onSettingsChange({ yaml: previewResult.yaml });
					sendUserToast('Changes applied - remember to save repository settings to persist changes');

					// Clear the preview state after applying settings
					previewResult = null;
					previewJobId = null;
					previewJobStatus = undefined;
					previewError = '';
				} catch (e) {
					previewError = 'Failed to apply pulled YAML: ' + e.message;
				}
			}
			return;
		}

		// Push mode - send current UI state as JSON
		isPushing = true
		pushJobId = null
		pushJobStatus = undefined

		try {
			const workspace = $workspaceStore
			if (!workspace) return

			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: git_repo_resource_path,
				dry_run: false,
				pull: isPullMode,
				only_wmill_yaml: true,
				settings_json: JSON.stringify(getUIState())
			}

			const jobId = await JobService.runScriptByPath({
				workspace,
				path: hubPaths.gitInitRepo,
				requestBody: payloadObj,
				skipPreprocessor: true
			})

			pushJobId = jobId
			pushJobStatus = 'running'
			let jobSuccess = false

			await (await import('$lib/utils')).tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({ workspace, id: jobId })
					jobSuccess = !!testResult.success
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: { reason: 'Push job timed out after 5s' }
						})
					} catch (err) {}
				},
				interval: 500,
				timeout: 10000
			})

			pushJobStatus = jobSuccess ? 'success' : 'failure'
			if (jobSuccess) {
				// Reset preview state after successful push
				previewResult = null
				previewJobId = null
				previewJobStatus = undefined
				previewError = ''
			}
		} catch (e) {
			pushJobStatus = 'failure'
		} finally {
			isPushing = false
		}
	}

	// Simplified export function for backward compatibility
	export function toYaml() {
		return generateYamlFromUI()
	}

	export function setSettings(settings: { yaml: string }) {
		yamlText = settings.yaml
		fromYaml(settings.yaml)
	}

	$effect(() => {
		// Reset preview state when switching modes
		if (isPullMode !== undefined) {
			previewResult = null
			previewJobId = null
			previewJobStatus = undefined
			pushJobId = null
			pushJobStatus = undefined
			isPreviewLoading = false
			isPushing = false
			previewError = ''
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
			{#if !collapsed}
				<button
					class="text-xs px-2 py-1 rounded border border-gray-300 bg-surface-primary hover:bg-surface-secondary"
					onclick={editAsYaml ? switchToUI : switchToYaml}
				>
					{editAsYaml ? 'Edit in UI' : 'Edit as YAML'}
				</button>
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
		{#if editAsYaml}
			<div class="px-4 py-4">
				<textarea
					class="w-full h-64 font-mono text-xs border rounded p-2 bg-gray-50 focus:outline-none focus:ring-2 focus:ring-primary"
					spellcheck="false"
					bind:value={yamlText}
				></textarea>
				{#if yamlError}
					<div class="text-xs text-red-600 mt-2">{yamlError}</div>
				{/if}
			</div>
		{:else}
			<div class="px-4 py-2">
				<div class="grid grid-cols-1 md:grid-cols-2 md:gap-32">
					<div class="flex flex-col gap-2">
						<Tabs bind:selected={filtersTab}>
							<Tab value="includes">Includes</Tab>
							<Tab value="excludes">Excludes</Tab>
							<Tab value="extra">Extra include</Tab>
						</Tabs>

						{#if filtersTab === 'includes'}
							<FilterList title="Include path filters" bind:items={include_path} placeholder="Add filter (e.g. f/**)">
								{#snippet tooltip()}
									<Tooltip>
										Only scripts, flows and apps with their path matching one of those filters will be
										synced to the Git repositories below. The filters allow '*' and '**' characters,
										with '*' matching any character allowed in paths until the next slash (/) and '**'
										matching anything including slashes. By default everything in folders will be
										synced.
									</Tooltip>
								{/snippet}
							</FilterList>
						{:else if filtersTab === 'excludes'}
							<FilterList title="Exclude path filters" bind:items={excludes} placeholder="Add filter (e.g. f/**)">
								{#snippet tooltip()}
									<Tooltip>
										After the include / extra include checks, if a file matches any of these patterns it will be skipped.
									</Tooltip>
								{/snippet}
							</FilterList>
						{:else}
							<FilterList title="Extra include filters" bind:items={extraIncludes} placeholder="Add filter (e.g. f/**)">
								{#snippet tooltip()}
									<Tooltip>
										Secondary allow-list applied after include/exclude. File must match at least one of these in addition to the main includes list if provided.
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
			<div class="mt-6 flex flex-col gap-2 p-2">
				<div class="flex flex-col gap-2 mb-2">
					<Toggle
						size="sm"
						bind:checked={isPullMode}
						options={{
							left: 'Push',
							right: 'Pull'
						}}
					/>
					<span class="text-xs text-tertiary">
						{isPullMode ? 'Pull settings from Git repository' : 'Push settings to Git repository'}
					</span>
				</div>
				<div class="flex gap-2 items-center">
					<Button
						size="sm"
						on:click={previewFiltersToGitRepo}
						disabled={isPreviewLoading || isPushing}
						startIcon={{
							icon: isPreviewLoading ? Loader2 : Eye,
							classes: isPreviewLoading ? 'animate-spin' : ''
						}}
					>
						{isPreviewLoading ? 'Previewing...' : 'Preview'}
					</Button>
					{#if previewResult?.hasChanges && previewResult?.diff && Object.keys(previewResult.diff).length > 0}
						<Button
							size="sm"
							on:click={pushFiltersToGitRepo}
							disabled={isPushing || isPreviewLoading}
							color={isPullMode ? 'dark' : 'red'}
							startIcon={{
								icon: isPushing ? Loader2 : isPullMode ? Check : Save,
								classes: isPushing ? 'animate-spin' : ''
							}}
						>
							{isPushing
								? isPullMode
									? 'Applying...'
									: 'Pushing...'
								: isPullMode
									? 'Apply'
									: 'Push Settings to Git'}
						</Button>
					{/if}
				</div>
				{#if previewError}
					<div class="text-xs text-red-600 mt-2">{previewError}</div>
				{/if}
				{#if previewJobId}
					<div class="flex items-center gap-2 text-xs text-tertiary mt-1">
						{#if previewJobStatus === 'running'}
							<Loader2 class="animate-spin" size={14} />
						{:else if previewJobStatus === 'success'}
							<CheckCircle2 size={14} class="text-green-600" />
						{:else if previewJobStatus === 'failure'}
							<XCircle size={14} class="text-red-700" />
						{/if}
						Preview job:
						<a
							target="_blank"
							class="underline"
							href={`/run/${previewJobId}?workspace=${$workspaceStore}`}>{previewJobId}</a
						>
					</div>
				{/if}
				{#if previewResult}
					<div
						class="border rounded p-2 text-xs max-h-40 overflow-y-auto bg-surface-secondary mt-2"
					>
						<div class="font-semibold text-[11px] mb-1 text-tertiary">Preview of changes:</div>
						{#if previewResult.hasChanges && previewResult.diff && Object.keys(previewResult.diff).length > 0}
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
				{#if pushJobId}
					<div class="flex items-center gap-2 text-xs text-tertiary mt-1">
						{#if pushJobStatus === 'running'}
							<Loader2 class="animate-spin" size={14} />
						{:else if pushJobStatus === 'success'}
							<CheckCircle2 size={14} class="text-green-600" />
						{:else if pushJobStatus === 'failure'}
							<XCircle size={14} class="text-red-700" />
						{/if}
						Push job:
						<a
							target="_blank"
							class="underline"
							href={`/run/${pushJobId}?workspace=${$workspaceStore}`}>{pushJobId}</a
						>
					</div>
				{/if}
			</div>
		{/if}
	{/if}
</div>
