<script lang="ts">
	import { deleteTheme, getTheme, updateTheme, resolveTheme, DEFAULT_THEME } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Code, Eye, GitBranch, Pin, Save, Trash } from 'lucide-svelte'
	import type { AppTheme, AppViewerContext } from '../../types'
	import { sendUserToast } from '$lib/toast'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import ThemeNameEditor from './ThemeNameEditor.svelte'

	import ThemeDrawer from './ThemeDrawer.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'

	export let previewThemePath: string | undefined = undefined

	export let row: {
		name: string
		path: string
	}

	const { previewTheme, app } = getContext<AppViewerContext>('AppViewerContext')

	let cssString: string | undefined = $app?.theme?.type === 'inlined' ? $app.theme.css : undefined
	$: type = $app?.theme?.type

	const dispatch = createEventDispatcher()

	async function toggleUpdate(row) {
		if (!$workspaceStore) return

		try {
			await updateTheme($workspaceStore, row.path, {
				value: {
					name: row.name,
					value: cssString ?? ''
				}
			})

			$app.theme = {
				type: 'path',
				path: row.path
			}

			sendUserToast('Theme updated:\n' + row.name)
		} catch (e) {
			sendUserToast('Theme update failed:\n' + e)
		}
	}

	async function makeDefaultTheme(path: string) {
		const defaultTheme = await getTheme($workspaceStore!, DEFAULT_THEME)
		const theme = await getTheme($workspaceStore!, path)

		await updateTheme($workspaceStore!, DEFAULT_THEME, {
			value: {
				value: theme.value ?? '',
				name: theme.name
			}
		})

		await updateTheme($workspaceStore!, path, {
			value: {
				value: defaultTheme.value ?? '',
				name: defaultTheme.name
			}
		})

		stopPreview()
		$app.theme = {
			type: 'path',
			path: DEFAULT_THEME
		}

		dispatch('reloadThemes')
	}

	async function toggleDelete() {
		stopPreview()
		if ($workspaceStore) {
			await deleteTheme($workspaceStore, row.path)
		}
		dispatch('reloadThemes')
		sendUserToast('Theme deleted:\n' + row.name)
	}

	async function preview() {
		previewThemePath = row.path

		const theme = await resolveTheme(
			{
				type: 'path',
				path: row.path ?? ''
			},
			$workspaceStore
		)
		$previewTheme = theme ?? ''
	}

	async function fork(path: string) {
		stopPreview()

		const theme: AppTheme = {
			type: 'path',
			path: path ?? ''
		}

		const resolvedTheme = await resolveTheme(theme, $workspaceStore)

		$app.theme = {
			type: 'inlined',
			css: resolvedTheme
		}

		dispatch('setCodeTab')
	}

	function stopPreview() {
		previewThemePath = undefined
		$previewTheme = undefined
	}

	function apply() {
		stopPreview()
		$app.theme = {
			type: 'path',
			path: row.path ?? ''
		}
	}

	async function getDropdownItems() {
		return [
			{
				action: () => themeDrawer?.openDrawer(),
				icon: Code,
				displayName: 'View code',
				type: 'action' as const
			},
			{
				action: () => fork(row.path),
				icon: GitBranch,
				displayName: 'Fork',
				type: 'action' as const
			},
			{
				action: () => makeDefaultTheme(row.path),
				icon: Pin,
				displayName: 'Make default',
				type: 'action' as const,
				disabled: row.path === DEFAULT_THEME
			},
			{
				action: toggleDelete,
				icon: Trash,
				displayName: 'Delete',
				type: 'delete' as const,
				disabled: row.path === DEFAULT_THEME
			}
		]
	}
	let themeDrawer: ThemeDrawer
</script>

<tr class={twMerge(previewThemePath === row.path ? 'bg-blue-200' : '', 'transition-all')}>
	<Cell first>
		<div class="flex flex-row gap-1 items-center">
			<ThemeNameEditor on:reloadThemes {row} />
			{row.name}
		</div>
	</Cell>

	<Cell last>
		<div class={twMerge('flex flex-row gap-1 justify-end ')}>
			{#if row.path === DEFAULT_THEME}
				<Badge color="blue" small>Default</Badge>
			{/if}

			{#if $app?.theme?.type === 'path' && $app.theme.path === row.path}
				<Badge color="green" small>Active</Badge>
			{/if}

			{#if type === 'inlined'}
				<Button
					color="light"
					size="xs"
					on:click={() => toggleUpdate(row)}
					startIcon={{ icon: Save }}
				>
					Update
				</Button>
			{/if}
			{#if $app?.theme?.type !== 'path' || $app.theme.path !== row.path}
				<Button color="light" size="xs" on:click={preview} startIcon={{ icon: Eye }}>
					Preview
				</Button>
				<Button color="dark" size="xs2" on:click={apply}>Apply</Button>
			{/if}

			<Dropdown items={getDropdownItems} class="w-fit" />
		</div>
	</Cell>
</tr>

<ThemeDrawer
	bind:this={themeDrawer}
	theme={{
		type: 'path',
		path: row.path ?? ''
	}}
/>
