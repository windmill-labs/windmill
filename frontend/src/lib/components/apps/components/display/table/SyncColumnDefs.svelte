<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { RefreshCw } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { isObject } from '$lib/utils'
	import type { WindmillColumnDef } from './utils'
	import { findGridItem } from '$lib/components/apps/editor/appUtilsCore'

	type ColumnDefsConfiguration =
		| { type: 'static'; value: WindmillColumnDef[] }
		| { type: 'evalv2'; expr: string }

	interface Props {
		id: string
		columnDefs?: WindmillColumnDef[]
		result?: Array<Record<string, any>> | undefined
		allowColumnDefsActions?: boolean
		children?: import('svelte').Snippet
		actionsPresent?: boolean
		customActionsHeader?: string | undefined
	}

	let {
		id,
		columnDefs = [],
		result = [],
		allowColumnDefsActions = true,
		children,
		actionsPresent = false,
		customActionsHeader = undefined
	}: Props = $props()

	const { app, mode, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	let syncInProgress = false

	function hasActionsPlaceholder(cols: WindmillColumnDef[] | undefined): boolean {
		if (!Array.isArray(cols)) return false
		return cols.findIndex((c) => c?._isActionsColumn === true) > -1
	}

	function addActionsPlaceholder(cols: WindmillColumnDef[] | undefined): WindmillColumnDef[] {
		const hdr = customActionsHeader ?? 'Actions'
		const placeholder: WindmillColumnDef = {
			field: '__actions__',
			_isActionsColumn: true,
			headerName: hdr,
			flex: 1
		}
		if (!Array.isArray(cols)) return [placeholder]
		return [...cols, placeholder]
	}

	function removeActionsPlaceholder(cols: WindmillColumnDef[] | undefined): WindmillColumnDef[] {
		if (!Array.isArray(cols)) return []
		return cols.filter((c) => c?._isActionsColumn !== true)
	}

	async function ensureActionsColumn() {
		// Only act in editor (DND) mode
		if ($mode !== 'dnd') return
		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		// Type the configuration more safely
		const rawConf = gridItem.data.configuration?.columnDefs
		if (!rawConf) return

		// Type guard for configuration structure
		const conf = rawConf as ColumnDefsConfiguration
		if (!conf.type || conf.type !== 'static') return

		let currentColumns: WindmillColumnDef[] | undefined
		currentColumns = Array.isArray(conf.value) ? conf.value : []

		const hasPlaceholder = hasActionsPlaceholder(currentColumns)

		// Auto-sync logic: add missing actions column, remove when actions gone
		const needsAdd = actionsPresent && !hasPlaceholder
		const needsRemove = !actionsPresent && hasPlaceholder

		if (!needsAdd && !needsRemove) return

		let nextColumns = currentColumns || []
		if (needsAdd) nextColumns = addActionsPlaceholder(nextColumns)
		if (needsRemove) nextColumns = removeActionsPlaceholder(nextColumns)

		// Update configuration with proper typing
		conf.value = nextColumns

		await updateConfiguration()
	}

	$effect(() => {
		const shouldSync = actionsPresent !== undefined || columnDefs?.length !== undefined
		if (shouldSync && !syncInProgress) {
			syncInProgress = true
			ensureActionsColumn().finally(() => {
				syncInProgress = false
			})
		}
	})

	let keys = $derived(Object.keys(result[0] ?? []).filter((x) => x !== '__index'))

	async function syncColumns() {
		const gridItem = findGridItem($app, id)

		if (gridItem && result) {
			const conf = gridItem.data.configuration.columnDefs as ColumnDefsConfiguration

			const newColumns: WindmillColumnDef[] = keys.map((key) => ({
				field: key,
				headerName: key,
				flex: 1
			}))

			if (conf.type === 'static') {
				conf.value = newColumns
			} else if (conf.type === 'evalv2') {
				conf.expr = JSON.stringify(newColumns)
			}

			await updateConfiguration()
		}
	}

	async function setEmptyColumns() {
		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		const conf = gridItem.data.configuration.columnDefs as ColumnDefsConfiguration

		if (conf.type === 'static') {
			conf.value = []
			await updateConfiguration()
		} else if (conf.type === 'evalv2') {
			conf.expr = '[]'
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
							{#each keys as key}
								<Badge small color="dark-gray">{key}</Badge>
							{/each}
						</div>
						<div class="w-full flex fles-row justify-end">
							<Button
								startIcon={{ icon: RefreshCw }}
								size="xs"
								variant="accent"
								on:click={syncColumns}
							>
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
