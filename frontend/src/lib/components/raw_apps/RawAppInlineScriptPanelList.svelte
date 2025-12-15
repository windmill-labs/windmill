<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plus, AlertCircle, AlertTriangle } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import Popover from '../Popover.svelte'

	import type { Runnable } from '../apps/inputType'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { rawAppLintStore } from './lintStore'

	interface Props {
		selectedRunnable: string | undefined
		runnables: Record<string, Runnable>
	}

	let { selectedRunnable = $bindable(), runnables }: Props = $props()

	// Subscribe to lint store for reactive updates
	let lintSnapshot = $state(rawAppLintStore.getSnapshot())

	$effect(() => {
		const unsubscribe = rawAppLintStore.subscribe((s) => {
			lintSnapshot = s
		})
		return unsubscribe
	})

	const errorCount = $derived(
		Object.values(lintSnapshot.errors).reduce((acc, arr) => acc + arr.length, 0)
	)
	const warningCount = $derived(
		Object.values(lintSnapshot.warnings).reduce((acc, arr) => acc + arr.length, 0)
	)

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

<PanelSection size="lg" fullHeight={false} title="backend" id="app-editor-runnable-panel">
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

			{#if errorCount > 0 || warningCount > 0}
				<Popover notClickable placement="right">
					<div
						class="flex items-center gap-2 px-2 py-1.5 mt-2 rounded border text-xs
						{errorCount > 0
							? 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
							: 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800'}"
					>
						{#if errorCount > 0}
							<span class="flex items-center gap-1 text-red-600 dark:text-red-400">
								<AlertCircle size={14} />
								{errorCount} error{errorCount !== 1 ? 's' : ''}
							</span>
						{/if}
						{#if warningCount > 0}
							<span class="flex items-center gap-1 text-yellow-600 dark:text-yellow-400">
								<AlertTriangle size={14} />
								{warningCount} warning{warningCount !== 1 ? 's' : ''}
							</span>
						{/if}
					</div>
					{#snippet text()}
						<div class="max-w-md max-h-80 overflow-auto text-xs">
							{#if errorCount > 0}
								<div class="mb-2">
									<div class="font-semibold text-red-600 dark:text-red-400 mb-1">Errors</div>
									{#each Object.entries(lintSnapshot.errors) as [key, errors]}
										{#if errors.length > 0}
											<div class="mb-1">
												<span class="font-medium text-secondary">{key}:</span>
												<ul class="ml-2 list-disc list-inside">
													{#each errors as error}
														<li class="text-tertiary">
															Line {error.startLineNumber}: {error.message}
														</li>
													{/each}
												</ul>
											</div>
										{/if}
									{/each}
								</div>
							{/if}
							{#if warningCount > 0}
								<div>
									<div class="font-semibold text-yellow-600 dark:text-yellow-400 mb-1">Warnings</div
									>
									{#each Object.entries(lintSnapshot.warnings) as [key, warnings]}
										{#if warnings.length > 0}
											<div class="mb-1">
												<span class="font-medium text-secondary">{key}:</span>
												<ul class="ml-2 list-disc list-inside">
													{#each warnings as warning}
														<li class="text-tertiary">
															Line {warning.startLineNumber}: {warning.message}
														</li>
													{/each}
												</ul>
											</div>
										{/if}
									{/each}
								</div>
							{/if}
						</div>
					{/snippet}
				</Popover>
			{/if}
		</div>
	</div>
</PanelSection>
