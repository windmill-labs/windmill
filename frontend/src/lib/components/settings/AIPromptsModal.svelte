<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Button from '../common/button/Button.svelte'
	import Label from '../Label.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { AIMode, getVisibleAIModes } from '../copilot/chat/AIChatManager.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { Alert } from '../common'

	const MAX_CUSTOM_PROMPT_LENGTH = 5000

	interface Props {
		open?: boolean
		customPrompts: Record<string, string>
		onSave?: () => void | Promise<void>
		onReset: () => void
		hasChanges: boolean
		scope?: 'user' | 'workspace' | 'instance'
		// Restrict the editor to a subset of modes (defaults to all visible modes)
		modes?: AIMode[]
		// Render the prompts read-only: inputs disabled, no Save/Reset, empty modes hidden
		readOnly?: boolean
		// Custom explanation shown in the read-only Alert (overrides the default text)
		readOnlyReason?: string
		title?: string
		// Portal target for the modal (defaults to #content, present on settings pages)
		target?: string
		// When set, render a link to the full AI settings page in the footer
		settingsHref?: string
		fixedHeight?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl'
	}

	let {
		open = $bindable(false),
		customPrompts = $bindable(),
		onSave,
		onReset,
		hasChanges,
		scope = 'user',
		modes = undefined,
		readOnly = false,
		readOnlyReason = undefined,
		title = 'Customize AI System Prompts',
		target = '#content',
		settingsHref = undefined,
		fixedHeight = 'xxl'
	}: Props = $props()

	const placeholders: Record<AIMode, string> = {
		[AIMode.SCRIPT]: 'Enter custom instructions for script generation and editing',
		[AIMode.FLOW]: 'Enter custom instructions for workflow creation and automation',
		[AIMode.APP]: 'Enter custom instructions for UI and app development',
		[AIMode.NAVIGATOR]: 'Enter custom instructions for navigation and guidance',
		[AIMode.API]: 'Enter custom instructions for API interactions and integrations',
		[AIMode.GLOBAL]: 'Enter custom instructions for workspace-wide draft assistance',
		[AIMode.ASK]: 'Enter custom instructions for general questions and assistance'
	}

	const modeLabels: Record<AIMode, string> = {
		[AIMode.SCRIPT]: 'Script Mode',
		[AIMode.FLOW]: 'Flow Mode',
		[AIMode.APP]: 'App Mode',
		[AIMode.NAVIGATOR]: 'Navigator Mode',
		[AIMode.API]: 'API Mode',
		[AIMode.GLOBAL]: 'Global Mode',
		[AIMode.ASK]: 'Ask Mode'
	}

	let visibleModes = $derived(modes ?? getVisibleAIModes())
	// In read-only mode, only show modes that actually have a prompt configured
	let displayModes = $derived(
		readOnly
			? visibleModes.filter((mode) => (customPrompts[mode] ?? '').trim().length > 0)
			: visibleModes
	)
	// A single prompt (e.g. the global-chat quick settings) doesn't need per-mode framing
	let singleMode = $derived(displayModes.length === 1)

	let saving = $state(false)

	async function handleSave() {
		if (saving) return
		saving = true
		try {
			// onSave may be async (e.g. the workspace round-trip); await it so the modal stays
			// open while the save is in flight and only closes on success. A failing onSave is
			// expected to surface its own toast and throw, leaving the modal open.
			await onSave?.()
			open = false
		} catch {
			// keep the modal open; onSave already reported the error
		} finally {
			saving = false
		}
	}

	function handleReset() {
		onReset()
	}
</script>

<Modal2 bind:isOpen={open} {title} fixedWidth="md" {fixedHeight} {target}>
	<div class="flex flex-col gap-6 h-full px-1 w-full">
		<div class="grow min-h-0 overflow-y-auto" style="scrollbar-gutter: stable;">
			{#if readOnly}
				<div class="mb-6">
					<Alert type="info" title="Read-only" size="xs">
						{#if readOnlyReason}
							{readOnlyReason}
						{:else if scope === 'workspace'}
							Only workspace admins can edit the workspace AI prompt. It applies to all workspace
							members.
						{:else}
							This prompt is read-only.
						{/if}
					</Alert>
				</div>
			{:else}
				<div class="text-xs text-secondary mb-6">
					{#if scope === 'workspace'}
						{#if singleMode}
							Customize the workspace system prompt. It applies to all workspace members.
						{:else}
							Customize the system prompts for each AI mode. These prompts apply to all workspace
							members.
						{/if}
					{:else if scope === 'instance'}
						{#if singleMode}
							Customize the system prompt. It applies to workspaces using instance AI defaults.
						{:else}
							Customize the system prompts for each AI mode. These prompts apply to workspaces using
							instance AI defaults.
						{/if}
					{:else if singleMode}
						Customize your system prompt. It is stored locally in your browser and applies in
						addition to the workspace-level prompt.
					{:else}
						Customize the system prompts for each AI mode. These prompts are stored locally in your
						browser and apply in addition to workspace-level prompts.
					{/if}
				</div>
			{/if}

			{#if readOnly && displayModes.length === 0}
				<div class="text-xs text-secondary">No workspace prompt configured.</div>
			{/if}
			{#each displayModes as mode (mode)}
				<div class="flex flex-col gap-2 pb-4 last:border-b-0">
					<Label
						label={modeLabels[mode]}
						for={`custom-prompt-${mode}`}
						headless={displayModes.length === 1}
					>
						<TextInput
							bind:value={customPrompts[mode]}
							underlyingInputEl="textarea"
							inputProps={{
								placeholder: placeholders[mode],
								rows: 3,
								maxlength: MAX_CUSTOM_PROMPT_LENGTH,
								id: `custom-prompt-${mode}`,
								readonly: readOnly
							}}
							class="min-h-12 resize-y"
							size="sm"
						/>
						{#if !readOnly}
							<div class="flex justify-end mt-1">
								<span class="text-2xs text-hint">
									{(customPrompts[mode] ?? '').length}/{MAX_CUSTOM_PROMPT_LENGTH} characters
								</span>
							</div>
						{/if}
					</Label>
				</div>
			{/each}
		</div>

		{#if !readOnly}
			<div class="flex justify-between items-center gap-2">
				<div>
					{#if settingsHref}
						<Button
							href={settingsHref}
							target="_blank"
							variant="subtle"
							size="sm"
							endIcon={{ icon: ExternalLink }}
						>
							AI settings
						</Button>
					{/if}
				</div>
				<div class="flex gap-2">
					<Button
						size="sm"
						variant="default"
						disabled={!hasChanges || saving}
						onclick={handleReset}
					>
						Reset
					</Button>
					{#if onSave}
						<Button
							size="sm"
							variant="accent"
							disabled={!hasChanges}
							loading={saving}
							onclick={handleSave}
						>
							Save Prompts
						</Button>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</Modal2>
