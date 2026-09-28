<script lang="ts">
	import { onDestroy } from 'svelte'
	import { registerToolDisplayActionHandler } from './createdResourceActions.svelte'
	import type CreatedResourceActionDrawersInner from './CreatedResourceActionDrawersInner.svelte'

	// The handler registers during render, so the action is offered right away and a page
	// that takes it over (the sessions panel) registers on top of it. The drawers themselves
	// load on first use: they reach the resource and variable editors.
	let inner: Promise<typeof import('./CreatedResourceActionDrawersInner.svelte')> | undefined =
		$state()
	let drawers: CreatedResourceActionDrawersInner | undefined = $state()
	let resolveBound: (d: CreatedResourceActionDrawersInner) => void = () => {}
	const bound = new Promise<CreatedResourceActionDrawersInner>((r) => (resolveBound = r))
	$effect(() => {
		if (drawers) resolveBound(drawers)
	})

	const unregister = registerToolDisplayActionHandler('open_created_resource', async (action) => {
		inner ??= import('./CreatedResourceActionDrawersInner.svelte')
		await inner
		await (await bound).openCreatedResource(action)
	})

	onDestroy(() => {
		unregister()
	})
</script>

{#if inner}
	{#await inner then Inner}
		<Inner.default bind:this={drawers} />
	{/await}
{/if}
