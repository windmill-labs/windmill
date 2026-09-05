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
	import { userStore, type UserExt } from '$lib/stores'
	import { getUserExt } from '$lib/user'
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
	// The section acts on `ws`, which in a session is not the nav workspace, so the link
	// has to name it or it opens settings the user may not administer.
	let aiSettingsHref = $derived(`${base}/workspace_settings?workspace=${ws}&tab=ai`)

	let mode = $derived(aiChatManager.mode)
	let tab = $state<'workspace' | 'user'>('workspace')
	let note = $derived(
		tab === 'workspace'
			? `Applies to everyone in ${ws}.`
			: 'Stored in this browser and sent in every workspace, so they follow you rather than the workspace.'
	)
	// `$userStore.is_admin` is the role in the nav workspace, not necessarily in `ws`, so
	// the resolved role is keyed to the workspace it was read for, and an unresolved one
	// reads as no admin: offering the field and taking it away on resolve would discard
	// whatever was typed in between. Superadmin holds everywhere.
	let targetRole = $state<{ workspace: string; user: UserExt | undefined } | undefined>(undefined)
	let roleRead = $derived(targetRole?.workspace === ws ? targetRole : undefined)
	let navIsTarget = $derived($userStore?.workspace_id === ws)
	let roleForTarget = $derived(roleRead?.user)
	let roleResolved = $derived(roleRead !== undefined || navIsTarget)
	// A read that came back with no user: read-only like a non-admin, but said differently,
	// since a failed lookup is not evidence of the role it failed to read.
	let roleUnknown = $derived(roleRead !== undefined && !roleRead.user && !navIsTarget)
	let isAdmin = $derived(
		Boolean(
			$userStore?.is_super_admin ||
				(roleForTarget ? roleForTarget.is_admin : navIsTarget && $userStore?.is_admin)
		)
	)

	// A session whose fork is still staged has no workspace of its own yet, so `ws`
	// resolves to the PARENT. Saving here would edit the live parent's shared AI config,
	// which every chat still in it would then be given.
	function pendingForkParent(): string | undefined {
		return aiChatManager.sessionContextResolver?.()?.pendingForkOf
	}
	let forkPending = $derived(pendingForkParent() !== undefined)

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
	let workspaceReadOnly = $derived(!isAdmin || workspaceMissingProviders || forkPending)
	let readOnlyReason = $derived(
		forkPending
			? `This session has not created its workspace yet, so these would be saved to "${pendingForkParent()}" and given to every chat already in it. Send a message first.`
			: !roleResolved
				? `Checking your access to ${ws}.`
				: roleUnknown
					? `Could not read your access to ${ws}. Reopen this section to try again.`
					: !isAdmin
						? 'Only workspace admins can edit the workspace instructions.'
						: 'This workspace uses instance AI defaults, so a workspace prompt would have no effect. Configure workspace AI providers in AI settings first.'
	)

	let workspaceChanged = $derived(!workspaceReadOnly && workspaceDraft !== workspaceSaved)
	let userChanged = $derived(userDraft !== userSaved)
	let dirty = $derived(workspaceChanged || userChanged)

	$effect(() => {
		blocksClose = dirty
	})

	// Loaded on the first look and again on a workspace switch: B's instructions must not
	// be saved over A's. A reload drops a draft, so it waits until one is saved or
	// reverted — hence `dirty` tracked, and `loadedWorkspace` to pick the wait up against
	// whatever `ws` is by then (a staged fork commits into a workspace of its own).
	let loadSeq = 0
	let loadedWorkspace: string | undefined = undefined
	$effect(() => {
		const target = ws
		const shown = active
		const clean = !dirty
		untrack(() => {
			// Parked: read again on the next visit, since the workspace half is editable
			// from the settings page too.
			if (!shown) {
				loadedWorkspace = undefined
				return
			}
			if (!target || !clean || target === loadedWorkspace) return
			loadedWorkspace = target
			void load(target)
		})
	})

	async function load(target: string) {
		const seq = ++loadSeq
		loading = true
		try {
			const resolved = await getUserExt(target).catch(() => undefined)
			if (seq !== loadSeq) return
			targetRole = { workspace: target, user: resolved }
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
			// Read before the assignments below move the baseline they compare against.
			// Each half is then seeded only while it still holds what it was loaded with:
			// the effect that starts this checks `dirty` before the request, not after it,
			// so an admin who opens the tab and types straight away would otherwise have
			// the settings response land on top of the text they are in the middle of.
			const keepWorkspaceDraft = workspaceChanged
			const keepUserDraft = userChanged
			workspaceSaved = workspace
			if (!keepWorkspaceDraft) workspaceDraft = workspace
			userSaved = user
			if (!keepUserDraft) userDraft = user
		} finally {
			if (seq === loadSeq) loading = false
		}
	}

	/** Escape puts the fields back rather than closing the modal: `blocksClose` holds the
	 * modal shut while there is unsaved text, so this is the way out of a draft, and a
	 * second press then closes. */
	function onKeydown(event: KeyboardEvent) {
		// Only while this is the section on screen: every section stays mounted, so an
		// Escape meant for another one would revert these drafts with nothing visible
		// to say that it had.
		if (!active || event.key !== 'Escape' || !dirty) return
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
		const parent = pendingForkParent()
		if (parent !== undefined) {
			sendUserToast(
				`This session has not created its workspace yet, so the instructions would be saved to "${parent}". Send a message first.`,
				true
			)
			return
		}
		// Pinned across both awaits: the read and the write have to land on one workspace.
		const target = ws
		try {
			// Saving prompts requires a full ai_config round-trip; fetch the current config
			// so we don't clobber providers/models/etc.
			const settings = await WorkspaceService.getSettings({ workspace: target })
			const config = settings.ai_config ?? {}
			const custom_prompts = { ...(config.custom_prompts ?? {}) }
			if (value) {
				custom_prompts[mode] = value
			} else {
				delete custom_prompts[mode]
			}
			const response = await WorkspaceService.editCopilotConfig({
				workspace: target,
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
				// Also while saving: the write spans two requests and its success path puts the
				// submitted value back, so text typed in between would be swallowed. `load`
				// keeps a newer draft instead, because it starts on its own and must not block
				// someone who opened the tab to type.
				readonly: p.readOnly || saving,
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
					href={aiSettingsHref}
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
