<script lang="ts">
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef, createPostgresInsert } from './utils'
	import { sendUserToast } from '$lib/toast'

	export let id: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_insert`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let loading = false
	let input: AppInput | undefined = undefined

	const dispatch = createEventDispatcher()

	export async function insertRow(
		resource: string,
		workspace: string | undefined,
		table: string | undefined,
		columns: ColumnDef[],
		values: Record<string, any>
	): Promise<boolean> {
		if (!resource || !table || !workspace) {
			return false
		}

		input = createPostgresInsert(table, columns, resource)

		await tick()

		if (runnableComponent) {
			await runnableComponent?.runComponent(undefined, undefined, undefined, values, {
				done: (x) => {
					dispatch('insert')
					sendUserToast('Row inserted', false)
				},
				cancel: () => {
					sendUserToast('Error inserting row', true)
				},
				error: () => {
					sendUserToast('Error inserting row', true)
				}
			})
		}

		return false
	}
</script>

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_insert`}
	{outputs}
/>
