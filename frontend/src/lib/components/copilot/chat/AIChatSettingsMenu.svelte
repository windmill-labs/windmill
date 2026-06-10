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

	// Quick access to the AI prompt preferences (user + workspace) for the
	// current chat mode, surfaced next to the model selector in the AI chat.
	const aiChatManager = getAiChatManager()
	let mode = $derived(aiChatManager.mode)

	let modalOpen = $state(false)
	let modalScope = $state<'user' | 'workspace'>('user')
	let customPrompts = $state<Record<string, string>>({})
	let initialPrompt = $state('')

	let isAdmin = $derived(Boolean($userStore?.is_admin || $userStore?.is_super_admin))
	// Workspace prompts are admin-only to edit; non-admins get a read-only view.
	let modalReadOnly = $derived(modalScope === 'workspace' && !isAdmin)
	let hasChanges = $derived((customPrompts[mode] ?? '') !== initialPrompt)

	function openUserPrompt() {
		initialPrompt = getUserCustomPrompts()[mode] ?? ''
		customPrompts = { [mode]: initialPrompt }
		modalScope = 'user'
		modalOpen = true
	}

	async function openWorkspacePrompt() {
		const m = mode
		modalScope = 'workspace'
		initialPrompt = await loadWorkspacePrompt(m)
		customPrompts = { [m]: initialPrompt }
		modalOpen = true
	}

	// Seed from the same source save() writes to (the raw workspace ai_config), so an
	// inherited instance prompt is never silently persisted. copilotInfo holds the
	// *effective* config (instance fallback when the workspace has no providers), which
	// would diverge from what gets saved. Non-admins can't read raw settings, so they
	// see the effective prompt (read-only).
	async function loadWorkspacePrompt(m: string): Promise<string> {
		if (!isAdmin) return $copilotInfo.customPrompts?.[m] ?? ''
		const workspace = $workspaceStore
		if (!workspace) return ''
		try {
			const settings = await WorkspaceService.getSettings({ workspace })
			return settings.ai_config?.custom_prompts?.[m] ?? ''
		} catch (err) {
			sendUserToast(`Failed to load workspace AI prompt: ${err}`, true)
			return $copilotInfo.customPrompts?.[m] ?? ''
		}
	}

	function reset() {
		customPrompts = { [mode]: initialPrompt }
	}

	async function save() {
		const value = (customPrompts[mode] ?? '').trim()
		if (modalScope === 'user') {
			const prompts = getUserCustomPrompts()
			if (value) {
				prompts[mode] = value
			} else {
				delete prompts[mode]
			}
			setUserCustomPrompts(prompts)
			initialPrompt = value
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
				custom_prompts[mode] = value
			} else {
				delete custom_prompts[mode]
			}
			const response = await WorkspaceService.editCopilotConfig({
				workspace,
				requestBody: { ...config, custom_prompts }
			})
			// effective_ai_config falls back to the instance config when the workspace has no
			// providers of its own, so it doesn't echo back the custom_prompts we just saved.
			// Re-apply them onto the effective config so copilotInfo (and getCombinedCustomPrompt)
			// reflect the saved workspace prompt without requiring a reload.
			setCopilotInfo({ ...response.effective_ai_config, custom_prompts })
			initialPrompt = value
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
			href: '/workspace_settings?tab=ai',
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
	modes={[mode]}
	readOnly={modalReadOnly}
	onSave={modalReadOnly ? undefined : save}
	onReset={reset}
	{hasChanges}
	title={modalScope === 'user' ? 'User AI prompt' : 'Workspace AI prompt'}
	target="body"
	fixedHeight="sm"
	settingsHref={isAdmin ? '/workspace_settings?tab=ai' : undefined}
/>
