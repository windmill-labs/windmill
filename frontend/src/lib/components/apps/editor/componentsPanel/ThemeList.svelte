<script lang="ts">
	import { listThemes, type Theme, createTheme } from './themeUtils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { getContext, onMount } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { EyeOff } from 'lucide-svelte'
	import type { AppViewerContext } from '../../types'
	import { sendUserToast } from '$lib/toast'
	import { ResourceService } from '$lib/gen'
	import { Alert } from '$lib/components/common'
	import ThemeRow from './ThemeRow.svelte'

	const { previewTheme, app } = getContext<AppViewerContext>('AppViewerContext')

	let cssString: string | undefined = $app?.theme?.type === 'inlined' ? $app.theme.css : undefined
	$: type = $app?.theme?.type

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
	let previewThemePath: string | undefined = undefined

	onMount(() => {
		getThemes()
	})
</script>

<div class="p-4 flex flex-col items-start w-auto gap-2 relative">
	{#if $enterpriseLicense === undefined}
		<div class="absolute top-0 left-0 w-full h-full bg-gray-50 opacity-50 z-10 bottom-0" />
		<Alert
			type="warning"
			title="Themes are available in the enterprise edition."
			class="w-full z-50"
			size="xs"
		>
			Upgrade to the enterprise edition to use themes.
		</Alert>
	{/if}
	{#if type === 'inlined'}
		<div class="text-xs leading-6 font-bold">Name</div>

		<div class="w-full flex flex-row gap-2 items-center">
			<input bind:value={nameField} />
			<Button on:click={() => addTheme(nameField)} color="dark" size="xs">Create theme</Button>
		</div>
	{/if}

	{#if Array.isArray(themes) && themes.length > 0}
		<div class="flex flex-row justify-end items-center w-full h-10">
			<Button
				disabled={!Boolean($previewTheme)}
				color="dark"
				variant="border"
				size="xs"
				on:click={() => {
					previewTheme.set(undefined)
					previewThemePath = undefined
				}}
			>
				<div class="flex flex-row gap-1 items-center">
					<EyeOff size={16} />
					Clear preview
				</div>
			</Button>
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
							{#key row}
								<ThemeRow
									{row}
									bind:previewThemePath
									on:reloadThemes={() => {
										getThemes()
									}}
								/>
							{/key}
						{/each}
					{:else}
						<tr>Loading...</tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	{/if}
</div>
