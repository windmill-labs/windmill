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
	import { onDestroy } from 'svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'

	const { previewTheme, app } = getContext<AppViewerContext>('AppViewerContext')

	let cssString: string | undefined = $app?.theme?.type === 'inlined' ? $app.theme.css : undefined
	$: type = $app?.theme?.type

	let themes: Array<{
		name: string
		path: string
	}> = []

	let loading: boolean = false

	async function getThemes() {
		loading = true
		themes = await listThemes($workspaceStore!)
		loading = false
	}

	async function addTheme(nameField: string) {
		const themes = await ResourceService.listResourceNames({
			workspace: $workspaceStore!,
			name: 'app_theme'
		})

		const theme: Theme = {
			path: 'f/app_themes/theme_' + themes.length,
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

	onDestroy(() => {
		previewTheme.set(undefined)
		previewThemePath = undefined
	})
</script>

<div class="p-2 flex flex-col items-start w-auto gap-2 relative">
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
	<div class="w-full flex flex-row gap-2 items-center">
		<input
			disabled={type != 'inlined'}
			bind:value={nameField}
			placeholder={type == 'inlined'
				? 'Theme name'
				: 'Fork a theme and edit it to create a new one'}
		/>
		<Button
			disabled={type != 'inlined' || nameField == ''}
			on:click={() => addTheme(nameField)}
			color="dark"
			size="xs">Create theme</Button
		>
	</div>

	{#if loading}
		<div class="flex flex-col w-full pt-12">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.5]} />
			{/each}
		</div>
	{:else if Array.isArray(themes) && themes.length > 0}
		<div class="flex flex-row justify-end items-center w-full h-10">
			{#if $previewTheme != undefined}
				<Button
					color="dark"
					variant="border"
					size="xs"
					on:click={() => {
						previewTheme.set(undefined)
						previewThemePath = undefined
					}}
					startIcon={{ icon: EyeOff }}
				>
					Clear preview
				</Button>
			{/if}
		</div>
		<div class="w-full">
			<DataTable size="xs">
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
