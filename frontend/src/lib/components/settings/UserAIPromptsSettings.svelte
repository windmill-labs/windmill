<script lang="ts">
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import CustomAIPrompts from '../copilot/CustomAIPrompts.svelte'
	import Button from '../common/button/Button.svelte'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

	let customPrompts = $state<Record<string, string>>({})
	let isExpanded = $state(false)

	$effect(() => {
		loadPrompts()
	})

	function loadPrompts() {
		const stored = getLocalSetting(USER_CUSTOM_PROMPTS_KEY)
		if (stored) {
			try {
				customPrompts = JSON.parse(stored)
			} catch (e) {
				console.error('Failed to parse user custom prompts', e)
				customPrompts = {}
			}
		} else {
			customPrompts = {}
		}
	}

	function save() {
		storeLocalSetting(USER_CUSTOM_PROMPTS_KEY, JSON.stringify(customPrompts))
		sendUserToast('User AI prompts saved')
	}

	let hasPrompts = $derived(Object.values(customPrompts).some((p) => p?.trim().length > 0))
</script>

<div class="mt-4">
	<button
		type="button"
		class="flex items-center border-b cursor-pointer hover:bg-surface-hover w-full transition-colors"
		onclick={() => (isExpanded = !isExpanded)}
	>
		{#if isExpanded}
			<ChevronDown size={16} />
		{:else}
			<ChevronRight size={16} />
		{/if}
		<h2>Custom system prompts</h2>
		{#if hasPrompts}
			<div class="w-2 h-2 bg-blue-500 rounded-full"></div>
		{/if}
	</button>

	{#if isExpanded}
		<div>
			<CustomAIPrompts bind:customPrompts />
			<div class="flex flex-row justify-end mt-2">
				<Button onclick={save}>Save</Button>
			</div>
		</div>
	{/if}
</div>
