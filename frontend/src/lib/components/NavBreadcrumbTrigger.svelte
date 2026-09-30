<!--
@component
A breadcrumb segment that opens a melt menu.

The menu hands its trigger builder to a snippet, and a builder store cannot be subscribed inside
one — so the segment takes it as a prop here, where both its attributes and its action are legal.
-->
<script lang="ts">
	import { conditionalMelt } from '$lib/utils'
	import type { Snippet } from 'svelte'

	let {
		trigger,
		class: className = '',
		title,
		children
	}: {
		trigger?: any
		class?: string
		title?: string
		children: Snippet
	} = $props()
</script>

<button use:conditionalMelt={trigger} {...$trigger} class={className} {title}>
	{@render children()}
</button>
