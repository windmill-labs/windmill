<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, EditorMode } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'

	export let navbarItem: NavbarItem
	export let id: string
	export let borderColor: string | undefined = undefined
	export let index: number

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath, mode } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedPath: string | undefined = undefined

	function computeHref(resolvedPath: string | undefined): string | undefined {
		if (resolvedPath && typeof resolvedPath === 'string') {
			return resolvedPath?.startsWith('http') || resolvedPath?.startsWith('https')
				? resolvedPath
				: resolvedPath
				? `/apps/get/${resolvedPath}`
				: undefined
		} else {
			return String(resolvedPath)
		}
	}

	function computeTarget(
		resolvedPath: string | undefined,
		mode: EditorMode
	): '_self' | '_blank' | undefined {
		if (resolvedPath?.startsWith('http') || resolvedPath?.startsWith('https')) {
			return '_blank'
		}

		return mode === 'dnd' ? '_blank' : '_self'
	}
</script>

{#key navbarItem}
	<ResolveConfig
		{id}
		key={'path'}
		extraKey={String(index)}
		bind:resolvedConfig={resolvedPath}
		configuration={navbarItem.path}
	/>
{/key}

<div
	class={twMerge('py-2 ', appPath === resolvedPath ? 'border-b-2 border-gray-500 ' : '')}
	style={`border-color: ${borderColor ?? 'transparent'}`}
>
	<Button
		href={computeHref(resolvedPath)}
		target={computeTarget(resolvedPath, $mode)}
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
