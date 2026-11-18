<script lang="ts">
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	// Which drafts failed to deploy and why, anchored next to the deploy button so
	// the reasons are one click away without leaving the editor. Deployed drafts
	// leave the map; what is listed is still unresolved.
	let {
		errors,
		open = $bindable()
	}: {
		errors: ReadonlyMap<string, string>
		open?: boolean
	} = $props()
</script>

<Popover
	placement="bottom-end"
	contentClasses="p-3 max-w-[480px]"
	usePointerDownOutside
	enableFlyTransition
	bind:isOpen={open}
>
	{#snippet trigger()}
		<button
			type="button"
			class="flex items-center gap-1.5 px-2 py-1 rounded-md text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/30 hover:bg-red-100 dark:hover:bg-red-900/50 transition-colors text-xs font-medium"
			title="View save errors"
		>
			<AlertTriangle size={14} />
			<span>{errors.size} failed</span>
		</button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-2">
			<span class="text-xs font-semibold text-emphasis">Save errors</span>
			<div class="flex flex-col gap-2 max-h-72 overflow-y-auto">
				{#each [...errors.entries()] as [path, message] (path)}
					<div class="flex flex-col gap-0.5 border-l-2 border-red-400 pl-2">
						<span class="text-2xs font-mono text-emphasis">{path}</span>
						<span class="text-2xs text-red-600 dark:text-red-400 break-words">
							{message}
						</span>
					</div>
				{/each}
			</div>
		</div>
	{/snippet}
</Popover>
