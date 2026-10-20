<script lang="ts">
	import type { Snippet } from 'svelte'

	interface Props {
		children: Snippet
		class?: string
	}

	let { children, class: className = '' }: Props = $props()
</script>

<!-- Names a section of Home without competing with what it labels. -->
<h2 class="text-xs font-normal text-secondary {className}">
	{@render children()}
</h2>
