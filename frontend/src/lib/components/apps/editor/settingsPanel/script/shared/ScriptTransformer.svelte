<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import type { AppEditorContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'
	import type { AppComponent } from '../../../component'

	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	export let appInput: ResultAppInput
	export let appComponent: AppComponent
</script>

<ScriptSettingsSection
	title="Transformer"
	tooltip={"A transformer is an optional frontend script that is executed right after the component's script whose purpose is to do lightweight transformation in the browser. It takes the previous computation's result as `result`"}
>
	<div class="flex gap-1 justify-between items-center">
		<span class="text-xs truncate"> Add a transformer </span>
		<div class="flex gap-1">
			<input
				checked={Boolean(appInput.transformer)}
				type="checkbox"
				class="!w-4 !h-4 !rounded-sm"
				on:click={() => {
					if (appInput.transformer) {
						appInput.transformer = undefined
					} else {
						appInput.transformer = {
							language: 'frontend',
							content: 'return result'
						}
						$selectedComponentInEditor = appComponent.id + '_transformer'
					}
				}}
			/>
		</div>
	</div>
</ScriptSettingsSection>
