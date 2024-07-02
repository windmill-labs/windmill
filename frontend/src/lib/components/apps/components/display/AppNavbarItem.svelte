<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, EditorMode } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Output } from '../../rx'

	export let navbarItem: NavbarItem
	export let id: string
	export let borderColor: string | undefined = undefined
	export let index: number
	export let output: {
		result: Output<{
			currentPath: string
		}>
	}

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedPath: string | undefined = undefined

	function computeHref(resolvedPath: string | undefined): string | undefined {
		if (navbarItem.path.type === 'static' && navbarItem.path.fieldType === 'select') {
			return resolvedPath ? `/apps/get/${resolvedPath}` : undefined
		} else {
			return resolvedPath ?? undefined
		}
	}

	function computeTarget(navbarItem: NavbarItem, mode: EditorMode): '_self' | '_blank' | undefined {
		if (navbarItem.path.type === 'evalv2') {
			return '_blank'
		}

		return mode === 'dnd' ? '_blank' : '_self'
	}

	let isSelected = false

	function checkIfSelected() {
		isSelected = output.result.peak()?.currentPath === resolvedPath
	}

	output.result.subscribe(
		{
			next: (next) => {
				isSelected = next.currentPath === resolvedPath
			},
			id: resolvedPath
		},
		{
			currentPath: resolvedPath ?? ''
		}
	)

	$: resolvedPath && checkIfSelected()
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
	class={twMerge('py-2 ', isSelected ? 'border-b-2 border-gray-500 ' : '')}
	style={`border-color: ${borderColor ?? 'transparent'}`}
>
	{#if navbarItem.writeOutputOnClick}
		<Button
			on:click={() => {
				output.result.set({ currentPath: resolvedPath ?? '' })
				isSelected = true
			}}
			color="light"
			size="xs"
			disabled={navbarItem.disabled}
		>
			{#if navbarItem.icon}
				{#key navbarItem.icon}
					<div class="min-w-4" bind:this={icon} />
				{/key}
			{/if}
			{navbarItem.label ?? 'No Label'}
		</Button>
	{:else}
		<Button
			href={computeHref(resolvedPath)}
			target={computeTarget(navbarItem, $mode)}
			color="light"
			size="xs"
			disabled={navbarItem.disabled}
		>
			{#if navbarItem.icon}
				{#key navbarItem.icon}
					<div class="min-w-4" bind:this={icon} />
				{/key}
			{/if}
			{navbarItem.label ?? 'No Label'}
		</Button>
	{/if}
</div>
