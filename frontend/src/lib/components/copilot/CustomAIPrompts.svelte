<script lang="ts">
	import { AIMode } from './chat/AIChatManager.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Label from '../Label.svelte'
	import autosize from '$lib/autosize'

	const MAX_CUSTOM_PROMPT_LENGTH = 5000

	let {
		customPrompts = $bindable(),
		title,
		description
	}: {
		customPrompts: Record<string, string>
		title?: string
		description?: string
	} = $props()

	let selectedAiMode = $state<AIMode>(AIMode.ASK)
</script>

<div class="flex flex-col gap-2">
	<p class="font-semibold">{title}</p>
	{#if description}
		<p class="text-xs text-secondary">{description}</p>
	{/if}
	<div class="flex flex-col gap-4">
		<Label label="AI Mode">
			<ToggleButtonGroup bind:selected={selectedAiMode}>
				{#snippet children({ item })}
					{#each Object.values(AIMode) as mode}
						<div class="relative">
							<ToggleButton
								value={mode}
								label={mode.charAt(0).toUpperCase() + mode.slice(1)}
								{item}
							/>
							{#if customPrompts[mode]?.length > 0}
								<div
									class="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full border border-surface"
								></div>
							{/if}
						</div>
					{/each}
				{/snippet}
			</ToggleButtonGroup>
		</Label>

		<Label
			label="Custom system prompt for {selectedAiMode.charAt(0).toUpperCase() +
				selectedAiMode.slice(1)} Mode"
		>
			<textarea
				bind:value={customPrompts[selectedAiMode]}
				placeholder="Enter a custom system prompt for {selectedAiMode} mode."
				class="w-full min-h-24 p-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary resize-y"
				rows="4"
				maxlength={MAX_CUSTOM_PROMPT_LENGTH}
				use:autosize
			></textarea>
			<div class="flex justify-end mt-1">
				<span class="text-xs text-secondary">
					{(customPrompts[selectedAiMode] ?? '').length}/{MAX_CUSTOM_PROMPT_LENGTH} characters
				</span>
			</div>
		</Label>
	</div>
</div>
