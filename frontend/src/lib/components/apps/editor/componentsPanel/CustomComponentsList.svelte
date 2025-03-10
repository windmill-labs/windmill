<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	import { sendUserToast } from '$lib/toast'
	import { ResourceService } from '$lib/gen'
	import CustomComponentRow from './CustomComponentRow.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { createEventDispatcher } from 'svelte'

	let customcomponents: Array<{
		name: string
		path: string
	}> = []

	let loading: boolean = false

	async function getCustomComponents() {
		loading = true
		customcomponents = await ResourceService.listResourceNames({
			workspace: $workspaceStore ?? '',
			name: 'app_custom'
		})
		loading = false
	}

	const dispatch = createEventDispatcher()

	async function addCustomComponent(nameField: string) {
		try {
			const message = await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					resource_type: 'app_custom',
					path: `f/app_custom/${nameField.replace(/-/g, '_').replace(/\s/g, '_')}`,
					value: {
						value: {
							additionalLibs: {
								reactVersion: useReact ? reactVersion : undefined
							}
						},
						name: nameField,
						js: await files[0].text()
					}
				}
			})
			sendUserToast('Component created: ' + message)
			dispatch('reload')
		} catch (e) {
			sendUserToast(
				'Component creation failed. Is the file uploaded, did you give it a name ? Do you have write privilege on folder app_custom: ' +
					(e.body ?? e),
				true
			)
		}
		getCustomComponents()
		nameField = ''
	}

	let nameField: string = ''
	let reactVersion: string = '18.2.0'
	let files: FileList
	let useReact = true

	getCustomComponents()
</script>

<div class="p-2 flex flex-col items-start w-auto gap-2 relative">
	<div class="w-full flex flex-col gap-y-2 pb-8">
		<div>
			<input type="text" bind:value={nameField} placeholder={'Custom Component name'} />
		</div>
		<div>
			<input type="file" accept="text/javascript " bind:files multiple={false} />
		</div>
		<div class="flex gap-4">
			<Toggle bind:checked={useReact} options={{ right: 'React Component' }} />
			<div>
				<input
					disabled={!useReact}
					type="text"
					bind:value={reactVersion}
					placeholder={'React version'}
				/></div
			></div
		>
		<Button on:click={() => addCustomComponent(nameField)} color="dark" size="xs"
			>Add Custom Component</Button
		>
	</div>

	{#if loading}
		<div class="flex flex-col w-full pt-12">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.5]} />
			{/each}
		</div>
	{:else if Array.isArray(customcomponents) && customcomponents.length > 0}
		<div class="w-full">
			<DataTable size="xs">
				<Head>
					<tr>
						<Cell first head>Path</Cell>
						<Cell last head />
					</tr>
				</Head>
				<tbody class="divide-y">
					{#if customcomponents && customcomponents.length > 0}
						{#each customcomponents as row}
							{#key row}
								<CustomComponentRow
									{row}
									on:reload={() => {
										getCustomComponents()
										dispatch('reload')
									}}
								/>
							{/key}
						{/each}
					{:else}
						<!-- <tr>Loading...</tr> -->
					{/if}
				</tbody>
			</DataTable>
		</div>
	{/if}
</div>
