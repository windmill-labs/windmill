<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Button from '../common/button/Button.svelte'
	import Label from '../Label.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { AIMode } from '../copilot/chat/AIChatManager.svelte'

	const MAX_CUSTOM_PROMPT_LENGTH = 5000

	interface Props {
		open?: boolean
		customPrompts: Record<string, string>
		onSave?: () => void
		onReset: () => void
		hasChanges: boolean
		isWorkspaceSettings?: boolean
	}

	let {
		open = $bindable(false),
		customPrompts = $bindable(),
		onSave,
		onReset,
		hasChanges,
		isWorkspaceSettings = false
	}: Props = $props()

	const placeholders: Record<AIMode, string> = {
		[AIMode.SCRIPT]: 'Enter custom instructions for script generation and editing',
		[AIMode.FLOW]: 'Enter custom instructions for workflow creation and automation',
		[AIMode.APP]: 'Enter custom instructions for UI and app development',
		[AIMode.NAVIGATOR]: 'Enter custom instructions for navigation and guidance',
		[AIMode.API]: 'Enter custom instructions for API interactions and integrations',
		[AIMode.ASK]: 'Enter custom instructions for general questions and assistance'
	}

	const modeLabels: Record<AIMode, string> = {
		[AIMode.SCRIPT]: 'Script Mode',
		[AIMode.FLOW]: 'Flow Mode',
		[AIMode.APP]: 'App Mode',
		[AIMode.NAVIGATOR]: 'Navigator Mode',
		[AIMode.API]: 'API Mode',
		[AIMode.ASK]: 'Ask Mode'
	}

	function handleSave() {
		onSave?.()
		open = false
	}

	function handleReset() {
		onReset()
	}
</script>

<Modal2
	bind:isOpen={open}
	title="Customize AI System Prompts"
	fixedWidth="md"
	fixedHeight="xxl"
	target="#content"
>
	<div class="flex flex-col gap-6 h-full px-1 w-full">
		<div class="grow min-h-0 overflow-y-auto" style="scrollbar-gutter: stable;">
			<div class="text-xs text-secondary mb-6">
				{#if isWorkspaceSettings}
					Customize the system prompts for each AI mode. These prompts apply to all workspace
					members.
				{:else}
					Customize the system prompts for each AI mode. These prompts are stored locally in your
					browser and apply in addition to workspace-level prompts.
				{/if}
			</div>

			{#each Object.values(AIMode) as mode}
				<div class="flex flex-col gap-2 pb-4 last:border-b-0">
					<Label label={modeLabels[mode]} for={`custom-prompt-${mode}`}>
						<TextInput
							bind:value={customPrompts[mode]}
							underlyingInputEl="textarea"
							inputProps={{
								placeholder: placeholders[mode],
								rows: 3,
								maxlength: MAX_CUSTOM_PROMPT_LENGTH,
								id: `custom-prompt-${mode}`
							}}
							class="min-h-12 resize-y"
							size="sm"
						/>
						<div class="flex justify-end mt-1">
							<span class="text-2xs text-hint">
								{(customPrompts[mode] ?? '').length}/{MAX_CUSTOM_PROMPT_LENGTH} characters
							</span>
						</div>
					</Label>
				</div>
			{/each}
		</div>

		<div class="flex justify-end gap-2">
			<Button size="sm" variant="default" disabled={!hasChanges} onclick={handleReset}>
				Reset
			</Button>
			{#if onSave}
				<Button size="sm" variant="accent" disabled={!hasChanges} onclick={handleSave}>
					Save Prompts
				</Button>
			{/if}
		</div>
	</div>
</Modal2>
