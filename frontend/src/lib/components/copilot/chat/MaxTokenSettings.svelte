<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { copilotMaxTokens, copilotSessionModel, copilotInfo } from '$lib/stores'
	import { getModelMaxTokens } from '../lib'
	import { storeLocalSetting } from '$lib/utils'

	let customMaxTokens = $state<string>('')

	// Reactive values
	const currentModel = $derived(
		$copilotSessionModel ?? $copilotInfo.defaultModel ?? $copilotInfo.aiModels[0]
	)
	const defaultMaxTokens = $derived.by(() =>
		getModelMaxTokens(currentModel.provider, currentModel.model)
	)
	const currentMaxTokens = $derived.by(() => $copilotMaxTokens[getModelKey()] ?? defaultMaxTokens)

	function getModelKey(): string {
		return `${currentModel.provider}:${currentModel.model}`
	}

	function resetToDefault() {
		const modelKey = getModelKey()
		const newSettings = { ...$copilotMaxTokens }
		delete newSettings[modelKey]
		$copilotMaxTokens = newSettings
		storeLocalSetting('copilotMaxTokens', JSON.stringify($copilotMaxTokens))
		customMaxTokens = ''
	}

	function saveMaxTokens() {
		const parsedTokens = parseInt(customMaxTokens)
		if (isNaN(parsedTokens) || parsedTokens < 1 || parsedTokens > 200000) {
			return
		}

		const modelKey = getModelKey()
		$copilotMaxTokens = {
			...$copilotMaxTokens,
			[modelKey]: parsedTokens
		}
		storeLocalSetting('copilotMaxTokens', JSON.stringify($copilotMaxTokens))
		customMaxTokens = ''
	}

	$effect(() => {
		// Initialize the input with current value when opened
		if (customMaxTokens === '') {
			customMaxTokens = currentMaxTokens.toString()
		}
	})

	$effect(() => {
		currentModel
		customMaxTokens = currentMaxTokens.toString()
	})
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	usePointerDownOutside
	contentClasses="flex flex-col gap-y-3 p-4 min-w-64"
>
	<svelte:fragment slot="trigger">
		<Button
			btnClasses="text-tertiary"
			color="light"
			size="xs2"
			nonCaptureEvent={true}
			startIcon={{ icon: Settings }}
			iconOnly
			title="Max tokens settings"
		/>
	</svelte:fragment>

	<svelte:fragment slot="content" let:close>
		<div class="flex flex-col gap-y-3">
			<div>
				<h3 class="font-semibold text-sm mb-2">Max Tokens</h3>
				<p class="text-xs text-secondary mb-2">
					Configure the maximum tokens for <span class="font-mono">{currentModel.model}</span>
				</p>
			</div>

			<div class="flex flex-col gap-2">
				<div class="flex gap-2">
					<input
						bind:value={customMaxTokens}
						type="number"
						min="1"
						max="200000"
						class="flex-1 px-2 py-1 text-xs border border-gray-200 dark:border-gray-700 rounded-md bg-surface focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
						placeholder="Enter max tokens"
					/>
				</div>
				<span class="text-xs text-tertiary">Current: {currentMaxTokens}</span>
				<span class="text-xs text-tertiary">Default: {defaultMaxTokens}</span>
			</div>

			<div class="flex gap-2">
				<Button
					size="xs"
					color="light"
					variant="border"
					on:click={resetToDefault}
					disabled={currentMaxTokens === defaultMaxTokens}
				>
					Reset to Default
				</Button>
				<Button
					size="xs"
					color="blue"
					on:click={() => {
						saveMaxTokens()
						close()
					}}>Save</Button
				>
			</div>
		</div>
	</svelte:fragment>
</Popover>
