<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Building2, ExternalLink, Settings, User } from 'lucide-svelte'
	import AIPromptsModal from '$lib/components/settings/AIPromptsModal.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import {
		copilotInfo,
		getUserCustomPrompts,
		setCopilotInfo,
		setUserCustomPrompts
	} from '$lib/aiStore'
	import { userStore, workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import type { Item } from '$lib/utils'
	import { base } from '$lib/base'

	const AI_SETTINGS_HREF = `${base}/workspace_settings?tab=ai`

	// Quick access to the AI prompt preferences (user + workspace) for the
	// current chat mode, surfaced next to the model selector in the AI chat.
	const aiChatManager = getAiChatManager()
	let mode = $derived(aiChatManager.mode)

	let modalOpen = $state(false)
	let modalScope = $state<'user' | 'workspace'>('user')
	let customPrompts = $state<Record<string, string>>({})
	let initialPrompt = $state('')
	// Snapshot of the mode the modal was opened for. The live chat mode can change
	// while the modal is open (mode selector sits behind it), so all edit/save/reset
	// operations must key off this snapshot, not the reactive `mode`.
	let activeMode = $state(aiChatManager.mode)

	let isAdmin = $derived(Boolean($userStore?.is_admin || $userStore?.is_super_admin))
	// True when the workspace has no AI providers of its own (it uses instance defaults).
	// In that case the backend never makes workspace custom_prompts effective
	// (get_copilot_info returns the instance config verbatim), so a saved workspace prompt
	// would be dead config — mirror the settings page and surface the prompt read-only.
	let workspaceMissingProviders = $state(false)
	// Workspace prompts are admin-only to edit; non-admins (and admins on a workspace
	// without its own providers) get a read-only view.
	let modalReadOnly = $derived(
		modalScope === 'workspace' && (!isAdmin || workspaceMissingProviders)
	)
	let readOnlyReason = $derived(
		modalScope === 'workspace' && isAdmin && workspaceMissingProviders
			? 'This workspace uses instance AI defaults, so a workspace prompt would have no effect. Configure workspace AI providers in AI settings first.'
			: undefined
	)
	let hasChanges = $derived((customPrompts[activeMode] ?? '') !== initialPrompt)

	function openUserPrompt() {
		activeMode = mode
		initialPrompt = getUserCustomPrompts()[activeMode] ?? ''
		customPrompts = { [activeMode]: initialPrompt }
		modalScope = 'user'
		modalOpen = true
	}

	// Seed from the same source save() writes to (the raw workspace ai_config) and detect
	// whether the workspace has its own providers in the same fetch. Non-admins can't read
	// raw settings, so they see the effective prompt from copilotInfo (read-only).
	async function openWorkspacePrompt() {
		activeMode = mode
		modalScope = 'workspace'
		workspaceMissingProviders = false
		if (!isAdmin) {
			initialPrompt = $copilotInfo.customPrompts?.[activeMode] ?? ''
		} else {
			const workspace = $workspaceStore
			try {
				const settings = workspace ? await WorkspaceService.getSettings({ workspace }) : undefined
				const providers = settings?.ai_config?.providers ?? {}
				workspaceMissingProviders = Object.keys(providers).length === 0
				initialPrompt = settings?.ai_config?.custom_prompts?.[activeMode] ?? ''
			} catch (err) {
				sendUserToast(`Failed to load workspace AI prompt: ${err}`, true)
				initialPrompt = $copilotInfo.customPrompts?.[activeMode] ?? ''
			}
		}
		customPrompts = { [activeMode]: initialPrompt }
		modalOpen = true
	}

	function reset() {
		customPrompts = { [activeMode]: initialPrompt }
	}

	async function save() {
		const value = (customPrompts[activeMode] ?? '').trim()
		if (modalScope === 'user') {
			const prompts = getUserCustomPrompts()
			if (value) {
				prompts[activeMode] = value
			} else {
				delete prompts[activeMode]
			}
			setUserCustomPrompts(prompts)
			initialPrompt = value
			// Sync the editor to the trimmed value so hasChanges resets to false.
			customPrompts = { [activeMode]: value }
			sendUserToast('User AI prompt saved')
			return
		}

		const workspace = $workspaceStore
		if (!workspace) return
		try {
			// Saving prompts requires a full ai_config round-trip; fetch the
			// current config so we don't clobber providers/models/etc.
			const settings = await WorkspaceService.getSettings({ workspace })
			const config = settings.ai_config ?? {}
			const custom_prompts = { ...(config.custom_prompts ?? {}) }
			if (value) {
				custom_prompts[activeMode] = value
			} else {
				delete custom_prompts[activeMode]
			}
			const response = await WorkspaceService.editCopilotConfig({
				workspace,
				requestBody: { ...config, custom_prompts }
			})
			// Editing is gated on the workspace having its own providers, so effective_ai_config
			// equals the saved config (it only falls back to the instance config when the
			// workspace has none) and already carries the custom_prompts we just wrote.
			setCopilotInfo(response.effective_ai_config)
			initialPrompt = value
			// Sync the editor to the trimmed value so hasChanges resets to false.
			customPrompts = { [activeMode]: value }
			sendUserToast('Workspace AI prompt saved')
		} catch (err) {
			sendUserToast(`Failed to save workspace AI prompt: ${err}`, true)
		}
	}

	let items = $derived<Item[]>([
		{ displayName: 'User prompt', icon: User, action: openUserPrompt },
		{ displayName: 'Workspace prompt', icon: Building2, action: openWorkspacePrompt },
		{
			displayName: 'AI settings',
			icon: Settings,
			href: AI_SETTINGS_HREF,
			hrefTarget: '_blank',
			separatorTop: true,
			hide: !isAdmin,
			extra: externalLinkIcon
		}
	])
</script>

{#snippet externalLinkIcon()}
	<ExternalLink size={14} class="shrink-0 text-secondary" />
{/snippet}

<DropdownV2 {items} placement="bottom-end" fixedHeight={false}>
	{#snippet buttonReplacement()}
		<Button
			nonCaptureEvent
			unifiedSize="2xs"
			variant="subtle"
			iconOnly
			startIcon={{ icon: Settings }}
			btnClasses="text-secondary"
			title="AI prompt settings"
		/>
	{/snippet}
</DropdownV2>

<AIPromptsModal
	bind:open={modalOpen}
	bind:customPrompts
	scope={modalScope}
	modes={[activeMode]}
	readOnly={modalReadOnly}
	{readOnlyReason}
	onSave={modalReadOnly ? undefined : save}
	onReset={reset}
	{hasChanges}
	title={modalScope === 'user' ? 'User AI prompt' : 'Workspace AI prompt'}
	target="body"
	fixedHeight="sm"
	settingsHref={isAdmin ? AI_SETTINGS_HREF : undefined}
/>
