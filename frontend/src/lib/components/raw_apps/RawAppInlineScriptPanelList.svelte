<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plus } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import DocLink from '../apps/editor/settingsPanel/DocLink.svelte'
	import HideButton from '../apps/editor/settingsPanel/HideButton.svelte'
	import type { Writable } from 'svelte/store'
	import type { Runnable } from '../apps/inputType'
	import { getNextId } from '$lib/components/flows/idUtils'

	export let selectedRunnable: string | undefined
	export let runnables: Writable<Record<string, Runnable>>

	function createBackgroundScript() {
		const nid = getNextId(Object.keys($runnables ?? {}))
		const newScriptPath = `Backend Runnable ${nid}`
		runnables.update((r) => {
			r[nid] = {
				name: newScriptPath,
				inlineScript: undefined,
				type: 'runnableByName'
			}
			return r
		})
		console.log('BAR 2')
		selectedRunnable = nid
	}

	const dispatch = createEventDispatcher()
</script>

<PanelSection title="Backend Runnables" id="app-editor-runnable-panel">
	<svelte:fragment slot="action">
		<div class="flex flex-row gap-1">
			<HideButton
				direction="bottom"
				on:click={() => {
					dispatch('hidePanel')
				}}
			/>
			<DocLink
				docLink="https://www.windmill.dev/docs/apps/app-runnable-panel#creating-a-runnable"
			/>
			<Button
				size="xs"
				color="light"
				variant="border"
				btnClasses="!rounded-full !p-1"
				title="Create a new background runnable"
				aria-label="Create a new background runnable"
				on:click={createBackgroundScript}
				id="create-background-runnable"
			>
				<Plus size={14} class="!text-primary" />
			</Button>
		</div>
	</svelte:fragment>
	<div class="w-full flex flex-col gap-6 py-1">
		<div>
			<div class="flex flex-col gap-1 w-full">
				{#if Object.keys($runnables ?? {}).length > 0}
					{#each Object.entries($runnables ?? {}) as [id, runnable]}
						{#if runnable}
							<button
								{id}
								class="panel-item
								{selectedRunnable === id
									? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
									: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
								on:click={() => (selectedRunnable = id)}
							>
								<span class="text-2xs truncate">{runnable?.name}</span>
								<Badge color="indigo">{id}</Badge>
							</button>
						{/if}
					{/each}
				{:else}
					<div class="text-xs text-tertiary">No backend runnable</div>
				{/if}
			</div>
		</div>
	</div>
</PanelSection>

<style lang="postcss">
	.panel-item {
		@apply border flex gap-1 truncate font-normal justify-between w-full items-center py-1 px-2 rounded-sm duration-200;
	}
</style>
