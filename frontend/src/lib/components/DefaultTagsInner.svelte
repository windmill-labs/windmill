<script lang="ts">
	import { Button } from './common'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'
	import { SettingService, WorkerService, WorkspaceService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import { DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING } from '$lib/consts'
	import Toggle from './Toggle.svelte'
	import MultiSelectWrapper from './multiselect/MultiSelectLegacyWrapper.svelte'

	let defaultTags: string[] | undefined = undefined
	export let defaultTagPerWorkspace: boolean | undefined = undefined
	export let defaultTagWorkspaces: string[] | undefined = undefined
	let limitToWorkspaces = false

	let workspaces: string[] = []
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

	loadDefaultTags()
	loadWorkspaces()
</script>

<div class="flex flex-col w-80 p-2 gap-2">
	{#if !$enterpriseLicense}
		<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap justify-end">
			<AlertTriangle size={16} />
			EE only <Tooltip>Enterprise Edition only feature</Tooltip>
		</div>
	{/if}
	{#if defaultTagPerWorkspace == undefined || defaultTags == undefined}
		<Loader2 class="animate-spin" />
	{:else}
		<div class="flex flex-col gap-y-1">
			{#each defaultTags.sort() as tag (tag)}
				<div class="flex gap-2 items-center"
					><div class="p-1 text-xs px-2 rounded border text-primary w-32">{tag} </div><div
						class="flex gap-2 items-center w-92"
						>&rightarrow;
						<input
							class="text-xs w-full"
							disabled
							type="text"
							value={defaultTagPerWorkspace ? `${tag}-$workspace` : tag}
						/></div
					>
				</div>
			{/each}
		</div>
		<div id="default-tags-settings" class="py-4 flex flex-col gap-2">
			<Toggle
				bind:checked={defaultTagPerWorkspace}
				options={{ right: 'workspace specific default tags' }}
			/>
			{#if defaultTagPerWorkspace}
				<Toggle bind:checked={limitToWorkspaces} options={{ right: 'only for some workspaces' }} />
				{#if limitToWorkspaces}
					<MultiSelectWrapper
						target="#default-tags-settings"
						items={workspaces}
						bind:value={defaultTagWorkspaces}
					/>
				{/if}
			{/if}
		</div>
		<Button
			variant="contained"
			color="blue"
			size="sm"
			on:click={async () => {
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
				loadDefaultTags()
				sendUserToast('Saved')
			}}
			disabled={!$enterpriseLicense || !$superadmin}
		>
			Save {#if !$superadmin}
				<span class="text-2xs text-tertiary">superadmin only</span>
			{/if}
		</Button>

		<span class="text-2xs text-tertiary"
			>When tags use <pre class="inline">$workspace</pre>, the final tag has
			<pre class="inline">$workspace</pre> replaced with the workspace id, allowing multi-vpc setup with
			more ease, without having to assign a specific tag each time.</span
		>
	{/if}
</div>
