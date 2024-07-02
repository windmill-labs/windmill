<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'

	export let navbarItem: NavbarItem
	export let id: string
	export let borderColor: string | undefined = undefined

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath, mode } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedPath: string | undefined = undefined
</script>

<ResolveConfig
	{id}
	key={'path' + id}
	bind:resolvedConfig={resolvedPath}
	configuration={navbarItem.path}
/>

<div
	class={twMerge('py-2', appPath === resolvedPath ? 'border-b-2 border-gray-500 ' : '')}
	style={`border-color: ${borderColor ?? 'transparent'}`}
>
	<Button
		href={resolvedPath && typeof resolvedPath === 'string'
			? resolvedPath?.startsWith('http') || resolvedPath?.startsWith('https')
				? resolvedPath
				: resolvedPath
				? `/apps/get/${resolvedPath}`
				: undefined
			: String(resolvedPath)}
		target={$mode === 'dnd' ? '_blank' : '_self'}
		color="light"
		size="xs"
		disabled={navbarItem.disabled || appPath === resolvedPath}
	>
		{#if navbarItem.icon}
			{#key navbarItem.icon}
				<div class="min-w-4" bind:this={icon} />
			{/key}
		{/if}
		{navbarItem.label ?? 'No Label'}
	</Button>
</div>
