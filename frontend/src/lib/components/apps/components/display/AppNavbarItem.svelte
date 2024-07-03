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
	let resolvedLabel: string | undefined = undefined
	let resolvedDisabled: boolean | undefined = undefined
	let resolvedHidden: boolean | undefined = undefined

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
	let s = false
</script>

<ResolveConfig
	{id}
	key={'path'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedPath}
	configuration={navbarItem.path}
/>

<ResolveConfig
	{id}
	key={'label'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedLabel}
	configuration={navbarItem.label}
/>

<ResolveConfig
	{id}
	key={'disabled'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedDisabled}
	configuration={navbarItem.disabled}
/>

<ResolveConfig
	{id}
	key={'hidden'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedHidden}
	configuration={navbarItem.hidden}
/>
{#if !resolvedHidden}
	<div
		class={twMerge('py-2 ', isSelected ? 'border-b-2 border-gray-500 ' : '')}
		style={`border-color: ${borderColor ?? 'transparent'}`}
	>
		{#if s}
			<Button
				on:click={() => {
					output.result.set({ currentPath: resolvedPath ?? '' })
					isSelected = true
				}}
				color="light"
				size="xs"
				disabled={resolvedDisabled}
			>
				{#if navbarItem.icon}
					{#key navbarItem.icon}
						<div class="min-w-4" bind:this={icon} />
					{/key}
				{/if}
				{resolvedLabel ?? 'No Label'}
			</Button>
		{:else}
			<Button
				href={computeHref(resolvedPath)}
				target={computeTarget(navbarItem, $mode)}
				color="light"
				size="xs"
				disabled={resolvedDisabled}
			>
				{#if navbarItem.icon}
					{#key navbarItem.icon}
						<div class="min-w-4" bind:this={icon} />
					{/key}
				{/if}
				{resolvedLabel ?? 'No Label'}
			</Button>
		{/if}
	</div>
{/if}
