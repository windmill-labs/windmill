<script lang="ts">
	/**
	 * The session chat's model button: a fixed ChatModelSettings config over the copilot's
	 * own state — the workspace's configured models, the session's model/effort selection
	 * and its localStorage pins, the custom-prompt editors, and the free-tier grant.
	 */
	import { User, Building2, Settings, ExternalLink } from 'lucide-svelte'
	import ChatModelSettings from '../ChatModelSettings.svelte'
	import type { ChatModelSettingsConfig } from '../chatModelSettings'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME,
		COPILOT_SESSION_REASONING_SETTING_NAME,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import { storeLocalSetting, type Item } from '$lib/utils'
	import {
		copilotInfo,
		copilotSessionModel,
		getUserCustomPrompts,
		setCopilotInfo,
		setUserCustomPrompts
	} from '$lib/aiStore'
	import { WorkspaceService, type AIProvider, type AIProviderModel } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import AIPromptsModal from '$lib/components/settings/AIPromptsModal.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { thinkingPreferences } from './thinkingPreferences.svelte'
	import {
		getReasoningCapability,
		REASONING_OFF,
		type ReasoningProviderModel
	} from '../reasoningRegistry'

	let {
		/** Whether this dropdown carries the custom-prompt entries. Off where the surface
		 * has an assistant settings modal — its Instructions section owns them there, and
		 * two ways in would drift. The home composer has no such modal, so it keeps them. */
		promptSettings = true
	}: { promptSettings?: boolean } = $props()

	const aiChatManager = getAiChatManager()
	const AI_SETTINGS_HREF = `${base}/workspace_settings?tab=ai`

	let providerModel = $derived(
		($copilotSessionModel ??
			$copilotInfo.defaultModel ??
			$copilotInfo.aiModels[0] ?? {
				model: 'No model',
				provider: 'No provider'
			}) as ReasoningProviderModel
	)
	let models = $derived($copilotInfo.aiModels)

	// Free tier: the workspace has no key of its own and is spending Windmill's one-time
	// grant. Label it so the user knows whose budget this is, and warn before it runs out
	// rather than letting the grant die mid-task.
	let freeTier = $derived($copilotInfo.freeTier)
	let freeUsedPct = $derived(Math.min(100, Math.round((freeTier?.used_ratio ?? 0) * 100)))
	let freeRunningLow = $derived(!!freeTier && !freeTier.exhausted && freeUsedPct >= 80)

	function selectModel(m: AIProviderModel) {
		// Carry the effort onto the new model only if it supports that level ('off'
		// only where the model can truly disable); otherwise drop it so the model's
		// default applies.
		const carried = providerModel.reasoning
		const cap = getReasoningCapability(m.provider, m.model)
		const keep =
			carried === REASONING_OFF
				? cap.canDisable
				: carried !== undefined && cap.levels.includes(carried)
		$copilotSessionModel = { ...m, ...(keep ? { reasoning: carried } : {}) }
		storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, m.model)
		storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, m.provider)
		storeLocalSetting(COPILOT_SESSION_REASONING_SETTING_NAME, keep ? carried : undefined)
	}

	function selectReasoning(value: string) {
		const reasoning = value === REASONING_OFF ? REASONING_OFF : value
		$copilotSessionModel = {
			...providerModel,
			provider: providerModel.provider as AIProvider,
			reasoning
		}
		// Pin the current model selection so the reasoning choice persists with it.
		storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, providerModel.model)
		storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, providerModel.provider)
		storeLocalSetting(COPILOT_SESSION_REASONING_SETTING_NAME, reasoning)
	}

	// ---- prompt parameters (User / Workspace custom prompts) ----
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
	// In that case the backend never makes workspace custom_prompts effective, so a saved
	// workspace prompt would be dead config — mirror the settings page and surface it read-only.
	let workspaceMissingProviders = $state(false)
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
			customPrompts = { [activeMode]: value }
			sendUserToast('User AI prompt saved')
			return
		}

		const workspace = $workspaceStore
		if (!workspace) return
		try {
			// Saving prompts requires a full ai_config round-trip; fetch the current
			// config so we don't clobber providers/models/etc.
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
			setCopilotInfo(response.effective_ai_config)
			initialPrompt = value
			customPrompts = { [activeMode]: value }
			sendUserToast('Workspace AI prompt saved')
		} catch (err) {
			sendUserToast(`Failed to save workspace AI prompt: ${err}`, true)
			// Re-throw so AIPromptsModal keeps the modal open on a failed save.
			throw err
		}
	}

	// Prompt parameters, surfaced as a melt submenu. The menu keeps itself open on item
	// click, so these actions close it explicitly before opening a modal.
	function paramItems(close: () => void): Item {
		return {
			displayName: 'Parameters',
			icon: Settings,
			submenuItems: [
				{
					displayName: 'User prompt',
					icon: User,
					action: () => {
						close()
						openUserPrompt()
					}
				},
				{
					displayName: 'Workspace prompt',
					icon: Building2,
					action: () => {
						close()
						openWorkspacePrompt()
					}
				},
				{
					displayName: 'AI settings',
					icon: Settings,
					href: AI_SETTINGS_HREF,
					hrefTarget: '_blank',
					separatorTop: true,
					hide: !isAdmin,
					extra: externalLinkIcon
				}
			]
		}
	}

	const config = $derived<ChatModelSettingsConfig>({
		label: providerModel.model,
		title: 'Model & reasoning settings',
		badge: freeTier && !freeTier.exhausted ? { text: 'Free', warn: freeRunningLow } : undefined,
		// Off in a session: the assistant settings modal's Instructions section owns the
		// prompt entries there, so the menu would offer the same thing twice.
		topItems: promptSettings ? (close) => [paramItems(close)] : undefined,
		sections: [
			{
				label: 'Model',
				options: models.map((m) => ({
					key: `${m.provider}/${m.model}`,
					label: m.model,
					selected: m.model === providerModel.model && m.provider === providerModel.provider,
					onSelect: () => selectModel(m)
				}))
			}
		],
		reasoning: {
			provider: providerModel.provider as AIProvider,
			model: providerModel.model,
			value: providerModel.reasoning,
			offToken: REASONING_OFF,
			// The copilot fills an unset effort in before it calls the provider, so unset
			// really does run at the default level and the button may name it.
			sendsDefaultWhenUnset: true,
			onSelect: selectReasoning
		},
		// A reading preference rather than a model parameter: it applies to every chat in
		// this browser, including thinking already in the transcript. No close(): flipping
		// it should not dismiss the menu.
		bottomItems: () => [
			{
				displayName: 'Always expand thinking',
				selected: thinkingPreferences.expandByDefault,
				action: () => (thinkingPreferences.expandByDefault = !thinkingPreferences.expandByDefault)
			}
		]
	})
</script>

{#snippet externalLinkIcon()}
	<ExternalLink size={14} class="shrink-0 text-secondary" />
{/snippet}

<ChatModelSettings {config} />

{#if promptSettings}
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
{/if}
