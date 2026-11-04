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
			const component = runnableComponent
			await new Promise<void>((resolve, reject) => {
				let settled = false
				let confirming = false
				const succeed = () => {
					if (settled) return
					settled = true
					dispatch('insert')
					sendUserToast('Row inserted', false)
					resolve()
				}
				const fail = (message: string) => {
					if (settled) return
					settled = true
					reject(new Error(message))
				}
				component
					.runComponent(undefined, undefined, undefined, values, {
						// The runnable loads only the result, so a failed job lands here as `{ error }`.
						onDone: (result) => (result?.error ? fail(insertErrorMessage(result)) : succeed()),
						onCancel: () => fail('Insert was canceled'),
						onError: (e, jobId) => {
							if (!jobId) return fail(insertErrorMessage(e))
							if (confirming) return
							confirming = true
							// A failed status read says nothing about the job, which may still commit:
							// failing here would invite a retry that inserts the row twice.
							pollJobResult(jobId, workspace, writingJobOptions).then(succeed, (err) =>
								fail(
									err instanceof ApiError
										? `Could not confirm the insert: ${insertErrorMessage(err)}. Refresh the table to check whether the row was added before retrying.`
										: insertErrorMessage(err)
								)
							)
						}
					})
					.catch((e) => fail(insertErrorMessage(e)))
			})
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
