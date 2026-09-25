<!-- A horizontal rule with something centred in it: the login page's "or" and the
	agent run's output separator. Takes a snippet rather than a string because the
	middle is sometimes a button, not a label. -->
<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		class?: string
		children: import('svelte').Snippet
	}

	let { class: clazz, children }: Props = $props()
</script>

<div class={twMerge('flex items-center gap-2', clazz)}>
	<div class="h-px flex-1 bg-border-light"></div>
	{@render children()}
	<div class="h-px flex-1 bg-border-light"></div>
</div>
