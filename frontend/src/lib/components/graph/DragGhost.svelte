<script lang="ts">
	import type { DragManager } from './dragManager.svelte'
	import { NODE } from './util'

	let { dragManager }: { dragManager: DragManager } = $props()
</script>

{#if dragManager.dragging}
	<div
		class="fixed pointer-events-none z-[10000]"
		style="left: {dragManager.ghostScreenX}px; top: {dragManager.ghostScreenY}px; transform: translate(-50%, -50%); width: {NODE.width}px;"
	>
		<div
			class="rounded-md bg-surface-tertiary shadow-lg border border-border-selected opacity-70 px-3 py-1.5 flex items-center gap-2 text-sm text-primary truncate"
		>
			<span class="font-medium truncate">{dragManager.dragging.label}</span>
			<span
				class="ml-auto text-2xs bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 px-1.5 py-0.5 rounded font-mono shrink-0"
			>
				{dragManager.dragging.moduleId}
			</span>
		</div>
	</div>
{/if}
