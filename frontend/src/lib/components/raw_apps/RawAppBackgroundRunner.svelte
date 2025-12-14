<script lang="ts">
	import { executeRunnable } from '../apps/components/helpers/executeRunnable'
	import { userStore } from '$lib/stores'
	import { waitJob } from '../waitJob'
	import type { JobById } from '../apps/types'
	import { JobService } from '$lib/gen'
	import type { Runnable } from './rawAppPolicy'
	import { undefinedIfEmpty } from '$lib/utils'

	interface Props {
		iframe: HTMLIFrameElement | undefined
		path: string
		runnables: Record<string, Runnable>
		jobs?: string[]
		jobsById?: Record<string, JobById>
		editor: boolean
		workspace: string
	}

	let {
		iframe,
		path,
		runnables,
		jobs = $bindable([]),
		jobsById = $bindable({}),
		editor,
		workspace
	}: Props = $props()

	let listener = async (event) => {
		const data = event.data
		function respond(o: object) {
			iframe?.contentWindow?.postMessage({ type: data.type + 'Res', ...o, reqId: data.reqId }, '*')
		}
		async function respondWithResult(uuid: string) {
			let error = false
			let result
			try {
				result = await waitJob(uuid)
			} catch (e) {
				error = true
				console.log('e', e)
				result = e
			}

			if (event.data.type == 'backend') {
				respond({ result, error })
			}
			return result
		}
		if (event.data.type == 'backend' || event.data.type == 'backendAsync') {
			const runnable_id = data.runnable_id
			let runnable = runnables[runnable_id]
			if (runnable) {
				const uuid = await executeRunnable(
					runnable,
					workspace,
					undefined,
					$userStore?.username,
					path,
					runnable_id,
					{
						component: runnable_id,
						args: data.v ?? {},
						force_viewer_allow_user_resources: editor
							? undefinedIfEmpty(
									Object.keys(runnable?.fields ?? {}).filter(
										(k) =>
											runnable?.fields?.[k]?.type == 'user' &&
											runnable?.fields?.[k]?.allowUserResources
									)
								)
							: undefined,
						force_viewer_one_of_fields: undefined,
						force_viewer_static_fields: editor
							? Object.fromEntries(
									Object.entries(runnable?.fields ?? {})
										.filter(([k, v]) => v.type == 'static')
										.map(([k, v]) => [k, v?.['value']])
								)
							: undefined
					},
					undefined
				)
				let job: JobById = { component: runnable_id, created_at: Date.now(), job: uuid }
				if (event.data.type == 'backendAsync') {
					let result = uuid
					respond({ result })
				}
				if (editor) {
					jobsById[uuid] = job
					jobs = [...jobs, uuid]
				}

				const result = await respondWithResult(uuid)

				if (editor) {
					job.result = result
				}
			} else if (event.data.type == 'waitJob') {
				await respondWithResult(data.jobId)
			} else if (event.data.type == 'getJob') {
				const job = JobService.getJob({ workspace, id: data.jobId })
				respond({ result: job })
			} else {
				console.error('No runnable found for', runnable_id)
			}
		}
	}
</script>

<svelte:window onmessage={listener} />
