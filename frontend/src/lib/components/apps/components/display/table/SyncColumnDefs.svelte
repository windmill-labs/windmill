<script lang="ts">
	import { getContext, tick, untrack } from 'svelte'
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

	function hasActionsPlaceholder(cols: any[] | undefined) {
		if (!Array.isArray(cols)) return false
		return (
			cols.findIndex(
				(c) => c?.field === '__actions__' || c?.type === 'actions' || c?.actions === true || c?.isActions === true
			) > -1
		)
	}

	function addActionsPlaceholder(cols: any[] | undefined): any[] {
		const hdr = customActionsHeader ?? 'Actions'
		const placeholder = { field: '__actions__', type: 'actions', headerName: hdr }
		if (!Array.isArray(cols)) return [placeholder]
		return [...cols, placeholder]
	}

	function removeActionsPlaceholder(cols: any[] | undefined): any[] {
		if (!Array.isArray(cols)) return []
		return cols.filter(
			(c) => !(c?.field === '__actions__' || c?.type === 'actions' || c?.actions === true || c?.isActions === true)
		)
	}

	async function ensureActionsColumn() {
		// Only act in editor (DND) mode
		if ($mode !== 'dnd') return
		const gridItem = findGridItem($app, id)
		if (!gridItem) return

		const conf: any = (gridItem.data.configuration as any)?.columnDefs
		if (!conf) return

		let currentColumns: any[] | undefined
		if (conf.type === 'static') {
			currentColumns = Array.isArray(conf.value) ? conf.value : []
		} else if (conf.type === 'evalv2') {
			try {
				currentColumns = JSON.parse(conf.expr ?? '[]')
			} catch (e) {
				currentColumns = []
			}
		}

		const hasPlaceholder = hasActionsPlaceholder(currentColumns)
		const needsAdd = actionsPresent && !hasPlaceholder
		const needsRemove = !actionsPresent && hasPlaceholder

		if (!needsAdd && !needsRemove) return

		let nextColumns = currentColumns
		if (needsAdd) nextColumns = addActionsPlaceholder(currentColumns)
		if (needsRemove) nextColumns = removeActionsPlaceholder(currentColumns)

		if (conf.type === 'static') {
			(gridItem.data.configuration as any).columnDefs.value = nextColumns
		} else if (conf.type === 'evalv2') {
			(gridItem.data.configuration as any).columnDefs.expr = JSON.stringify(nextColumns)
		}

		await updateConfiguration()
	}

	$effect(() => {
		// Re-run when inputs change; guarded by equality checks to avoid loops
		actionsPresent
		columnDefs
		untrack(() => ensureActionsColumn())
	})

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
