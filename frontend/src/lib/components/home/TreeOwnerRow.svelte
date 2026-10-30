<script lang="ts">
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { inUnruledList, rowRule } from '../common/table/unruledList'

	const unruled = inUnruledList()

	/** A row of the tree that groups others under it (a folder, a user, "Other users"): clicking
	 * anywhere on it opens or closes what it groups. */
	interface Props {
		icon: typeof import('lucide-svelte').Folder
		name: string
		opened: boolean
		onToggle: () => void
		/** Extra left padding in px, for a nested folder. */
		indent?: number
		/** The toggle's accessible name; the row's name when unset. */
		toggleLabel?: string
		/** The line under the name, usually its item count. */
		detail: Snippet
		/** Controls rendered next to the name. Clicks on them do not toggle the row. */
		actions?: Snippet
	}

	let {
		icon: Icon,
		name,
		opened,
		onToggle,
		indent = 0,
		toggleLabel,
		detail,
		actions
	}: Props = $props()
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	onclick={onToggle}
	class="px-4 py-2 {rowRule(
		unruled
	)} group/row w-full flex flex-row items-center justify-between cursor-pointer hover:bg-surface-hover"
>
	<div
		class="flex flex-row items-center gap-4"
		style={indent > 0 ? `padding-left: ${indent}px;` : ''}
	>
		<Icon size={16} class="text-secondary shrink-0" />
		<div>
			<div class="flex flex-row items-center gap-1">
				<span class="whitespace-nowrap text-xs text-emphasis font-semibold">{name}</span>
				{#if actions}
					<!-- Shown for the hovered row, a tabbed-into one, or one whose menu is open.
					     `-my-0.5` keeps the 20px trigger from making this line taller than the
					     name alone, so rows with and without actions line up. -->
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="-my-0.5 opacity-0 transition-opacity duration-100 group-hover/row:opacity-100 group-has-[:focus-visible]/row:opacity-100 has-[[aria-expanded=true]]:opacity-100"
						onclick={(e) => e.stopPropagation()}
					>
						{@render actions()}
					</div>
				{/if}
			</div>
			<div class="text-2xs font-normal text-secondary whitespace-nowrap">
				{@render detail()}
			</div>
		</div>
	</div>
	<!-- Keyboard access only: its click bubbles to the row, which does the toggling. -->
	<button
		class="w-full flex flex-row-reverse"
		aria-expanded={opened}
		aria-label={toggleLabel ?? name}
	>
		{#if opened}
			<ChevronUp size={16} />
		{:else}
			<ChevronDown size={16} />
		{/if}
	</button>
</div>
