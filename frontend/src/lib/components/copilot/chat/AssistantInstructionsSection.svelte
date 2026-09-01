<!--
@component
The Instructions section of the assistant settings modal: the two blocks of custom
instructions the system prompt carries — the workspace one an admin sets for everyone,
and the user one stored in this browser — a tab each, both editable in place. One Save
writes whichever of the two changed, including the tab that is not on screen.
-->
<script lang="ts">
	import { Button, Section, Tab, TabContent, Tabs } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import {
		copilotInfo,
		getUserCustomPrompts,
		setCopilotInfo,
		setUserCustomPrompts
	} from '$lib/aiStore'
	import { WorkspaceService } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import { Building2, ExternalLink, User } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	let {
		ws,
		active,
		blocksClose = $bindable()
	}: {
		/** The workspace the chat operates on, which is not always the one on screen. */
		ws: string
		/** Whether this is the panel on screen. The workspace half costs a settings read,
		 * so it is fetched when someone looks at instructions rather than on every open
		 * of the modal. */
		active: boolean
		/** True while this section is in the middle of something the modal must not
		 * close under — here, instructions typed and not yet saved. */
		blocksClose: boolean
	} = $props()

	const MAX_PROMPT_LENGTH = 5000

	const aiChatManager = getAiChatManager()
	const AI_SETTINGS_HREF = `${base}/workspace_settings?tab=ai`

	let mode = $derived(aiChatManager.mode)
	let tab = $state<'workspace' | 'user'>('workspace')
	let note = $derived(
		tab === 'workspace'
			? `Applies to everyone in ${ws}.`
			: 'Stored in this browser and sent in every workspace, so they follow you rather than the workspace.'
	)
	let isAdmin = $derived(Boolean($userStore?.is_admin || $userStore?.is_super_admin))

	// What is on screen, against what was loaded. Two pairs rather than one: the halves
	// are stored in different places and save through different calls, and only the one
	// that changed should be written.
	let workspaceDraft = $state('')
	let workspaceSaved = $state('')
	let userDraft = $state('')
	let userSaved = $state('')
	let loading = $state(false)
	let saving = $state(false)

	// True when the workspace has no AI providers of its own (it uses instance defaults).
	// In that case the backend never makes workspace custom_prompts effective, so a saved
	// workspace prompt would be dead config — mirror the settings page and show it read-only.
	let workspaceMissingProviders = $state(false)
	let workspaceReadOnly = $derived(!isAdmin || workspaceMissingProviders)
	let readOnlyReason = $derived(
		!isAdmin
			? 'Only workspace admins can edit the workspace instructions.'
			: 'This workspace uses instance AI defaults, so a workspace prompt would have no effect. Configure workspace AI providers in AI settings first.'
	)

	let workspaceChanged = $derived(!workspaceReadOnly && workspaceDraft !== workspaceSaved)
	let userChanged = $derived(userDraft !== userSaved)
	let dirty = $derived(workspaceChanged || userChanged)

	$effect(() => {
		blocksClose = dirty
	})

	// Loaded when the panel is first looked at, and again on a workspace switch: the
	// workspace half belongs to one workspace, and B's instructions must not be saved
	// over A's. A reload also drops a draft, so it is skipped while one is unsaved.
	let loadSeq = 0
	$effect(() => {
		const target = ws
		const shown = active
		untrack(() => {
			if (!shown || !target || dirty) return
			void load(target)
		})
	})

	async function load(target: string) {
		const seq = ++loadSeq
		loading = true
		try {
			const user = getUserCustomPrompts()[mode] ?? ''
			// Seeded from the same source `saveWorkspace` writes to (the raw workspace
			// ai_config), which also says whether the workspace has providers of its own.
			// Non-admins cannot read raw settings, so they get the effective prompt.
			let workspace = $copilotInfo.customPrompts?.[mode] ?? ''
			let missingProviders = false
			if (isAdmin) {
				try {
					const settings = await WorkspaceService.getSettings({ workspace: target })
					missingProviders = Object.keys(settings?.ai_config?.providers ?? {}).length === 0
					workspace = settings?.ai_config?.custom_prompts?.[mode] ?? ''
				} catch (err) {
					sendUserToast(`Failed to load workspace AI prompt: ${err}`, true)
				}
			}
			if (seq !== loadSeq) return
			workspaceMissingProviders = missingProviders
			workspaceSaved = workspace
			workspaceDraft = workspace
			userSaved = user
			userDraft = user
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	/** Escape puts the fields back rather than closing the modal: `blocksClose` holds the
	 * modal shut while there is unsaved text, so this is the way out of a draft, and a
	 * second press then closes. */
	function onKeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !dirty) return
		event.preventDefault()
		event.stopPropagation()
		workspaceDraft = workspaceSaved
		userDraft = userSaved
	}

	async function save() {
		saving = true
		try {
			if (userChanged) saveUser(userDraft.trim())
			if (workspaceChanged) await saveWorkspace(workspaceDraft.trim())
		} finally {
			saving = false
		}
	}

	function saveUser(value: string) {
		const prompts = getUserCustomPrompts()
		if (value) {
			prompts[mode] = value
		} else {
			delete prompts[mode]
		}
		setUserCustomPrompts(prompts)
		// These live in localStorage, which nothing observes, so the chat is told rather
		// than left to pick them up on its next send. `update_user_instructions` already
		// rebuilds this way when the assistant edits the same block.
		aiChatManager.rebuildGlobalSystemMessage()
		userSaved = value
		userDraft = value
		sendUserToast('User instructions were saved')
	}

	async function saveWorkspace(value: string) {
		try {
			// Saving prompts requires a full ai_config round-trip; fetch the current config
			// so we don't clobber providers/models/etc.
			const settings = await WorkspaceService.getSettings({ workspace: ws })
			const config = settings.ai_config ?? {}
			const custom_prompts = { ...(config.custom_prompts ?? {}) }
			if (value) {
				custom_prompts[mode] = value
			} else {
				delete custom_prompts[mode]
			}
			const response = await WorkspaceService.editCopilotConfig({
				workspace: ws,
				requestBody: { ...config, custom_prompts }
			})
			setCopilotInfo(response.effective_ai_config)
			workspaceSaved = value
			workspaceDraft = value
			sendUserToast('Workspace instructions were saved')
		} catch (err) {
			// The field keeps what was typed, so the save can be retried.
			sendUserToast(`Failed to save workspace AI prompt: ${err}`, true)
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet field(p: { value: string; readOnly: boolean; onInput: (v: string) => void })}
	<div class="flex flex-col gap-1">
		<TextInput
			value={p.value}
			underlyingInputEl="textarea"
			size="sm"
			class="min-h-24 resize-y"
			inputProps={{
				placeholder: p.readOnly ? '' : 'Anything the assistant should always keep in mind',
				rows: 4,
				maxlength: MAX_PROMPT_LENGTH,
				readonly: p.readOnly,
				oninput: (e) => p.onInput(e.currentTarget.value)
			}}
		/>
		{#if !p.readOnly}
			<span class="self-end text-2xs text-hint">
				{p.value.length}/{MAX_PROMPT_LENGTH} characters
			</span>
		{/if}
	</div>
{/snippet}

<Section
	label="Instructions"
	description="Text added to every system prompt in this workspace, on top of the assistant's own. Both blocks are sent, the workspace one first."
	class="flex flex-col gap-4"
>
	{#snippet action()}
		<div class="flex items-center gap-2 shrink-0">
			{#if isAdmin}
				<Button
					href={AI_SETTINGS_HREF}
					target="_blank"
					variant="subtle"
					unifiedSize="sm"
					endIcon={{ icon: ExternalLink }}
				>
					AI settings
				</Button>
			{/if}
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={!dirty || saving || loading}
				onClick={save}
			>
				Save
			</Button>
		</div>
	{/snippet}

	<Tabs values={['workspace', 'user']} bind:selected={tab}>
		<Tab value="workspace" label="Workspace" icon={Building2} />
		<Tab value="user" label="User (you)" icon={User} />
		{#snippet content()}
			<!-- The note belongs to the panel rather than to either tab: one element that
			     follows the selection, so switching tabs does not rebuild it and the space
			     under the tab row is the same on both. The wrapper carries no padding of its
			     own — `Tabs` renders the row and this content as two roots, so the Section's
			     `gap-4` is already the space between them. -->
			<div class="flex flex-col gap-2">
				<Description>{note}</Description>
				{#if workspaceReadOnly && tab === 'workspace'}
					<Alert type="info" title="These are read-only for you" size="xs">
						{readOnlyReason}
					</Alert>
				{/if}
				<!-- `alwaysMounted`: Save writes whichever half changed, including the tab that
				     is not on screen, so a field left with unsaved text has to go on holding it. -->
				<TabContent value="workspace" alwaysMounted>
					{@render field({
						value: workspaceDraft,
						readOnly: workspaceReadOnly,
						onInput: (v) => (workspaceDraft = v)
					})}
				</TabContent>
				<TabContent value="user" alwaysMounted>
					{@render field({
						value: userDraft,
						readOnly: false,
						onInput: (v) => (userDraft = v)
					})}
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</Section>
