<script lang="ts">
	import { storeLocalSetting } from '$lib/utils'
	import CustomAIPrompts from '../copilot/CustomAIPrompts.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { getUserCustomPrompts } from '$lib/aiStore'
	import Section from '../Section.svelte'

	const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

	let customPrompts = $state<Record<string, string>>(getUserCustomPrompts())

	function save() {
		storeLocalSetting(USER_CUSTOM_PROMPTS_KEY, JSON.stringify(customPrompts))
		sendUserToast('User AI prompts saved')
	}

	let hasPrompts = $derived(Object.values(customPrompts).some((p) => p?.trim().length > 0))
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
			<div class="flex flex-row justify-end mt-2">
				<Button onclick={save}>Save</Button>
			</div>
		</div>
	</Section>
</div>
