<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'

	interface Props {
		loading: boolean
		jobId: string | undefined
		workspaceId: string | undefined
		refreshLog?: boolean
		logs: string | undefined
	}

	let {
		loading,
		jobId = undefined,
		workspaceId = undefined,
		refreshLog = false,
		logs = $bindable()
	}: Props = $props()

	let lastJobId: string | undefined = $state(undefined)

	let iteration = 0
	let logOffset = 0

	async function diffJobId() {
		if (jobId != lastJobId) {
			lastJobId = jobId
			logs = undefined
			logOffset = 0
			iteration = 0
			getLogs()
		}
	}

	async function getLogs() {
		iteration += 1
		if (jobId) {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: loading ?? false,
				logOffset: logOffset == 0 ? (logs?.length ? logs?.length + 1 : 0) : logOffset
			})
			logs = (logs ?? '').concat(getUpdate.new_logs ?? '')
			logOffset = getUpdate.log_offset ?? 0
		}
		if (refreshLog) {
			setTimeout(
				() => {
					if (refreshLog) {
						getLogs()
					}
				},
				iteration < 10 ? 1000 : iteration < 20 ? 2000 : 5000
			)
		}
	}
	$effect(() => {
		jobId
		untrack(() => {
			jobId != lastJobId && diffJobId()
		})
	})
</script>
