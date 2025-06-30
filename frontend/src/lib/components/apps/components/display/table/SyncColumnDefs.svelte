<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import { findGridItem } from '$lib/components/apps/editor/appUtils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { RefreshCw } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { isObject } from '$lib/utils'

	interface Props {
		id: string
		columnDefs?: Array<any>
		result?: Array<any> | undefined
		allowColumnDefsActions?: boolean
		children?: import('svelte').Snippet
	}

	let {
		id,
		columnDefs = [],
		result = [],
		allowColumnDefsActions = true,
		children
	}: Props = $props()

	const { app, mode, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	async function syncColumns() {
		let gridItem = findGridItem($app, id)

		if (gridItem && result) {
			const keys = Object.keys(result[0] ?? {}) ?? []

			if (gridItem.data.configuration.columnDefs.type === 'static') {
				gridItem.data.configuration.columnDefs.value = keys.map((key) => ({
					field: key,
					headerName: key,
					flex: 1
				}))
			} else if (gridItem.data.configuration.columnDefs.type === 'evalv2') {
				gridItem.data.configuration.columnDefs.expr = JSON.stringify(
					keys.map((key, index) => ({
						field: key,
						headerName: key,
						flex: 1
					}))
				)
			}

			await updateConfiguration()
		}
	}

	async function setEmptyColumns() {
		let gridItem = findGridItem($app, id)
		if (gridItem && gridItem.data.configuration.columnDefs.type === 'static') {
			gridItem.data.configuration.columnDefs.value = []
			await updateConfiguration()
		} else if (gridItem && gridItem.data.configuration.columnDefs.type === 'evalv2') {
			gridItem.data.configuration.columnDefs.expr = '[]'
			await updateConfiguration()
		}
	}

	async function updateConfiguration() {
		$selectedComponent = undefined
		await tick()
		$selectedComponent = [id]
	}
</script>

{#if Array.isArray(result) && result.every(isObject)}
	{#if Array.isArray(columnDefs) && columnDefs.every(isObject)}
		{#if $mode === 'dnd' && columnDefs?.length === 0 && result?.length > 0 && allowColumnDefsActions}
			<div class="m-16">
				<Alert title="Missing column definitions">
					<div class="flex flex-col items-start gap-2">
						<div class="text-xs"> No columns definition found. Columns found in data: </div>
						<div class="text-sm flex flex-row gap-2">
							{#each Object.keys(result[0] ?? []) as key}
								<Badge small color="dark-gray">{key}</Badge>
							{/each}
						</div>
						<div class="w-full flex fles-row justify-end">
							<Button startIcon={{ icon: RefreshCw }} size="xs" color="dark" on:click={syncColumns}>
								Sync columns definition
							</Button>
						</div>
					</div>
				</Alert>
			</div>
		{:else}
			{@render children?.()}
		{/if}
	{:else if columnDefs !== undefined}
		<div class="m-16">
			<Alert title="Parsing issues" type="error" size="xs">
				<div class="flex flex-col items-start gap-2">
					The columnDefs should be an array of objects, received:
					<pre class="overflow-auto">
{JSON.stringify(columnDefs)}
				</pre>
					{#if allowColumnDefsActions}
						<div class="w-full flex fles-row justify-end">
							<Button
								startIcon={{ icon: RefreshCw }}
								size="xs"
								color="red"
								on:click={setEmptyColumns}
							>
								Fix columns definitions
							</Button>
						</div>
					{/if}
				</div>
			</Alert>
		</div>
	{:else if $mode === 'dnd'}
		<div class="m-16">
			<Alert title="Parsing issues" type="error" size="xs">
				<div class="flex flex-col items-start gap-2">
					The columnDefs are undefined.
					{#if allowColumnDefsActions}
						<div class="w-full flex fles-row justify-end">
							<Button
								startIcon={{ icon: RefreshCw }}
								size="xs"
								color="red"
								on:click={setEmptyColumns}
							>
								Fix columns definitions
							</Button>
						</div>
					{/if}
				</div>
			</Alert>
		</div>
	{/if}
{/if}
