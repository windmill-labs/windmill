<!--
@component
Re-exposes contexts from one component tree inside another.

A snippet handed to the page header renders under the header, not under the page that wrote it, so
any component it creates looks its contexts up in the header's tree and finds nothing. The page
passes the entries its buttons need and this sets them again around the render.
-->
<script lang="ts">
	import { setContext, type Snippet } from 'svelte'

	let { contexts, children }: { contexts?: Map<any, any>; children: Snippet } = $props()

	// setContext only takes effect during init, so this instance is keyed on `contexts` by callers.
	if (contexts) for (const [key, value] of contexts) setContext(key, value)
</script>

{@render children()}
