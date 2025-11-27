<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Toggle from '../Toggle.svelte'
	import type { QueuedJob } from '$lib/gen/types.gen'
	import Button from '../common/button/Button.svelte'
	import { workspaceStore } from '$lib/stores'
	import RunRow from '../runs/RunRow.svelte'
	import '../runs/runs-grid.css'
	import { JobService, TriggerService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { type JobTriggerType } from './utils'
	import { ChevronLeft, ChevronRight } from 'lucide-svelte'
	type Props = {
		suspended_mode: boolean
		triggerPath: string
		jobTriggerKind: JobTriggerType
	}

	let { suspended_mode = $bindable(), jobTriggerKind, triggerPath }: Props = $props()

	let wasInInactiveMode = $state(!suspended_mode)
	let shouldShowModal = $state(false)

	$effect(() => {
		if (suspended_mode && wasInInactiveMode) {
			shouldShowModal = true
		} else if (!suspended_mode) {
			wasInInactiveMode = true
			shouldShowModal = false
		}
	})
	let queuedJobs = $state<QueuedJob[]>([])
	let loading = $state(false)
	let error = $state<string | null>(null)
	let processingAction = $state(false)
	let isPreviousLoading = $state(false)
	let isNextLoading = $state(false)
	let workspace = $workspaceStore!
	let containerWidth = $state(1000)
	let currentPage = $state(1)
	let perPage = $state(20)
	let hasMorePages = $derived(queuedJobs.length === perPage)
	$effect(() => {
		if (shouldShowModal) {
			fetchQueuedJobs()
		}
	})

	async function fetchQueuedJobs(resetPage = false) {
		if (resetPage) {
			currentPage = 1
		}
		loading = true
		error = null

		try {
			const allSuspendedJobs = await JobService.listQueue({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath,
				running: false,
				perPage,
				page: currentPage
			})

			queuedJobs = allSuspendedJobs
		} catch (e) {
			error = `Failed to fetch queued jobs: ${e}`
			console.error('Failed to fetch queued jobs:', e)
		} finally {
			loading = false
		}
	}

	function nextPage() {
		if (hasMorePages && !loading) {
			isNextLoading = true
			currentPage += 1
			fetchQueuedJobs().finally(() => {
				isNextLoading = false
			})
		}
	}

	function prevPage() {
		if (currentPage > 1 && !loading) {
			isPreviousLoading = true
			currentPage -= 1
			fetchQueuedJobs().finally(() => {
				isPreviousLoading = false
			})
		}
	}

	async function runAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			const resumedJobs = await TriggerService.resumeSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath
			})
			sendUserToast(resumedJobs)
		} catch (e) {
			error = `Failed to run jobs: ${e}`
			console.error('Failed to run jobs:', e)
		} finally {
			processingAction = false
			closeModal()
		}
	}

	async function discardAllJobs() {
		if (queuedJobs.length === 0) return

		processingAction = true
		error = null

		try {
			await TriggerService.cancelSuspendedTriggerJobs({
				workspace,
				triggerKind: jobTriggerKind,
				triggerPath
			})

			sendUserToast(`Successfully canceled all jobs`)
		} catch (e) {
			error = `Failed to discard jobs: ${e}`
			console.error('Failed to discard jobs:', e)
		} finally {
			processingAction = false
			closeModal()
		}
	}

	function closeModal() {
		wasInInactiveMode = false
		shouldShowModal = false
	}
</script>

<Toggle bind:checked={suspended_mode} options={{ right: 'Active', left: 'Inactive' }} />
