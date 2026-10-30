<script lang="ts">
	import type { Snippet } from 'svelte'

	interface Props {
		label: string
		/** Applied to the wrapper of the group's settings. */
		class?: string
		children?: Snippet
	}

	let { label, class: clazz = '', children }: Props = $props()
</script>

<!-- A group of settings whose rows explain themselves: the label only names the group, so it stays
     small and quiet instead of a section heading. -->
<section class="flex flex-col gap-3">
	<div class="text-2xs font-medium uppercase tracking-wide text-secondary">{label}</div>
	<div class={clazz}>
		{@render children?.()}
	</div>
</section>
