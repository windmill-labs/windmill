<script lang="ts">
	import { Badge } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	export let selectedScriptName: string | undefined = undefined

	function selectInlineScript(name: string) {
		selectedScriptName = name
	}

	$: componentInlineScripts = $app.grid.reduce((acc, gridComponent) => {
		const componentInput: AppInput = gridComponent.data.componentInput

		if (componentInput.type === 'runnable') {
			if (componentInput.runnable?.type === 'runnableByName') {
				acc.push({
					name: componentInput.runnable.name,
					id: gridComponent.id
				})
			}
		}
		return acc
	}, [])
</script>

<PanelSection title="Inline scripts">
	<div class="flex flex-col gap-2 w-full">
		{#if componentInlineScripts.length > 0}
			<div class="flex gap-2 flex-col ">
				{#each componentInlineScripts as { name, id }, index}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class="{classNames(
							'border flex justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
							selectedScriptName === name ? 'bg-blue-100 text-blue-600' : ''
						)},"
						on:click={() => selectInlineScript(name)}
					>
						<span class="text-xs">{name}</span>
						{#if selectedScriptName === name}
							<Badge color="dark-indigo">{id}</Badge>
						{:else}
							<Badge color="red">Unused</Badge>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-sm text-gray-500">No inline scripts</div>
		{/if}
	</div>
</PanelSection>
