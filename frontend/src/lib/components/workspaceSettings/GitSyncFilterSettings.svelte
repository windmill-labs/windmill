<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Plus, X, Filter, Save, Eye, Loader2, CheckCircle2, XCircle, Check } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import yaml from 'js-yaml'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

	// UI-controlled fields in the YAML based on SyncOptions interface
	interface UIControlledFields {
		includes?: string[]
		// Skip flags
		skipVariables?: boolean
		skipResources?: boolean
		skipResourceTypes?: boolean
		skipSecrets?: boolean
		// Include flags
		includeSchedules?: boolean
		includeTriggers?: boolean
		includeUsers?: boolean
		includeGroups?: boolean
	}

	// Full YAML config type that includes UI fields and other possible fields from SyncOptions
	interface FullYamlConfig extends UIControlledFields {
		defaultTs?: 'bun' | 'deno'
		excludes?: string[]
		extraIncludes?: string[]
		codebases?: Array<{
			relative_path: string
			includes?: string[]
			excludes?: string[]
			assets?: Array<{
				from: string
				to: string
			}>
			customBundler?: string
			external?: string[]
			define?: { [key: string]: string }
			inject?: string[]
		}>
		[key: string]: unknown
	}

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
	}

	type PreviewResult = {
		diff?: string
		yaml?: string
	}

	function validateYamlStructure(obj: unknown): obj is FullYamlConfig {
		return obj !== null && typeof obj === 'object'
	}

	let {
		git_repo_resource_path = $bindable(''),
		include_path = $bindable(['f/**']),
		include_type = $bindable(['script', 'flow', 'app', 'folder'] as ObjectType[]),
		yamlText = $bindable(''),
		onSettingsChange = (settings: { yaml: string }) => {}
	} = $props()

	// Component state
	let newFilter = $state('')
	let newFilterInput: HTMLInputElement | null = $state(null)
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

	// Compute type toggles from include_type
	const typeToggles = $derived({
		scripts: include_type.includes('script'),
		flows: include_type.includes('flow'),
		apps: include_type.includes('app'),
		folders: include_type.includes('folder'),
		resourceTypes: include_type.includes('resourcetype'),
		resources: include_type.includes('resource'),
		variables: include_type.includes('variable'),
		secrets: include_type.includes('secret'),
		schedules: include_type.includes('schedule'),
		users: include_type.includes('user'),
		groups: include_type.includes('group'),
		triggers: include_type.includes('trigger')
	})

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
			triggers: 'trigger'
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

	function addPathFilterInline() {
		const value = newFilter.trim()
		if (value && !include_path.includes(value)) {
			include_path = [...include_path, value]
			newFilter = ''
			newFilterInput?.focus()
		}
	}

	function removePathFilter(idx: number) {
		include_path = include_path.filter((_, i) => i !== idx)
	}

	function capitalize(str: string) {
		return str.charAt(0).toUpperCase() + str.slice(1)
	}

	function fromYaml(yamlStr: string) {
		yamlError = ''
		try {
			const parsed = yaml.load(yamlStr)

			if (!validateYamlStructure(parsed)) {
				throw new Error('Invalid YAML structure')
			}

			const obj: FullYamlConfig = parsed

			// Store the full YAML for later use when saving
			yamlText = yamlStr

			// Handle includes with proper validation
			if (obj.includes !== undefined) {
				if (!Array.isArray(obj.includes)) {
					throw new Error('includes must be an array')
				}
				include_path = obj.includes.map((p: unknown) => {
					if (typeof p !== 'string') {
						throw new Error('includes must contain only strings')
					}
					// Handle quoted strings
					if (/^['"].*['"]$/.test(p)) {
						return p.slice(1, -1).replace(/''/g, "'")
					}
					return p
				})
			}

			// Update type toggles based on skip/include flags
			const newTypes = new Set<ObjectType>()

			// Always include core types (scripts, flows, apps, folders)
			// These are fundamental Windmill types not controlled by CLI flags
			newTypes.add('script')
			newTypes.add('flow')
			newTypes.add('app')
			newTypes.add('folder')

			// Handle resource management (controlled by skip flags)
			if (!obj.skipResourceTypes) newTypes.add('resourcetype')
			if (!obj.skipResources) newTypes.add('resource')
			if (!obj.skipVariables) newTypes.add('variable')
			if (!obj.skipSecrets) newTypes.add('secret')

			// Handle include flags
			if (obj.includeSchedules) newTypes.add('schedule')
			if (obj.includeTriggers) newTypes.add('trigger')
			if (obj.includeUsers) newTypes.add('user')
			if (obj.includeGroups) newTypes.add('group')

			// Special handling for secrets - they can only be included if variables are included
			if (!newTypes.has('variable')) {
				newTypes.delete('secret')
			}

			include_type = Array.from(newTypes)

		} catch (e) {
			yamlError = e.message || 'Invalid YAML'
			console.error('Error parsing YAML:', e)
		}
	}

	export function toYaml() {
		try {
			// If we have existing YAML text, modify it directly to preserve structure
			if (yamlText?.trim()) {
				// Generate a base YAML with our current settings
				let baseConfig: FullYamlConfig = {
					defaultTs: 'bun',
					includes: include_path,
					excludes: [],
					codebases: []
				};

				// Add skip flags if true
				if (!include_type.includes('variable')) baseConfig.skipVariables = true;
				if (!include_type.includes('resource')) baseConfig.skipResources = true;
				if (!include_type.includes('secret')) baseConfig.skipSecrets = true;
				if (!include_type.includes('resourcetype')) baseConfig.skipResourceTypes = true;

				// Add include flags if true
				if (include_type.includes('schedule')) baseConfig.includeSchedules = true;
				if (include_type.includes('trigger')) baseConfig.includeTriggers = true;
				if (include_type.includes('user')) baseConfig.includeUsers = true;
				if (include_type.includes('group')) baseConfig.includeGroups = true;

				// Create a new YAML
				let newYaml = yaml.dump(baseConfig, {
					indent: 2,
					lineWidth: -1,
					quotingType: '"',
					forceQuotes: false,
					noRefs: true
				});

				// Now preserve explicit false values from the original YAML
				const includeFields = ['includeSchedules', 'includeTriggers', 'includeUsers', 'includeGroups'];

				// Check for explicit false values in original YAML
				for (const field of includeFields) {
					// If field exists in original YAML as false, but not in new YAML, add it
					const regex = new RegExp(`^${field}:\\s*false\\s*$`, 'm');
					if (regex.test(yamlText) && !new RegExp(`^${field}:`, 'm').test(newYaml)) {
						newYaml += `${field}: false\n`;
					}
				}

				return newYaml;
			}

			// If no existing YAML, create a new config
			let config: FullYamlConfig = {
				defaultTs: 'bun',
				includes: include_path,
				excludes: [],
				codebases: []
			}

			// Add skip flags if true
			if (!include_type.includes('variable')) config.skipVariables = true
			if (!include_type.includes('resource')) config.skipResources = true
			if (!include_type.includes('secret')) config.skipSecrets = true
			if (!include_type.includes('resourcetype')) config.skipResourceTypes = true

			// Add include flags if true
			if (include_type.includes('schedule')) config.includeSchedules = true
			if (include_type.includes('trigger')) config.includeTriggers = true
			if (include_type.includes('user')) config.includeUsers = true
			if (include_type.includes('group')) config.includeGroups = true

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
			return yamlText
		}
	}

	function switchToYaml() {
		yamlText = toYaml()
		yamlError = ''
		editAsYaml = true
	}

	function switchToUI() {
		fromYaml(yamlText)
		if (!yamlError) {
			editAsYaml = false
		}
	}

	async function previewFiltersToGitRepo() {
		isPreviewLoading = true
		previewError = ''
		previewResult = null
		previewJobId = null
		previewJobStatus = undefined
		try {
			const workspace = $workspaceStore
			if (!workspace) return
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: git_repo_resource_path,
				yaml: editAsYaml ? yamlText : toYaml(),
				only_wmill_yaml: true,
				dry_run: true,
				pull: isPullMode
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
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})
					jobSuccess = !!testResult.success
					if (jobSuccess) {
						const jobResult = await JobService.getCompletedJobResult({
							workspace,
							id: jobId
						})
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
				if (isPullMode && result) {
					// In pull mode, we need to compare the YAMLs properly
					try {
						// Parse both YAMLs
						const repoYaml = result.yaml
						const currentYaml = editAsYaml ? yamlText : toYaml()

						if (repoYaml) {
							const repoObj = yaml.load(repoYaml) as FullYamlConfig
							const currentObj = yaml.load(currentYaml) as FullYamlConfig

							// Function to normalize YAML object for comparison
							const normalizeYamlObj = (obj: FullYamlConfig) => {
								const normalized = { ...obj }
								// Ensure all boolean fields exist with default values
								// Skip flags default to false in YAML config (by default, we include these resources)
								normalized.skipVariables = normalized.skipVariables ?? false
								normalized.skipResources = normalized.skipResources ?? false
								normalized.skipResourceTypes = normalized.skipResourceTypes ?? false
								normalized.skipSecrets = normalized.skipSecrets ?? false
								// Include flags default to false (by default, we exclude these special types)
								normalized.includeSchedules = normalized.includeSchedules ?? false
								normalized.includeTriggers = normalized.includeTriggers ?? false
								normalized.includeUsers = normalized.includeUsers ?? false
								normalized.includeGroups = normalized.includeGroups ?? false
								// Ensure arrays exist
								normalized.includes = normalized.includes ?? []
								normalized.excludes = normalized.excludes ?? []
								normalized.codebases = normalized.codebases ?? []
								return normalized
							}

							const normalizedRepo = normalizeYamlObj(repoObj)
							const normalizedCurrent = normalizeYamlObj(currentObj)

							// Compare normalized objects
							const isDifferent = JSON.stringify(normalizedRepo) !== JSON.stringify(normalizedCurrent)

							if (isDifferent) {
								// Generate a semantic diff showing only the actual changes
								let diffOutput = '';

								// Compare each field and show only what actually changed
								const allFields = ['defaultTs', 'includes', 'excludes', 'codebases', 'skipVariables', 'skipResources', 'skipSecrets', 'includeSchedules', 'includeTriggers', 'includeUsers', 'includeGroups', 'skipResourceTypes'];

								for (const field of allFields) {
									const currentValue = normalizedCurrent[field];
									const repoValue = normalizedRepo[field];

									// Deep compare the values
									if (JSON.stringify(currentValue) !== JSON.stringify(repoValue)) {
										diffOutput += `${field}:\n`;
										if (Array.isArray(currentValue) && Array.isArray(repoValue)) {
											// Show array differences more clearly
											const currentSet = new Set(currentValue);
											const repoSet = new Set(repoValue);

											// Items being removed (in current but not in repo)
											for (const item of currentValue) {
												if (!repoSet.has(item)) {
													diffOutput += `  - ${item}\n`;
												}
											}

											// Items being added (in repo but not in current)
											for (const item of repoValue) {
												if (!currentSet.has(item)) {
													diffOutput += `  + ${item}\n`;
												}
											}
										} else {
											// Show simple value changes
											if (currentValue !== undefined) {
												diffOutput += `  - ${field}: ${currentValue}\n`;
											}
											if (repoValue !== undefined) {
												diffOutput += `  + ${field}: ${repoValue}\n`;
											}
										}
									}
								}

								result.diff = diffOutput || 'Configuration files differ but specific changes could not be determined';
							} else {
								result.diff = '';  // No differences
							}
						}
					} catch (e) {
						console.error('Error comparing YAMLs:', e)
						previewError = 'Failed to compare YAML files: ' + e.message
					}
					previewResult = result
				} else {
					previewResult = result
				}
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

	async function pushFiltersToGitRepo() {
		if (isPullMode) {
			if (previewResult?.yaml) {
				try {
					fromYaml(previewResult.yaml)
					yamlText = previewResult.yaml
					onSettingsChange({ yaml: previewResult.yaml })
					sendUserToast('Changes applied - remember to save repository settings to persist changes')

					// Clear the preview state after applying settings
					previewResult = null
					previewJobId = null
					previewJobStatus = undefined
					previewError = ''
				} catch (e) {
					previewError = 'Failed to parse pulled YAML: ' + e.message
				}
			}
			return
		}

		isPushing = true
		pushJobId = null
		pushJobStatus = undefined
		try {
			const workspace = $workspaceStore
			if (!workspace) return
			const payloadObj = {
				workspace_id: workspace,
				repo_url_resource_path: git_repo_resource_path,
				yaml: editAsYaml ? yamlText : toYaml(),
				dry_run: false,
				pull: isPullMode
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
			await (
				await import('$lib/utils')
			).tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})
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
					<!-- Path Filters Section (Left) -->
					<div>
						<div class="flex items-center gap-2 mb-3">
							<h4 class="font-semibold text-sm">Path filters</h4>
							<Tooltip>
								Only scripts, flows and apps with their path matching one of those filters will be
								synced to the Git repositories below. The filters allow '*' and '**' characters,
								with '*' matching any character allowed in paths until the next slash (/) and '**'
								matching anything including slashes. By default everything in folders will be
								synced.
							</Tooltip>
						</div>
						<div class="flex flex-wrap gap-2 items-center mb-2">
							{#each include_path as path, idx (path)}
								<span
									class="flex items-center bg-gray-100 rounded-full px-3 py-1 text-xs text-gray-700"
								>
									{path}
									<button
										class="ml-2 text-gray-400 hover:text-red-500 focus:outline-none"
										onclick={() => removePathFilter(idx)}
										aria-label="Remove filter"
									>
										<X size={14} />
									</button>
								</span>
							{/each}
							<input
								bind:this={newFilterInput}
								class="border border-gray-300 rounded-full px-3 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-primary"
								placeholder="Add filter (e.g. f/**)"
								value={newFilter}
								oninput={(e) => (newFilter = e.currentTarget.value)}
								onkeydown={(e) => e.key === 'Enter' && (addPathFilterInline(), e.preventDefault())}
							/>
							<button
								class="ml-1 text-primary hover:bg-primary/10 rounded-full p-1"
								onclick={addPathFilterInline}
								aria-label="Add filter"
							>
								<Plus size={14} />
							</button>
						</div>
						{#if include_path.length === 0}
							<div class="text-xs text-gray-400 mt-1"
								>No filters set. Everything will be synced.</div
							>
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
					{#if previewResult?.diff?.trim()}
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
						{#if previewResult.diff?.trim()}
							<div class="mt-2">
								<pre class="rounded p-2 text-2xs"><code>{previewResult.diff}</code></pre>
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
