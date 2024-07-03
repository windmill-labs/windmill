<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
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
	export let replaceStateFn: (url: string) => void = (url) =>
		window.history.replaceState(null, '', url)

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedPath: string | undefined = undefined
	let resolvedLabel: string | undefined = undefined
	let resolvedDisabled: boolean | undefined = undefined
	let resolvedHidden: boolean | undefined = undefined

	function extractPathDetails() {
		const fullUrl = window.location.href

		const inEditor = fullUrl.includes('/apps/edit/')
		const baseIndex = inEditor
			? fullUrl.indexOf('/apps/edit/') + 11
			: fullUrl.indexOf('/apps/get/') + 10

		return fullUrl.substring(baseIndex)
	}

	$: isSelected = resolvedPath === extractPathDetails()

	output.result.subscribe(
		{
			next: (next) => {
				if (next.currentPath) {
					isSelected = next.currentPath === resolvedPath
				}
			},
			id: resolvedPath
		},
		{
			currentPath: resolvedPath ?? ''
		}
	)
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
		{#if resolvedPath?.includes(appPath)}
			<Button
				on:click={() => {
					output.result.set({ currentPath: resolvedPath ?? '' })

					if (!resolvedPath) return

					const url = new URL(resolvedPath, window.location.origin)
					const queryParams = url.search
					const hash = url.hash

					replaceStateFn(`${window.location.pathname}${queryParams}${hash}`)
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
				href={resolvedPath}
				target="_blank"
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
