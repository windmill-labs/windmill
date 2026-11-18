<script lang="ts">
	import { fly } from 'svelte/transition'
	import { CheckCircle2 } from 'lucide-svelte'

	interface Props {
		/** Sized by the row, not the caller: 20px is what an integration logo needs to stay legible. */
		icon: import('svelte').Snippet
		title: import('svelte').Snippet
		detail?: import('svelte').Snippet
		action: import('svelte').Snippet
		/** Rendered full-width under the row, for detail that does not fit on one line. */
		extra?: import('svelte').Snippet
		/** Plays the confirmation flash over the action once. */
		flash?: boolean
	}

	let { icon, title, detail, action, extra, flash = false }: Props = $props()
</script>

<!-- One row for both lists on the setup step. Data tables and credentials are the same
     thing to the reader — something the import could not configure, with an action that
     configures it — so they get the same icon size, spacing and text block. -->
<li class="flex flex-col rounded-md border border-border-light px-3 py-2 text-xs">
	<div class="flex items-center gap-3">
		<div class="shrink-0">{@render icon()}</div>
		<!-- No gap and no leading override: `text-xs` already carries `leading-4`, and the two
	     lines are one block of text, not two stacked items. -->
		<div class="flex min-w-0 flex-1 flex-col">
			{@render title()}
			{@render detail?.()}
		</div>
		<!-- The confirmation flash is the one from SaveButton: the work itself happens
	     elsewhere — a drawer, a wizard — so only the overlay is reused here. The button
	     stays live underneath either way; being configured is a state, not a dead end. -->
		<div class="relative shrink-0 overflow-hidden rounded-md">
			{@render action()}
			{#if flash}
				<div
					class="absolute inset-0 flex items-center justify-center rounded-md bg-green-200 dark:bg-green-800"
					transition:fly={{ y: -10, duration: 300 }}
				>
					<CheckCircle2 class="h-5 w-5 text-green-700 dark:text-green-300" />
				</div>
			{/if}
		</div>
	</div>
	{@render extra?.()}
</li>
