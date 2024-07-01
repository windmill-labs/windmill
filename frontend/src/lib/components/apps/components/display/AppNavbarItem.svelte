<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'

	export let navbarItem: NavbarItem

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	$: console.log('navbarItem:', navbarItem)
	async function handleIcon() {
		console.log('navbarItem.icon:', navbarItem.icon)
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath } = getContext<AppViewerContext>('AppViewerContext')
</script>

<Button
	href={navbarItem.path ? `/apps/get/${navbarItem.path}` : undefined}
	target="_blank"
	color="light"
	size="xs"
	disabled={navbarItem.disabled || appPath === navbarItem.path}
>
	{#if navbarItem.icon}
		{#key navbarItem.icon}
			<div class="min-w-4" bind:this={icon} />
		{/key}
	{/if}
	{navbarItem.label ?? 'No Label'}
</Button>
