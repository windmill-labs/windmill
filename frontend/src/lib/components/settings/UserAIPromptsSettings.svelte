<script lang="ts">
	import { storeLocalSetting } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { getUserCustomPrompts } from '$lib/aiStore'
	import Label from '../Label.svelte'
	import AIPromptsModal from './AIPromptsModal.svelte'
	import { ExternalLink, Settings } from 'lucide-svelte'
	import { userStore } from '$lib/stores'

	const USER_CUSTOM_PROMPTS_KEY = 'userCustomAIPrompts'

	const userCustomPrompts = getUserCustomPrompts()

	let initialPrompts = $state({ ...userCustomPrompts })
	let customPrompts = $state<Record<string, string>>({ ...userCustomPrompts })
	let modalOpen = $state(false)

	function save() {
		storeLocalSetting(USER_CUSTOM_PROMPTS_KEY, JSON.stringify(customPrompts))
		initialPrompts = { ...customPrompts }
		sendUserToast('User AI prompts saved')
	}

	function reset() {
		customPrompts = { ...initialPrompts }
		sendUserToast('Reset to last saved state')
	}

	let hasPrompts = $derived(Object.values(customPrompts).some((p) => p?.trim().length > 0))

	let promptCount = $derived(
		Object.values(customPrompts).filter((p) => p?.trim().length > 0).length
	)

	let hasChanges = $derived(
		Array.from(new Set([...Object.keys(customPrompts), ...Object.keys(initialPrompts)])).some(
			(key) => {
				const currentValue = customPrompts[key] || ''
				const initialValue = initialPrompts[key] || ''
				return currentValue !== initialValue
			}
		)
	)
</script>

<div class="mt-4">
	<Label label="Custom system prompts">
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Customize AI behavior with system prompts. These are stored locally in your browser and
				apply in addition to {#if $userStore?.is_admin || $userStore?.is_super_admin}<a
						href="workspace_settings?tab=ai#"
						>workspace-level prompts <ExternalLink size={12} class="inline-block" /></a
					>{:else}workspace-level prompts{/if}.
			</p>

			<div class="flex justify-between items-center">
				<div class="flex items-center gap-2">
					<Button
						onclick={() => (modalOpen = true)}
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Settings }}
					>
						Configure AI prompts
					</Button>
					{#if hasPrompts}
						<span class="text-xs text-secondary">({promptCount} configured)</span>
					{/if}
				</div>

				{#if hasChanges}
					<span class="text-xs text-yellow-600 dark:text-yellow-400"> Unsaved changes </span>
				{/if}
			</div>
		</div>
	</Label>
</div>

<AIPromptsModal
	bind:open={modalOpen}
	bind:customPrompts
	onSave={save}
	onReset={reset}
	{hasChanges}
/>
