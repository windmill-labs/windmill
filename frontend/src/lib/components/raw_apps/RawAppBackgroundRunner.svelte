<script lang="ts">
	import { executeRunnable } from '../apps/components/helpers/executeRunnable'
	import { userStore } from '$lib/stores'
	import { waitJob } from '../waitJob'
	import type { HiddenRunnable, JobById } from '../apps/types'
	import { JobService } from '$lib/gen'

	export let iframe: HTMLIFrameElement | undefined
	export let path: string
	export let runnables: Record<string, HiddenRunnable>
	export let jobs: string[] = []
	export let jobsById: Record<string, JobById> = {}
	export let editor: boolean
	export let workspace: string

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

			if (event.data.type == 'runBg') {
				respond({ result, error })
			}
			return result
		}
		if (event.data.type == 'runBg' || event.data.type == 'runBgAsync') {
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
						args: data.v,
						force_viewer_allow_user_resources: Object.keys(runnable.fields).filter(
							(k) => runnable.fields[k]?.type == 'user' && runnable.fields[k]?.allowUserResources
						),
						force_viewer_one_of_fields: {},
						force_viewer_static_fields: Object.fromEntries(
							Object.entries(runnable.fields)
								.filter(([k, v]) => v.type == 'static')
								.map(([k, v]) => [k, v?.['value']])
						)
					},
					undefined
				)
				let job: JobById = { component: runnable_id, created_at: Date.now(), job: uuid }
				if (event.data.type == 'runBgAsync') {
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

<svelte:window on:message={listener} />
