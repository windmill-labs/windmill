<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Plus, X, Filter, Save, Eye, Loader2, CheckCircle2, XCircle } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import yaml from 'js-yaml'
	import hubPaths from '$lib/hubPaths.json'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'

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

	let {
		git_repo_resource_path = $bindable(''),
		include_path = $bindable(['f/**']),
		include_type = $bindable<GitSyncTypeMap>({
			scripts: true,
			flows: true,
			apps: true,
			folders: true,
			resourceTypes: false,
			resources: false,
			variables: false,
			secrets: false,
			schedules: false,
			users: false,
			groups: false,
			triggers: false
		}),
		yamlText = $bindable(''),
		onSettingsChange = (settings: { yaml: string }) => {}
	} = $props()

	// Initialize yamlText with current settings
	$effect(() => {
		const initialYaml = toYaml()
		if (initialYaml !== yamlText) {
			onSettingsChange({ yaml: initialYaml })
		}
	})

	// Add pull mode state
	let isPullMode = $state(false)

	// Individual $state variables for toggles
	let scripts = $state(include_type.scripts)
	let flows = $state(include_type.flows)
	let apps = $state(include_type.apps)
	let folders = $state(include_type.folders)
	let resourceTypes = $state(include_type.resourceTypes)
	let resources = $state(include_type.resources)
	let variables = $state(include_type.variables)
	let secrets = $state(include_type.secrets)
	let schedules = $state(include_type.schedules)
	let users = $state(include_type.users)
	let groups = $state(include_type.groups)
	let triggers = $state(include_type.triggers)

	let newFilter = $state('')
	let newFilterInput: HTMLInputElement | null = $state(null)
	let collapsed = $state(false)
	let editAsYaml = $state(false)
	let yamlError = $state('')
	let previewResult = $state<PreviewResult | null>(null)
	let previewJobId = $state<string | null>(null)
	let previewJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let pushJobId = $state<string | null>(null)
	let pushJobStatus = $state<'running' | 'success' | 'failure' | undefined>(undefined)
	let isPreviewLoading = $state(false)
	let isPushing = $state(false)
	let previewError = $state('')

	function updateSettings() {
		const newYaml = toYaml()
		if (newYaml !== yamlText) {
			onSettingsChange({ yaml: newYaml })
		}
	}

	function updateIncludeType(key: keyof GitSyncTypeMap, value: boolean) {
		switch (key) {
			case 'scripts':
				scripts = value
				break
			case 'flows':
				flows = value
				break
			case 'apps':
				apps = value
				break
			case 'folders':
				folders = value
				break
			case 'resourceTypes':
				resourceTypes = value
				break
			case 'resources':
				resources = value
				break
			case 'variables':
				variables = value
				if (!value) secrets = false
				break
			case 'secrets':
				secrets = value
				break
			case 'schedules':
				schedules = value
				break
			case 'users':
				users = value
				break
			case 'groups':
				groups = value
				break
			case 'triggers':
				triggers = value
				break
		}
		updateSettings()
	}

	function addPathFilterInline() {
		const value = newFilter.trim()
		if (value && !include_path.includes(value)) {
			include_path = [...include_path, value]
			newFilter = ''
			newFilterInput?.focus()
			updateSettings()
		}
	}

	function removePathFilter(idx: number) {
		include_path.splice(idx, 1)
		include_path = [...include_path]
		updateSettings()
	}

	function getIncludeTypeObj(): GitSyncTypeMap {
		return {
			scripts,
			flows,
			apps,
			folders,
			resourceTypes,
			resources,
			variables,
			secrets,
			schedules,
			users,
			groups,
			triggers
		}
	}

	function capitalize(str: string) {
		return str.charAt(0).toUpperCase() + str.slice(1)
	}

	function toYaml() {
		const obj: any = {
			includes: include_path
		}
		for (const [key, value] of Object.entries(getIncludeTypeObj())) {
			obj[`skip${key.charAt(0).toUpperCase() + key.slice(1)}`] = !value
		}
		obj.codebases = []
		obj.excludes = []
		return yaml.dump(obj)
	}

	function fromYaml(yamlStr: string) {
		console.log('fromYaml', yamlStr)
		yamlError = ''
		try {
			const obj = yaml.load(yamlStr) as any
			if (!obj || typeof obj !== 'object') throw new Error('Invalid YAML')
			if (!Array.isArray(obj.includes)) throw new Error('Missing includes')

			// Update path filters
			include_path = obj.includes.map((p: string) => {
				if (typeof p === 'string' && /^'.*'$/.test(p)) {
					return p.slice(1, -1).replace(/''/g, "'")
				}
				return p
			})

			// Update type filters with defaults
			const parsed = {
				scripts: obj.skipScripts === undefined ? true : !obj.skipScripts,
				flows: obj.skipFlows === undefined ? true : !obj.skipFlows,
				apps: obj.skipApps === undefined ? true : !obj.skipApps,
				folders: obj.skipFolders === undefined ? true : !obj.skipFolders,
				resourceTypes: obj.skipResourceTypes === undefined ? false : !obj.skipResourceTypes,
				resources: obj.skipResources === undefined ? false : !obj.skipResources,
				variables: obj.skipVariables === undefined ? false : !obj.skipVariables,
				secrets: obj.skipSecrets === undefined ? false : !obj.skipSecrets,
				schedules: obj.skipSchedules === undefined ? false : !obj.skipSchedules,
				users: obj.skipUsers === undefined ? false : !obj.skipUsers,
				groups: obj.skipGroups === undefined ? false : !obj.skipGroups,
				triggers: obj.skipTriggers === undefined ? false : !obj.skipTriggers
			}

			// Update all state variables
			scripts = parsed.scripts
			flows = parsed.flows
			apps = parsed.apps
			folders = parsed.folders
			resourceTypes = parsed.resourceTypes
			resources = parsed.resources
			variables = parsed.variables
			secrets = parsed.secrets
			schedules = parsed.schedules
			users = parsed.users
			groups = parsed.groups
			triggers = parsed.triggers

			// Update include_type
			include_type = parsed
			console.log('include_type', include_type)
		} catch (e) {
			yamlError = e.message || 'Invalid YAML'
			console.error('Error parsing YAML:', e)
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
				timeout: 5000
			})
			previewJobStatus = jobSuccess ? 'success' : 'failure'
			if (jobSuccess) {
				if (isPullMode && result) {
					// In pull mode, compute diff between current and pulled YAML
					const currentYaml = toYaml()
					const lines = currentYaml.split('\n')
					const pulledLines = result.yaml?.split('\n') || []
					const diff: string[] = []

					// Simple line-by-line diff
					for (let i = 0; i < Math.max(lines.length, pulledLines.length); i++) {
						if (i >= lines.length) {
							diff.push(`+ ${pulledLines[i]}`)
						} else if (i >= pulledLines.length) {
							diff.push(`- ${lines[i]}`)
						} else if (lines[i] !== pulledLines[i]) {
							diff.push(`- ${lines[i]}`)
							diff.push(`+ ${pulledLines[i]}`)
						}
					}

					result.diff = diff.join('\n')
				}
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

	async function pushFiltersToGitRepo() {
		if (isPullMode) {
			if (previewResult?.yaml) {
				try {
					fromYaml(previewResult.yaml)
					yamlText = previewResult.yaml
					sendUserToast('Repo filter settings saved')
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
				timeout: 5000
			})
			pushJobStatus = jobSuccess ? 'success' : 'failure'
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
									bind:checked={scripts}
									onchange={(e) => updateIncludeType('scripts', e.detail)}
									options={{ right: capitalize('scripts') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={flows}
									onchange={(e) => updateIncludeType('flows', e.detail)}
									options={{ right: capitalize('flows') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={apps}
									onchange={(e) => updateIncludeType('apps', e.detail)}
									options={{ right: capitalize('apps') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={folders}
									onchange={(e) => updateIncludeType('folders', e.detail)}
									options={{ right: capitalize('folders') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={resourceTypes}
									onchange={(e) => updateIncludeType('resourceTypes', e.detail)}
									options={{ right: capitalize('resourceTypes') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={resources}
									onchange={(e) => updateIncludeType('resources', e.detail)}
									options={{ right: capitalize('resources') }}
								/>
							</div>
							<div class="col-span-2 flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={variables}
									onchange={(e) => updateIncludeType('variables', e.detail)}
									options={{ right: 'Variables' }}
								/>
								<span class="text-gray-400">-</span>
								<Toggle
									size="xs"
									disabled={!variables}
									bind:checked={secrets}
									onchange={(e) => updateIncludeType('secrets', e.detail)}
									options={{ left: 'Include secrets' }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={schedules}
									onchange={(e) => updateIncludeType('schedules', e.detail)}
									options={{ right: capitalize('schedules') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={users}
									onchange={(e) => updateIncludeType('users', e.detail)}
									options={{ right: capitalize('users') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={groups}
									onchange={(e) => updateIncludeType('groups', e.detail)}
									options={{ right: capitalize('groups') }}
								/>
							</div>
							<div class="flex items-center gap-2">
								<Toggle
									size="xs"
									bind:checked={triggers}
									onchange={(e) => updateIncludeType('triggers', e.detail)}
									options={{ right: capitalize('triggers') }}
								/>
							</div>
						</div>
					</div>
				</div>
			</div>
			<div class="mt-6 flex flex-col gap-2 p-2">
				<div class="flex flex-col  gap-2 mb-2">
					<Toggle
						size="sm"
						bind:checked={isPullMode}
						options={{
							left: 'Push',
							right: 'Pull',
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
							color="red"
							startIcon={{
								icon: isPushing ? Loader2 : Save,
								classes: isPushing ? 'animate-spin' : ''
							}}
						>
							{isPushing ? (isPullMode ? 'Saving...' : 'Pushing...') : (isPullMode ? 'Save' : 'Save & Push')}
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
								<pre
									class="rounded p-2 text-2xs"
									><code>{previewResult.diff}</code></pre
								>
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
