<script lang="ts">
	import { Button } from './common'
	import { ExternalLink, Loader2, Save } from 'lucide-svelte'
	import { SettingService, WorkerService, WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import {
		DEFAULT_TAGS_PER_WORKSPACE_SETTING,
		DEFAULT_TAGS_WORKSPACES_SETTING,
		FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING,
		PREVIEW_TAGS_OVERRIDE_SETTING
	} from '$lib/consts'
	import Toggle from './Toggle.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import Badge from './common/badge/Badge.svelte'
	import Section from './Section.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	interface Props {
		defaultTagPerWorkspace?: boolean | undefined
		defaultTagWorkspaces?: string[]
	}

	let {
		defaultTagPerWorkspace = $bindable(undefined),
		defaultTagWorkspaces = $bindable([])
	}: Props = $props()

	let defaultTags = $state<string[] | undefined>(undefined)
	let limitToWorkspaces = $state(false)
	let previewTagsOverride = $state(false)
	let forkAppendForkSuffix = $state(false)

	// Change detection
	let originalDefaultTagPerWorkspace = $state<boolean | undefined>(defaultTagPerWorkspace)
	let originalDefaultTagWorkspaces = $state<string[]>(defaultTagWorkspaces)
	let originalPreviewTagsOverride = $state(false)
	let originalForkAppendForkSuffix = $state(false)

	// Detect changes
	let hasChanges = $derived(
		originalDefaultTagPerWorkspace !== defaultTagPerWorkspace ||
			JSON.stringify($state.snapshot(originalDefaultTagWorkspaces)?.sort() || []) !==
				JSON.stringify($state.snapshot(defaultTagWorkspaces)?.sort() || []) ||
			originalPreviewTagsOverride !== previewTagsOverride ||
			originalForkAppendForkSuffix !== forkAppendForkSuffix
	)

	let workspaces: string[] = $state([])
	async function loadWorkspaces() {
		workspaces = (await WorkspaceService.listWorkspacesAsSuperAdmin()).map((m) => m.id)
	}

	async function loadDefaultTags() {
		try {
			defaultTags = (await WorkerService.geDefaultTags()) ?? []
			defaultTagWorkspaces =
				((await SettingService.getGlobal({
					key: DEFAULT_TAGS_WORKSPACES_SETTING
				})) as any) ?? []
			limitToWorkspaces = defaultTagWorkspaces ? defaultTagWorkspaces.length > 0 : false
			previewTagsOverride =
				((await SettingService.getGlobal({
					key: PREVIEW_TAGS_OVERRIDE_SETTING
				})) as any) ?? false
			originalPreviewTagsOverride = previewTagsOverride
			const forkSetting = (await SettingService.getGlobal({
				key: FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING
			})) as any
			forkAppendForkSuffix = forkSetting ?? false
			originalForkAppendForkSuffix = forkAppendForkSuffix
		} catch (err) {
			sendUserToast(`Could not load default tags: ${err}`, true)
		}
	}

	async function handleSave() {
		await SettingService.setGlobal({
			key: DEFAULT_TAGS_PER_WORKSPACE_SETTING,
			requestBody: {
				value: defaultTagPerWorkspace
			}
		})
		await SettingService.setGlobal({
			key: DEFAULT_TAGS_WORKSPACES_SETTING,
			requestBody: {
				value:
					limitToWorkspaces && defaultTagWorkspaces && defaultTagWorkspaces.length > 0
						? defaultTagWorkspaces
						: undefined
			}
		})
		await SettingService.setGlobal({
			key: PREVIEW_TAGS_OVERRIDE_SETTING,
			requestBody: {
				value: previewTagsOverride
			}
		})
		await SettingService.setGlobal({
			key: FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING,
			requestBody: {
				value: forkAppendForkSuffix
			}
		})

		// Update original state after save
		originalDefaultTagPerWorkspace = defaultTagPerWorkspace
		originalDefaultTagWorkspaces = [...(defaultTagWorkspaces || [])]
		originalPreviewTagsOverride = previewTagsOverride
		originalForkAppendForkSuffix = forkAppendForkSuffix

		loadDefaultTags()
		sendUserToast('Saved')
	}

	loadDefaultTags()
	loadWorkspaces()
</script>

<Section label="Default tags">
	<div class="text-2xs text-secondary mb-2">
		Jobs that have not been specifically assigned custom tags will use a <a
			href="https://www.windmill.dev/docs/core_concepts/worker_groups#default-worker-group"
			target="_blank"
			class="gap-1 items-baseline">default tags <ExternalLink size={12} class="inline-block" /></a
		> based on the language they are in or their kind.
	</div>

	{#snippet action()}
		{#if !$enterpriseLicense}
			<span class="text-secondary text-xs">Read only</span>
		{:else}
			<Button
				variant="accent"
				unifiedSize="md"
				on:click={handleSave}
				startIcon={{ icon: Save }}
				disabled={!hasChanges || !$enterpriseLicense || !$superadmin}
			>
				Save
			</Button>
		{/if}
	{/snippet}

	{#if defaultTagPerWorkspace == undefined || defaultTags == undefined}
		<Loader2 class="animate-spin" />
	{:else if !$enterpriseLicense}
		<!-- Tag List -->
		<div class="flex gap-y-1 gap-x-2 flex-wrap">
			{#each $state.snapshot(defaultTags).sort() as tag (tag)}
				<Badge color="blue">{defaultTagPerWorkspace ? `${tag}-$workspace` : tag}</Badge>
			{/each}
		</div>
	{:else}
		<!-- Settings -->
		<div class="py-4 flex flex-col gap-2">
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={defaultTagPerWorkspace}
					options={{
						right: 'make default tags workspace specific',
						rightTooltip:
							'When tags use $workspace, the final tag has $workspace replaced with the workspace id, allowing multi-vpc setup with more ease, without having to assign a specific tag each time.'
					}}
					class="w-fit"
					disabled={!$enterpriseLicense}
				/>
			</div>
			{#if defaultTagPerWorkspace}
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold">Fork workspace behavior</span>
					<ToggleButtonGroup
						selected={forkAppendForkSuffix ? 'parent-fork' : 'parent'}
						onSelected={(v) => (forkAppendForkSuffix = v === 'parent-fork')}
						disabled={!$enterpriseLicense}
					>
						{#snippet children({ item })}
							<ToggleButton
								value="parent"
								label="Use parent workspace tag"
								tooltip={'Fork jobs are tagged with the parent workspace id (e.g. python3-{parent_id}), so they are picked up by workers assigned to the parent workspace.'}
								{item}
							/>
							<ToggleButton
								value="parent-fork"
								label="Use parent workspace tag + -fork suffix"
								tooltip={'Fork jobs are tagged with the parent workspace id and a "-fork" suffix (e.g. python3-{parent_id}-fork), so all forks of a given parent share a dedicated tag. Route these jobs to workers provisioned specifically for forks of the parent.'}
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<Toggle
					bind:checked={limitToWorkspaces}
					options={{ right: 'only for some workspaces' }}
					class="w-fit"
					disabled={!$enterpriseLicense}
				/>
				{#if limitToWorkspaces}
					<MultiSelect
						disablePortal
						disabled={!$enterpriseLicense}
						items={safeSelectItems(workspaces)}
						bind:value={defaultTagWorkspaces}
					/>
				{/if}
			{/if}
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={previewTagsOverride}
					options={{
						right: 'route preview jobs to dedicated preview tag',
						rightTooltip:
							'When enabled, preview jobs (script previews and flow previews) will be routed to the "preview" tag instead of their language-specific tag, allowing you to dedicate specific workers for previews.'
					}}
					class="w-fit"
					disabled={!$enterpriseLicense}
				/>
			</div>
		</div>

		<div class="flex gap-2 items-center mb-1">
			<div class="w-36 text-2xs font-semibold text-secondary">Job language or kind</div>
			<div class="w-6 text-2xs font-semibold text-secondary"></div>
			<div class="flex-1 text-2xs font-semibold text-secondary">Default tag</div>
		</div>

		<!-- Tag List -->
		<div class="flex gap-y-1 flex-col">
			{#each $state.snapshot(defaultTags).sort() as tag (tag)}
				<div class="flex gap-2 items-center">
					<div class="w-36">
						<Badge color="transparent">{tag}</Badge>
					</div>

					<div class="w-6 flex justify-center text-secondary">&rightarrow;</div>
					<div class="flex-1">
						<Badge color="blue">{defaultTagPerWorkspace ? `${tag}-$workspace` : tag}</Badge>
					</div>
				</div>
			{/each}
			{#if previewTagsOverride}
				<div class="flex gap-2 items-center">
					<div class="w-36">
						<Badge color="transparent">preview</Badge>
					</div>
					<div class="w-6 flex justify-center text-secondary">&rightarrow;</div>
					<div class="flex-1">
						<Badge color="blue">{defaultTagPerWorkspace ? 'preview-$workspace' : 'preview'}</Badge>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</Section>
