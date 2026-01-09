<script lang="ts">
	import { Button } from './common'
	import { ExternalLink, Loader2, Save } from 'lucide-svelte'
	import { SettingService, WorkerService, WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import { DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING } from '$lib/consts'
	import Toggle from './Toggle.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import Badge from './common/badge/Badge.svelte'
	import Section from './Section.svelte'
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

	// Change detection
	let originalDefaultTagPerWorkspace = $state<boolean | undefined>(defaultTagPerWorkspace)
	let originalDefaultTagWorkspaces = $state<string[]>(defaultTagWorkspaces)

	// Detect changes
	let hasChanges = $derived(
		originalDefaultTagPerWorkspace !== defaultTagPerWorkspace ||
			JSON.stringify($state.snapshot(originalDefaultTagWorkspaces)?.sort() || []) !==
				JSON.stringify($state.snapshot(defaultTagWorkspaces)?.sort() || [])
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

		// Update original state after save
		originalDefaultTagPerWorkspace = defaultTagPerWorkspace
		originalDefaultTagWorkspaces = [...(defaultTagWorkspaces || [])]

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
		</div>
	{/if}
</Section>
