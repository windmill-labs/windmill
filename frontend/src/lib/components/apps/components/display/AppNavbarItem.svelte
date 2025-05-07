<script lang="ts" context="module">
	import { writable, type Writable } from 'svelte/store'
	let selected: Writable<string | undefined> = writable(undefined)
</script>

<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { type NavbarItem } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { loadIcon } from '../icon'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Output } from '../../rx'
	import ResolveNavbarItemPath from './ResolveNavbarItemPath.svelte'

	export let navbarItem: NavbarItem
	export let id: string
	export let borderColor: string | undefined = undefined
	export let index: number
	export let output: {
		result: Output<{
			currentPath: string
		}>
	}
	export let orientation: 'horizontal' | 'vertical' | undefined = undefined

	let icon: any

	$: navbarItem.icon && icon && handleIcon()

	async function handleIcon() {
		if (navbarItem.icon) {
			icon = await loadIcon(navbarItem.icon, icon, 14, undefined, undefined)
		}
	}

	const { appPath, replaceStateFn, gotoFn, isEditor, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedPath: string | undefined = undefined
	let resolvedLabel: string | undefined = undefined
	let resolvedDisabled: boolean | undefined = undefined
	let resolvedHidden: boolean | undefined = undefined

	function extractPathDetails() {
		const url = window.location.pathname + window.location.search + window.location.hash
		const processedUrl = url.replace('/apps/edit/', '').replace('/apps/get/', '')
		return processedUrl
	}

	onMount(() => {
		$selected = resolvedPath === extractPathDetails() ? resolvedPath : undefined
	})

	let initialized: boolean = false

	function initSelection() {
		initialized = true

		if ($selected) return

		$selected = resolvedPath === extractPathDetails() ? resolvedPath : undefined
	}

	$: !initialized && resolvedPath && initSelection()

	function getButtonProps(resolvedPath: string | undefined) {
		if ($appPath && resolvedPath?.includes($appPath)) {
			return {
				onClick: () => {
					output.result.set({ currentPath: resolvedPath ?? '' })
					if (!resolvedPath) return
					const url = new URL(resolvedPath, window.location.origin)
					const queryParams = url.search
					const hash = url.hash
					replaceStateFn?.(`${window.location.pathname}${queryParams}${hash}`)
					$worldStore.outputsById['ctx'].query.set(
						Object.fromEntries(new URLSearchParams(queryParams).entries())
					)
					$worldStore.outputsById['ctx'].hash.set(hash.substring(1))

					$selected = resolvedPath === extractPathDetails() ? resolvedPath : undefined
				},
				href: undefined,
				target: undefined
			}
		} else if (navbarItem.path.selected === 'app') {
			if (isEditor) {
				return {
					href: `/apps/get/${resolvedPath}`,
					target: '_blank' as const,
					onClick: undefined
				}
			} else {
				return {
					onClick: () => {
						if (resolvedPath) {
							gotoFn?.(`/apps/get/${resolvedPath}`)
						}
					},
					href: undefined,
					target: undefined
				}
			}
		} else {
			return {
				href: resolvedPath,
				target: '_blank' as const,
				onClick: undefined
			}
		}
	}

	$: buttonProps = getButtonProps(resolvedPath)
</script>

<ResolveNavbarItemPath {navbarItem} {id} {index} bind:resolvedPath />

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
		class={twMerge('py-2 border-b-2')}
		style={`border-color: ${
			$selected === resolvedPath ? (borderColor ?? 'transparent') : 'transparent'
		}`}
	>
		<Button
			on:click={buttonProps.onClick ?? (() => {})}
			href={buttonProps.href}
			target={buttonProps.target ?? '_self'}
			color="light"
			size="xs"
			disabled={resolvedDisabled}
			btnClasses={orientation === 'vertical' ? '!justify-start !whitespace-normal !text-left' : ''}
		>
			{#if navbarItem.icon}
				{#key navbarItem.icon}
					<div class="min-w-4" bind:this={icon}></div>
				{/key}
			{/if}
			{resolvedLabel ?? 'No Label'}
		</Button>
	</div>
{/if}
