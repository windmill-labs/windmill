<script lang="ts">
	import { listThemes, type Theme, createTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { getContext, onMount } from 'svelte'
	import Path from '$lib/components/Path.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Highlight } from 'svelte-highlight'
	import css from 'svelte-highlight/languages/css'
	import { Eye, EyeOff } from 'lucide-svelte'
	import type { AppViewerContext } from '../../types'
	import { sendUserToast } from '$lib/toast'

	export let cssString: string | undefined = undefined

	const { previewTheme, app } = getContext<AppViewerContext>('AppViewerContext')

	let path: string = ''
	let themes: Theme[] = []

	async function getThemes() {
		themes = await listThemes($workspaceStore!)
	}

	async function addTheme() {
		const theme: Theme = {
			path: path,
			value: cssString
		}

		await createTheme($workspaceStore!, theme)

		getThemes()

		sendUserToast('Theme created')
	}

	onMount(() => {
		getThemes()
	})
</script>

<div class="p-4 flex flex-col items-start w-auto gap-2">
	<div class="text-sm leading-6 font-semibold">Create theme</div>

	<Path bind:path initialPath="" namePlaceholder={'theme'} kind="theme" />
	<Button on:click={() => addTheme()} color="dark" size="xs">Create theme</Button>

	<div class="flex flex-row justify-between items-center w-full mt-8 h-10">
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

	{#if Array.isArray(themes) && themes.length > 0}
		<div class="w-full">
			<DataTable size="sm">
				<Head>
					<tr>
						<Cell first head>Path</Cell>
						<Cell head>Theme</Cell>
						<Cell last head />
					</tr>
				</Head>
				<tbody class="divide-y">
					{#if themes && themes.length > 0}
						{#each themes as row}
							<tr>
								<Cell first>{row.path}</Cell>
								<Cell>
									<Highlight
										class="!text-xs border p-2 rounded-sm"
										language={css}
										code={row.value}
									/>
								</Cell>
								<Cell last>
									<div class="flex flex-row gap-1">
										<Button
											color="light"
											size="xs"
											on:click={() => {
												previewTheme.set(row.value)
											}}
										>
											<div class="flex flex-row gap-1 items-center">
												<Eye size={16} />
												Preview
											</div>
										</Button>

										<Button
											color="dark"
											size="xs"
											on:click={() => {
												$app.cssString = row.value
											}}
										>
											Apply</Button
										>
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
