<script lang="ts">
	import { createEventDispatcher, getContext, tick, untrack } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef } from './utils'
	import { sendUserToast } from '$lib/toast'
	import { getInsertInput } from './queries/insert'
	import type { DbInput } from '$lib/components/dbTypes'
	import { ApiError } from '$lib/gen'
	import { pollJobResult } from '$lib/components/jobs/utils'
	import { writingJobOptions } from '$lib/components/jobs/writingJob'

	interface Props {
		id: string
	}

	let { id }: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${untrack(() => id)}_insert`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state()
	let loading = $state(false)
	let input: AppInput | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	export async function insertRow(
		dbInput: DbInput,
		workspace: string | undefined,
		table: string | undefined,
		columns: ColumnDef[],
		values: Record<string, any>
	): Promise<boolean> {
		if (
			(dbInput.type == 'ducklake' && !dbInput.ducklake) ||
			(dbInput.type == 'database' && !dbInput.resourcePath) ||
			!table ||
			!workspace
		) {
			return false
		}

		input = getInsertInput(dbInput, table, columns)

		await tick()

		if (runnableComponent) {
			let submitError: unknown
			const jobId = await runnableComponent.runComponent(undefined, undefined, undefined, values, {
				onError: (e) => (submitError ??= e)
			})
			if (!jobId) {
				throw new Error(submitError ? insertErrorMessage(submitError) : 'The insert did not start')
			}
			// The job's own outcome, not the runnable's loader: the loader also reports failed
			// status reads and gives up watching while the job may still commit, and failing on
			// either would invite a retry that inserts the row twice.
			try {
				await pollJobResult(jobId, workspace, writingJobOptions)
			} catch (err) {
				// A job that ran and failed: its error is the database's.
				if (err instanceof Error && !(err instanceof ApiError)) throw err
				console.error('Could not read the insert job result', err)
				throw new Error(
					'Could not confirm the insert. Refresh the table to check whether the row was added before retrying.'
				)
			}
			dispatch('insert')
			sendUserToast('Row inserted', false)
		}

		return false
	}

	function insertErrorMessage(e: any): string {
		const error = e?.error ?? e
		if (typeof error === 'string') return error
		return error?.message ?? JSON.stringify(error)
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
