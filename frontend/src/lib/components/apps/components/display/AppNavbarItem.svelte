<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'

	export let navbarItem: NavbarItem

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath, mode } = getContext<AppViewerContext>('AppViewerContext')
</script>

<Button
	href={navbarItem.path ? `/apps/get/${navbarItem.path}` : undefined}
	target={$mode === 'dnd' ? '_blank' : '_self'}
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
