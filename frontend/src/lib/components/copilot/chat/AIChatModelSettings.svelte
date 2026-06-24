<script lang="ts">
	import { ChevronDown, Check, User, Building2, Settings, ExternalLink } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import DropdownSubmenuItem from '$lib/components/DropdownSubmenuItem.svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import MenuItemWrapper from '$lib/components/meltComponents/MenuItemWrapper.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
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
	import {
		getReasoningCapability,
		resolveEffectiveReasoning,
		REASONING_OFF,
		type ReasoningProviderModel
	} from '../reasoningRegistry'

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

	let capability = $derived(
		getReasoningCapability(providerModel.provider as AIProvider, providerModel.model)
	)
	// Effective effort accounts for the default-on level on capable models.
	let currentEffort = $derived(resolveEffectiveReasoning(providerModel))
	// Slider stops: an off position only where the model can truly disable (else the
	// provider would coerce it to the lowest level), then the provider-native levels.
	let stops = $derived([...(capability.canDisable ? [REASONING_OFF] : []), ...capability.levels])
	let currentStop = $derived(
		providerModel.reasoning === REASONING_OFF
			? REASONING_OFF
			: (currentEffort ?? stops[stops.length - 1])
	)
	let stopIndex = $derived(Math.max(0, stops.indexOf(currentStop)))
	// Percentage filled (accent) up to the thumb; the rest of the track stays surface-secondary.
	let fillPct = $derived(stops.length > 1 ? Math.round((stopIndex / (stops.length - 1)) * 100) : 0)
	// Button suffix: the effort token, or 'off' when explicitly disabled. Omitted entirely
	// for models with no reasoning support.
	let effortLabel = $derived(capability.supported ? (currentEffort ?? REASONING_OFF) : undefined)

	// The trigger label resizes when the effort changes (e.g. dragging the slider while the menu
	// is open). With a `bottom-end` popover anchored to the trigger's right edge, that resize would
	// shift the popover. So we freeze the trigger to its width at open time and release it on close —
	// no movement while open, and natural sizing (no reserved padding) the rest of the time.
	let menuOpen = $state(false)
	let triggerEl: HTMLElement | undefined = $state(undefined)
	let lockedWidth = $state<number | undefined>(undefined)
	$effect(() => {
		if (menuOpen) {
			if (lockedWidth === undefined && triggerEl) {
				lockedWidth = triggerEl.getBoundingClientRect().width
			}
		} else {
			lockedWidth = undefined
		}
	})

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

	// Prompt parameters, surfaced as a melt submenu (hover-opens and is floating-positioned,
	// so it flips on screen edges instead of overflowing). The menu keeps itself open on
	// item click (closeOnItemClick=false), so these actions close it explicitly via `close`.
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

	// Keep the slider's pointer events from bubbling to the enclosing melt item: melt's
	// roving focus blurs the focused element on pointermove, which would abort the native
	// thumb drag. Direct (non-delegated) listeners so they run before melt's item listener.
	function isolatePointer(node: HTMLElement) {
		const stop = (e: Event) => e.stopPropagation()
		node.addEventListener('pointerdown', stop)
		node.addEventListener('pointermove', stop)
		return {
			destroy() {
				node.removeEventListener('pointerdown', stop)
				node.removeEventListener('pointermove', stop)
			}
		}
	}

	// Adjust the reasoning effort with the arrow keys while the Thinking item is focused.
	function adjustEffort(e: KeyboardEvent) {
		if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
		e.preventDefault()
		const next = Math.min(
			stops.length - 1,
			Math.max(0, stopIndex + (e.key === 'ArrowRight' ? 1 : -1))
		)
		selectReasoning(stops[next])
	}
</script>

{#snippet externalLinkIcon()}
	<ExternalLink size={14} class="shrink-0 text-secondary" />
{/snippet}

<DropdownV2
	customMenu
	placement="bottom-end"
	fixedHeight={false}
	closeOnItemClick={false}
	bind:open={menuOpen}
>
	{#snippet buttonReplacement()}
		<div
			bind:this={triggerEl}
			style={lockedWidth !== undefined ? `width: ${lockedWidth}px` : undefined}
		>
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				endIcon={{ icon: ChevronDown }}
				btnClasses="w-full max-w-[200px] text-secondary font-normal"
				title="Model & reasoning settings"
			>
				<span class="flex items-center gap-1 min-w-0">
					<span class="truncate">{providerModel.model}</span>
					{#if effortLabel}
						<span class="shrink-0 text-tertiary">· {effortLabel}</span>
					{/if}
				</span>
			</Button>
		</div>
	{/snippet}
	{#snippet menu({ item, builders, close })}
		<div
			class="bg-surface-tertiary dark:border w-64 origin-top-right rounded-lg shadow-lg focus:outline-none py-1 text-xs"
		>
			<!-- Melt submenu: hover-opens and is floating-positioned (flips on screen edges). -->
			<DropdownSubmenuItem item={paramItems(close)} {builders} meltItem={item} />

			<div class="my-1 border-t border-border-light"></div>
			<div class="px-3 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">Model</div>
			<div class="max-h-48 overflow-y-auto">
				{#each models as m (m.provider + m.model)}
					<MenuItem
						{item}
						class="w-full flex items-center gap-2 px-3 py-1.5 text-left font-normal hover:bg-surface-hover data-[highlighted]:bg-surface-hover rounded-sm transition-colors cursor-pointer"
						onClick={() => selectModel(m)}
					>
						<span class="truncate grow min-w-0">{m.model}</span>
						{#if m.model === providerModel.model && m.provider === providerModel.provider}
							<Check size={14} class="shrink-0 text-primary" />
						{/if}
					</MenuItem>
				{/each}
			</div>

			<div class="my-1 border-t border-border-light"></div>
			{#if capability.supported}
				<!-- Registered as a melt item so it joins the roving focus/highlight (and arrow
				     up/down navigation), and so hovering it takes the highlight off the Parameters
				     trigger. Left/right adjust the effort; the slider's input handler also drives it. -->
				<MenuItemWrapper {item} onKeydown={adjustEffort} class="block group">
					<div class="px-3 pt-1 pb-0.5 flex items-center justify-between">
						<span class="text-2xs uppercase tracking-wide text-secondary">Thinking</span>
						<span class="text-2xs text-secondary tabular-nums">{currentStop}</span>
					</div>
					{#if stops.length > 1}
						<!-- Only the slider area reflects the item's highlight, not the header. -->
						<div
							class="px-3 py-1.5 rounded-sm transition-colors group-data-[highlighted]:bg-surface-hover"
						>
							<input
								type="range"
								min="0"
								max={stops.length - 1}
								step="1"
								value={stopIndex}
								style="--fill: {fillPct}%"
								oninput={(e) => selectReasoning(stops[+e.currentTarget.value])}
								use:isolatePointer
								class="lean-range no-default-style w-full"
								aria-label="Reasoning effort"
							/>
						</div>
					{/if}
				</MenuItemWrapper>
			{:else}
				<!-- Reasoning unsupported: keep the section but show it disabled with a reason,
				     rather than hiding it. Not a melt item, so it's skipped by keyboard navigation. -->
				<div class="px-3 pt-1 pb-1.5 opacity-60 cursor-default" aria-disabled="true">
					<div class="text-2xs uppercase tracking-wide text-secondary">Thinking</div>
					<div class="text-2xs text-tertiary mt-0.5">Not supported by this model</div>
				</div>
			{/if}
		</div>
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

<style>
	/* Lean reasoning slider: a thin track and a small, borderless accent thumb. Native range
	   thumbs can't be styled with Tailwind, and Svelte prunes scoped vendor pseudo-element
	   rules — so they are wrapped in :global (the class is unique to this component). */
	.lean-range {
		-webkit-appearance: none;
		appearance: none;
		height: 10px;
		margin: 0;
		padding: 0;
		/* override the global `input { background-color: ... !important }` so only the
		   thin track shows, not a full-height band behind it */
		background-color: transparent !important;
		cursor: pointer;
		outline: none;
	}
	.lean-range:focus,
	.lean-range:focus-visible {
		outline: none;
	}
	:global(.lean-range::-webkit-slider-runnable-track) {
		height: 3px;
		border-radius: 9999px;
		background: linear-gradient(
			to right,
			rgb(var(--color-surface-accent-primary)) var(--fill, 0%),
			rgb(var(--color-surface-secondary)) var(--fill, 0%)
		);
	}
	:global(.lean-range::-webkit-slider-thumb) {
		-webkit-appearance: none;
		appearance: none;
		margin-top: -3.5px;
		width: 10px;
		height: 10px;
		border: none;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
	:global(.lean-range::-moz-range-track) {
		height: 3px;
		border-radius: 9999px;
		background: rgb(var(--color-surface-secondary));
	}
	:global(.lean-range::-moz-range-progress) {
		height: 3px;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
	:global(.lean-range::-moz-range-thumb) {
		width: 10px;
		height: 10px;
		border: none;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
</style>
