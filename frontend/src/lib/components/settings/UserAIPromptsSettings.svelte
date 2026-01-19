<script lang="ts">
	import { storeLocalSetting } from '$lib/utils'
	import CustomAIPrompts from '../copilot/CustomAIPrompts.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { getUserCustomPrompts } from '$lib/aiStore'
	import Section from '../Section.svelte'

	const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

	let initialPrompts = getUserCustomPrompts()
	let customPrompts = $state<Record<string, string>>({ ...initialPrompts })

	function save() {
		storeLocalSetting(USER_CUSTOM_PROMPTS_KEY, JSON.stringify(customPrompts))
		initialPrompts = { ...customPrompts }
		sendUserToast('User AI prompts saved')
	}

	let hasPrompts = $derived(Object.values(customPrompts).some((p) => p?.trim().length > 0))

	let hasChanges = $derived(
		Object.keys(customPrompts).length !== Object.keys(initialPrompts).length ||
			Object.keys(customPrompts).some((key) => customPrompts[key] !== initialPrompts[key])
	)
</script>

<div class="mt-4">
	<Section label="Custom system prompts" collapsable animate>
		{#snippet header()}
			<div class="w-2 h-2 bg-blue-500 rounded-full ml-2 {hasPrompts ? '' : 'hidden'}"></div>
		{/snippet}
		<div>
			<CustomAIPrompts
				bind:customPrompts
				description="These prompts are stored locally in your browser and apply in addition to the workspace-level prompts."
			/>
			<div class="flex flex-row justify-end mt-2 gap-2">
				<Button onclick={save} disabled={!hasChanges} variant="accent">Save custom prompts</Button>
			</div>
		</div>
	</Section>
</div>
