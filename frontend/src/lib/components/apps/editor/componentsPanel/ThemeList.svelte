<script lang="ts">
	import {
		listThemes,
		type Theme,
		createTheme,
		deleteTheme,
		getTheme,
		updateTheme,
		resolveTheme
	} from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { getContext, onMount } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { Eye, EyeOff, GitBranch, Pin, Save, Trash } from 'lucide-svelte'
	import type { AppViewerContext } from '../../types'
	import { sendUserToast } from '$lib/toast'
	import { ResourceService } from '$lib/gen'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import ThemeNameEditor from './ThemeNameEditor.svelte'

	const { previewTheme, app } = getContext<AppViewerContext>('AppViewerContext')

	let cssString: string | undefined = $app?.theme?.type === 'inlined' ? $app.theme.css : undefined
	$: type = $app?.theme?.type

	const dispatch = createEventDispatcher()

	let themes: Array<{
		name: string
		path: string
	}> = []

	async function getThemes() {
		themes = await listThemes($workspaceStore!)
	}

	async function addTheme(nameField: string) {
		const themes = await ResourceService.listResourceNames({
			workspace: $workspaceStore!,
			name: 'theme'
		})

		const theme: Theme = {
			path: 'f/themes/theme_' + themes.length,
			value: {
				value: cssString ?? '',
				name: nameField
			}
		}

		const message = await createTheme($workspaceStore!, theme)

		getThemes()

		nameField = ''

		sendUserToast('Theme created:' + message)

		$app.theme = {
			type: 'path',
			path: theme.path
		}
	}

	let nameField: string = ''

	async function makeDefaultTheme(path: string) {
		const defaultTheme = await getTheme($workspaceStore!, 'f/themes/theme_0')
		const theme = await getTheme($workspaceStore!, path)

		updateTheme($workspaceStore!, 'f/themes/theme_0', {
			value: theme.value
		})

		updateTheme($workspaceStore!, path, {
			value: defaultTheme.value
		})

		getThemes()
	}

	onMount(() => {
		getThemes()
	})
</script>

<div class="p-4 flex flex-col items-start w-auto gap-2">
	{#if type === 'inlined'}
		<div class="text-xs leading-6 font-bold">Name</div>

		<div class="w-full flex flex-row gap-2 items-center">
			<input bind:value={nameField} />
			<Button on:click={() => addTheme(nameField)} color="dark" size="xs">Create theme</Button>
		</div>
	{/if}

	{#if Array.isArray(themes) && themes.length > 0}
		<div class="flex flex-row justify-between items-center w-full h-10">
			<div class="text-sm leading-6 font-semibold"> List of themes </div>
			{#if $previewTheme}
				<Button color="red" size="xs" on:click={() => previewTheme.set(undefined)}>
					<div class="flex flex-row gap-1 items-center">
						<EyeOff size={16} />
						Clear preview
					</div>
				</Button>
			{/if}
		</div>
		<div class="w-full">
			<DataTable size="sm">
				<Head>
					<tr>
						<Cell first head>Path</Cell>
						<Cell last head />
					</tr>
				</Head>
				<tbody class="divide-y">
					{#if themes && themes.length > 0}
						{#each themes as row}
							<tr
								class={twMerge(
									$app?.theme?.type === 'path' && $app.theme.path === row.path
										? 'bg-surface-selected'
										: ''
								)}
							>
								<Cell first>
									<div class="flex flex-row gap-1 items-center">
										<ThemeNameEditor
											on:reloadThemes={() => {
												getThemes()
											}}
											{row}
										/>
										{row.name}
									</div>
								</Cell>

								<Cell last>
									<div class={twMerge('flex flex-row gap-1 justify-end ')}>
										{#if row.path !== 'f/themes/theme_0'}
											<Button
												color="light"
												size="xs"
												on:click={() => {
													makeDefaultTheme(row.path)
												}}
											>
												<div class="flex flex-row gap-1 items-center">
													<Pin size={16} />
													Make default
												</div>
											</Button>

											<Button
												color="light"
												size="xs"
												on:click={async () => {
													if ($workspaceStore) {
														await deleteTheme($workspaceStore, row.path)
													}
													await getThemes()
												}}
											>
												<div class="flex flex-row gap-1 items-center">
													<Trash size={16} />
												</div>
											</Button>
										{:else}
											<Badge color="blue">Default theme</Badge>
										{/if}

										{#if type === 'inlined'}
											<Button
												color="light"
												size="xs"
												on:click={async () => {
													if (!$workspaceStore) return

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
												}}
											>
												<div class="flex flex-row gap-1 items-center">
													<Save size={16} />
													Update
												</div>
											</Button>
										{/if}

										<Button
											color="light"
											size="xs"
											on:click={async () => {
												const theme = await resolveTheme(
													{
														type: 'path',
														path: row.path ?? ''
													},
													$workspaceStore
												)
												$previewTheme = theme
											}}
										>
											<div class="flex flex-row gap-1 items-center">
												<Eye size={16} />
											</div>
										</Button>
										<Button
											color="light"
											size="xs"
											on:click={async () => {
												const theme = await resolveTheme($app.theme, $workspaceStore)
												$app.theme = {
													type: 'inlined',
													css: theme
												}
												dispatch('setCodeTab')
											}}
										>
											<div class="flex flex-row gap-1 items-center">
												<GitBranch size={16} />
												Fork
											</div>
										</Button>

										<Button
											color="dark"
											size="xs"
											on:click={() => {
												$app.theme = {
													type: 'path',
													path: row.path ?? ''
												}
											}}
										>
											Apply
										</Button>
									</div>
								</Cell>
							</tr>
						{/each}
					{:else}
						<tr>Loading...</tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	{/if}
</div>
