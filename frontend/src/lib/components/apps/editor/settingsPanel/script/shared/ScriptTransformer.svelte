<script lang="ts">
	import type { AppEditorContext, InlineScript } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { Button } from '$lib/components/common'
	import { Plus, X } from 'lucide-svelte'
	import Section from '$lib/components/Section.svelte'

	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	export let appInput: { transformer?: InlineScript & { language: 'frontend' } }
	export let id: string

	$: checked = Boolean(appInput.transformer)
</script>

<div class="mt-2">
	<Section
		label="Transformer"
		tooltip={"A transformer is an optional frontend script that is executed right after the component's script whose purpose is to do lightweight transformation in the browser. It takes the previous computation's result as `result`"}
	>
		<svelte:fragment slot="action">
			<Button
				size="xs"
				color={checked ? 'red' : 'light'}
				variant="border"
				on:click={() => {
					if (appInput.transformer) {
						appInput.transformer = undefined
					} else {
						appInput.transformer = {
							language: 'frontend',
							content: 'return result'
						}
						$selectedComponentInEditor = id + '_transformer'
					}
				}}
				startIcon={{ icon: checked ? X : Plus }}
			>
				{checked ? 'Remove' : 'Add'}
			</Button>
		</svelte:fragment>
	</Section>
</div>
