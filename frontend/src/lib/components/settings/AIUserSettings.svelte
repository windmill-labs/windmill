<script lang="ts">
	import {
		codeCompletionSessionEnabled,
		metadataCompletionEnabled,
		stepInputCompletionEnabled
	} from '$lib/stores'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import Toggle from '../Toggle.svelte'
	import type { Writable } from 'svelte/store'
	import UserAIPromptsSettings from './UserAIPromptsSettings.svelte'

	$effect(() => {
		loadSettings()
	})

	function loadSettings() {
		$codeCompletionSessionEnabled =
			(getLocalSetting('codeCompletionSessionEnabled') ?? 'true') == 'true'
		$metadataCompletionEnabled = (getLocalSetting('metadataCompletionEnabled') ?? 'true') == 'true'
		$stepInputCompletionEnabled =
			(getLocalSetting('stepInputCompletionEnabled') ?? 'true') == 'true'
	}

	function updateSetting(store: Writable<boolean>, value: boolean, setting: string) {
		store.set(value)
		storeLocalSetting(setting, value.toString())
	}
</script>

<div class="border border-border-light rounded-md p-4 h-full">
	<h2 class="text-emphasis text-sm font-semibold mb-2">AI user settings</h2>

	<div class="flex flex-col gap-4">
		<Toggle
			on:change={(e) => {
				updateSetting(codeCompletionSessionEnabled, e.detail, 'codeCompletionSessionEnabled')
			}}
			checked={$codeCompletionSessionEnabled}
			options={{
				right: 'Code completion',
				rightTooltip: 'AI completion in the code editors'
			}}
		/>

		<Toggle
			on:change={(e) => {
				updateSetting(metadataCompletionEnabled, e.detail, 'metadataCompletionEnabled')
			}}
			checked={$metadataCompletionEnabled}
			options={{
				right: 'Metadata completion',
				rightTooltip: 'AI completion for summaries and descriptions'
			}}
		/>
		<Toggle
			on:change={(e) => {
				updateSetting(stepInputCompletionEnabled, e.detail, 'stepInputCompletionEnabled')
			}}
			checked={$stepInputCompletionEnabled}
			options={{
				right: 'Flow step input completion',
				rightTooltip: 'AI completion for flow step inputs'
			}}
		/>
	</div>

	<UserAIPromptsSettings />
</div>
