<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import {
		ArrowLeft,
		Bot,
		Braces,
		FormInput,
		History,
		MessageSquare,
		Pencil,
		Save,
		Settings
	} from 'lucide-svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		AGENT_CHAT_BLOCKED_REASON,
		agentArgsChatReady
	} from '$lib/components/flows/agentChatFlow'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import AgentEditorHost from '$lib/components/flows/content/AgentEditorHost.svelte'
	import { agentWriteCount, markAgentWritten } from '$lib/components/flows/agentEditorStore.svelte'
	import { ResourceService } from '$lib/gen'
	import { copilotInfo } from '$lib/aiStore'
	import { workspaceStore } from '$lib/stores'

	/**
	 * The standalone agent editor: the same host the flow step's dialog and the resources page
	 * open, on a page of its own. It edits the `ai_agent` resource at the route's path through
	 * the per-user resource draft and runs it from the host's test pane.
	 */
	let path = $derived(page.params.path ?? '')
	let ws = $derived($workspaceStore)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let toolId = $state<string | undefined>(undefined)
	let versionDrawer: Drawer | undefined = $state(undefined)
	let saving = $state(false)
	// A version restore writes the resource behind the host, which still holds the baseline it
	// loaded and any draft over it; deploying from that would write the pre-restore value back.
	// Remounting reloads both, as the dialog form of this editor does by closing.
	let restores = $state(0)

	// Counted per agent so a deploy refetches the version it just minted.
	let writes = $derived(agentWriteCount(ws, path))
	let versionResource = resource(
		() => ({ ws, path, writes }),
		async ({ ws, path, writes }) => {
			if (!ws || !path) return { ws, path, writes }
			const history = await ResourceService.getResourceHistory({ workspace: ws, path })
			return { ws, path, writes, version: history.versions?.[0]?.version }
		}
	)
	let version = $derived.by(() => {
		const loaded = versionResource.current
		return loaded !== undefined &&
			loaded.ws === ws &&
			loaded.path === path &&
			loaded.writes === writes
			? loaded.version
			: undefined
	})

	let draft = $derived(host?.draftHandle())
	let refused = $derived(draft?.refusal != null)
	let readOnly = $derived(draft ? !draft.canWrite : false)
	let draftOnly = $derived(draft?.noDeployed ?? false)

	/** Chat first; the form stays for a run with explicit inputs. Chat needs `auto` memory,
	 *  read off the draft so the toggle follows the memory field as it is edited. */
	let runPane = $state<'chat' | 'form'>('chat')
	let chatReady = $derived(agentArgsChatReady(draft?.state?.args))

	/** Minted by `/agents/add`, as `/scripts/add` mints a script's: the editor starts empty. */
	let isNew = $derived(page.url.searchParams.get('new_draft') === 'true')
	// A draft-only agent is shown at the path its first deploy will create, not at its storage path.
	let shownPath = $derived(draftOnly ? (draft?.state?.draft_path ?? '') : path)
	let pathEditable = $derived(draftOnly && !readOnly && !!draft?.state)
	let settingsOpen = $state(false)

	// A deploy lands on the deployed agent's page, as a flow's does: what was just written is what
	// that page shows and runs.
	async function onDeploy() {
		saving = true
		let deployed = false
		try {
			deployed = (await host?.deploy()) ?? false
		} finally {
			saving = false
		}
		if (deployed) await goto(`${base}/agents/get/${draft?.lastDeployedPath ?? path}`)
	}

	function onSaved(savedPath: string) {
		if (ws) markAgentWritten(ws, savedPath)
	}
</script>

<div class="h-full min-h-0 flex flex-col">
	<div class="flex items-center gap-2 px-4 py-2 border-b shrink-0">
		<Button
			href="{base}/"
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: ArrowLeft }}
			title="Back to home"
		/>
		<Bot size={18} class="text-violet-500 shrink-0" />
		<div class="flex items-center gap-2 min-w-0">
			<!-- Laid out as the script and flow editors' header is: the path above with its pen, the
			     summary line editable in place. -->
			<div class="inline-block max-w-full align-top group px-2 py-0.5 leading-tight min-w-0">
				<div class="flex items-center max-w-full text-2xs text-secondary font-mono min-w-0">
					<span class="truncate">{shownPath}</span>
					<!-- A deployed agent is renamed with Move, which keeps the flows linking it in view;
					     only a first deploy picks its path here. -->
					{#if pathEditable}
						<Popover
							placement="bottom-start"
							contentClasses="p-4"
							usePointerDownOutside
							disableFocusTrap
							closeOnOtherPopoverOpen
						>
							{#snippet trigger()}
								<Button
									variant="subtle"
									unifiedSize="xs"
									iconOnly
									startIcon={{ icon: Pencil }}
									title="Edit path"
									aria-label="Edit path"
									btnClasses="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
								/>
							{/snippet}
							{#snippet content()}
								<div class="w-[480px]">{@render pathField(true)}</div>
							{/snippet}
						</Popover>
					{/if}
				</div>
				<div class="max-w-full -mt-[2px]" title={draft?.state?.description}>
					<EditableInput
						value={draft?.state?.description ?? ''}
						placeholder="Add a summary..."
						editable={!readOnly && !!draft?.state}
						commitOnInput
						size="sm"
						onSave={(v) => {
							if (draft?.state) draft.state.description = v.trim()
						}}
						textClass="text-xs font-semibold text-emphasis leading-tight"
						class="max-w-full"
					/>
				</div>
			</div>
			{#if version != undefined && !refused}
				<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
					v{version}
				</Badge>
			{/if}
			<!-- The draft autosave state, as the flow and script editors show it. -->
			{#if ws && !refused}
				<AutosaveIndicator
					workspace={ws}
					itemKind="resource"
					{path}
					{draftOnly}
					onResetToDeployed={() => draft?.sync.resetToDeployed(path)}
				/>
			{/if}
			{#if readOnly}
				<Badge color="gray" class="shrink-0" title="You do not have write access to this agent">
					Read only
				</Badge>
			{/if}
		</div>
		<span class="text-xs text-tertiary hidden xl:inline truncate">
			Changes here update the saved agent, and every flow that links to it.
		</span>
		<div class="grow"></div>
		{#if !refused}
			<ToggleButtonGroup
				selected={runPane}
				onSelected={(v) => (runPane = v as 'chat' | 'form')}
				noWFull
			>
				{#snippet children({ item })}
					<ToggleButton
						value="chat"
						label="Chat"
						icon={MessageSquare}
						size="md"
						disabled={!chatReady}
						tooltip={chatReady ? undefined : AGENT_CHAT_BLOCKED_REASON}
						{item}
					/>
					<ToggleButton value="form" label="Form" icon={FormInput} size="md" {item} />
				{/snippet}
			</ToggleButtonGroup>
			<div class="grow"></div>
			<!-- Nothing deployed means no resource to open as JSON and no history to list. -->
			{#if !draftOnly}
				<Button
					unifiedSize="sm"
					variant="default"
					startIcon={{ icon: Braces }}
					iconOnly
					title="Edit as JSON in the resources page"
					href="{base}/resources#/resource/{path}"
				/>
				<Button
					unifiedSize="sm"
					variant="default"
					startIcon={{ icon: History }}
					iconOnly
					title="Version history"
					on:click={() => versionDrawer?.openDrawer()}
				/>
			{/if}
			<Button
				unifiedSize="sm"
				variant="default"
				startIcon={{ icon: Settings }}
				title="Settings"
				on:click={() => (settingsOpen = true)}
			>
				Settings
			</Button>
			<Button
				unifiedSize="sm"
				variant="accent"
				startIcon={{ icon: Save }}
				loading={saving}
				disabled={readOnly}
				title={readOnly ? 'You do not have write access to this agent' : undefined}
				on:click={onDeploy}
			>
				Deploy
			</Button>
		{/if}
	</div>

	<div class="shrink-0">
		<LocalDraftBanner
			show={draft?.sync.hasDraft ?? false}
			reserveSpace={false}
			getDeployed={() => draft?.deployed}
			getCurrent={() => draft?.state}
			onDiscard={() => draft?.sync.resetToDeployed(path)}
			title="Deployed <> Unsaved agent changes"
		/>
	</div>

	<div class="flex-1 min-h-0">
		{#key `${ws}:${path}:${restores}`}
			<AgentEditorHost
				bind:this={host}
				{path}
				workspace={ws}
				enableAi={$copilotInfo.enabled}
				{toolId}
				onSelectTool={(id) => (toolId = id)}
				{onSaved}
				{runPane}
				{isNew}
			/>
		{/key}
	</div>
</div>

<!-- The path field, shared by the header's pen and the settings drawer. A deployed agent's path
     is shown, not edited: renaming it is Move, from its page. -->
{#snippet pathField(autofocus: boolean)}
	{#if pathEditable}
		<Path
			{autofocus}
			bind:path={
				() => draft?.state?.draft_path ?? '',
				(v) => {
					if (draft?.state) draft.state.draft_path = v
				}
			}
			initialPath={draft?.state?.draft_path ?? ''}
			namePlaceholder="agent"
			kind="resource"
			size="sm"
			drawerOffset={4000}
		/>
	{:else}
		<span class="text-xs font-mono text-secondary">{shownPath}</span>
		<p class="text-2xs text-tertiary mt-1">
			Rename a deployed agent with Move, from its page, so the flows linking it follow.
		</p>
	{/if}
{/snippet}

<!-- Laid out as the script editor's settings: summary, path, then what the item is for. -->
<Drawer placement="right" bind:open={settingsOpen} size="800px">
	<DrawerContent title="Settings" on:close={() => (settingsOpen = false)}>
		<Section label="Metadata">
			<div class="flex flex-col gap-6">
				<Label label="Summary">
					<TextInput
						bind:value={
							() => draft?.state?.description ?? '',
							(v) => {
								if (draft?.state) draft.state.description = v
							}
						}
						disabled={readOnly || !draft?.state}
						inputProps={{ placeholder: 'Short summary to be displayed when listed' }}
					/>
				</Label>
				<Label label="Path">
					{#snippet header()}
						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions#path"
						>
							The unique identifier of the agent in the workspace that defines permissions
						</Tooltip>
					{/snippet}
					{@render pathField(false)}
				</Label>
			</div>
		</Section>
	</DrawerContent>
</Drawer>

<Drawer bind:this={versionDrawer} size="1200px">
	<DrawerContent title="Version history" on:close={() => versionDrawer?.closeDrawer()} noPadding>
		<ResourceVersionHistory
			{path}
			workspace={ws}
			canRestore={!readOnly}
			onRestore={() => {
				versionDrawer?.closeDrawer()
				if (ws) markAgentWritten(ws, path)
				restores++
			}}
		/>
	</DrawerContent>
</Drawer>
