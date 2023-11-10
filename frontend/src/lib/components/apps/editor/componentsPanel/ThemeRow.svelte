<script lang="ts">
	import { deleteTheme, getTheme, updateTheme, resolveTheme, DEFAULT_THEME } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Code, Eye, GitBranch, Pin, Save, Trash } from 'lucide-svelte'
	import type { AppViewerContext } from '../../types'
	import { sendUserToast } from '$lib/toast'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import ThemeNameEditor from './ThemeNameEditor.svelte'

	import ThemeDrawer from './ThemeDrawer.svelte'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

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

	async function fork() {
		stopPreview()
		const theme = await resolveTheme($app.theme, $workspaceStore)
		$app.theme = {
			type: 'inlined',
			css: theme
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

			<button on:pointerdown|stopPropagation>
				<ButtonDropdown hasPadding={false}>
					<svelte:fragment slot="items">
						<MenuItem on:click={() => themeDrawer?.openDrawer()}>
							<div
								class={classNames(
									'!text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
								)}
							>
								<Code size={16} />
								View code
							</div>
						</MenuItem>
						<MenuItem on:click={fork}>
							<div
								class={classNames(
									'!text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
								)}
							>
								<GitBranch size={16} />
								Fork
							</div>
						</MenuItem>
						{#if row.path !== DEFAULT_THEME}
							<MenuItem on:click={() => makeDefaultTheme(row.path)}>
								<div
									class={classNames(
										'!text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
									)}
								>
									<Pin size={16} />
									Make default
								</div>
							</MenuItem>

							<MenuItem on:click={toggleDelete}>
								<div
									class={classNames(
										'!text-red-600 flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
									)}
								>
									<Trash size={16} />
									Delete
								</div>
							</MenuItem>
						{/if}
					</svelte:fragment>
				</ButtonDropdown>
			</button>
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
