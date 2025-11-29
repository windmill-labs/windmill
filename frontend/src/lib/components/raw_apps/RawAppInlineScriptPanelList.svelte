<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plus } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'

	import type { Runnable } from '../apps/inputType'
	import { getNextId } from '$lib/components/flows/idUtils'

	interface Props {
		selectedRunnable: string | undefined
		runnables: Record<string, Runnable>
	}

	let { selectedRunnable = $bindable(), runnables }: Props = $props()

	function createBackgroundScript() {
		const nid = getNextId(Object.keys(runnables ?? {}))
		const newScriptPath = `Backend Runnable ${nid}`
		runnables[nid] = {
			name: newScriptPath,
			inlineScript: undefined,
			type: 'inline'
		}

		selectedRunnable = nid
	}
</script>

<PanelSection size="lg" fullHeight={false} title="Backend" id="app-editor-runnable-panel">
	{#snippet action()}
		<div class="flex flex-row gap-1">
			<Button
				size="xs"
				variant="default"
				btnClasses="!rounded-full !p-1"
				title="Create a new background runnable"
				aria-label="Create a new background runnable"
				on:click={createBackgroundScript}
				id="create-background-runnable"
			>
				<Plus size={14} class="!text-primary" />
			</Button>
		</div>
	{/snippet}
	<div class="w-full flex flex-col gap-6 py-1">
		<div>
			<div class="flex flex-col gap-1 w-full">
				{#if Object.keys(runnables ?? {}).length > 0}
					{#each Object.entries(runnables ?? {}) as [id, runnable]}
						{#if runnable}
							<button
								{id}
								class="panel-item
								{selectedRunnable === id
									? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
									: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
								onclick={() => (selectedRunnable = id)}
							>
								<span class="text-2xs truncate">{runnable?.name}</span>
								<Badge color="indigo">{id}</Badge>
							</button>
						{/if}
					{/each}
				{:else}
					<div class="text-xs text-primary">No backend runnable</div>
				{/if}
			</div>
		</div>
	</div>
</PanelSection>
