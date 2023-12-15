<script lang="ts">
	import { JobService, type Script } from '$lib/gen'
	import { Badge, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { displayDate, sleep, sendUserToast } from '$lib/utils'
	import TableCustom from './TableCustom.svelte'
	import { Hourglass, Loader2, Play } from 'lucide-svelte'

	let dispatch = createEventDispatcher()
	let drawer: Drawer

	let script: Script
	let loadQueuedJobs = true
	let queuedJobs: {
		status: 'running' | 'queued'
		jobId: string
		scheduledFor: string
		scriptHash: string
	}[] = []

	let cancellingInProgress = false

	async function continuouslyLoadQueuedJobs() {
		while (loadQueuedJobs) {
			let qjs = await JobService.listQueue({
				workspace: $workspaceStore ?? '',
				orderDesc: false,
				scriptPathExact: script.path
			})
			let loadingQueuedJobs: {
				status: 'running' | 'queued'
				jobId: string
				scheduledFor: string
				scriptHash: string
			}[] = []
			for (const qj of qjs) {
				loadingQueuedJobs.push({
					status: qj.started_at ? 'running' : 'queued',
					jobId: qj.id,
					scriptHash: qj.script_hash ?? '',
					scheduledFor: displayDate(qj.scheduled_for, true)
				})
			}
			queuedJobs = loadingQueuedJobs
			await sleep(3 * 1000)
		}
	}

	async function scaleToZero() {
		cancellingInProgress = true
		await JobService.cancelPersistentQueuedJobs({
			workspace: $workspaceStore ?? '',
			path: script.path,
			requestBody: {
				reason: undefined
			}
		})
		sendUserToast(`All jobs cancelled for ${script.path}`)
		cancellingInProgress = false
	}

	export async function open(persistentScript: Script | undefined) {
		if (persistentScript === undefined) {
			console.log('Unable to open persistent script drawer without a proper script definition')
			return
		}
		console.log('opening drawer')
		script = persistentScript!
		loadQueuedJobs = true
		continuouslyLoadQueuedJobs()
		drawer.openDrawer?.()
	}

	async function exit() {
		loadQueuedJobs = false
		console.log('closing drawer')
		drawer.closeDrawer?.()
	}
</script>

<Drawer
	bind:this={drawer}
	on:close={() => {
		loadQueuedJobs = false
		dispatch('close')
	}}
	size="800px"
>
	<DrawerContent
		title="Persistent script"
		overflow_y={false}
		on:close={exit}
		tooltip="Manage runs of persistent scripts. Scaling a persistent script to zero will cancel all current runs of this script based on the script path."
	>
		<h2 class="flex gap-2 items-center">
			<Loader2 class={loadQueuedJobs ? 'animate-spin' : ''} />Queued jobs for {script.path}
		</h2>
		<TableCustom>
			<tr slot="header-row">
				<th class="text-xs">Script Hash</th>
				<th class="text-xs">Job ID</th>
				<th class="text-xs">Status</th>
				<th class="text-xs">Scheduled For</th>
			</tr>
			<tbody slot="body">
				{#each queuedJobs as { jobId, status, scriptHash, scheduledFor }}
					<tr class="">
						<td class="text-xs">
							<a
								class="pr-3"
								href="/scripts/get/{scriptHash}?workspace={$workspaceStore}"
								target="_blank"
							>
								{scriptHash}
							</a>
						</td>
						<td class="text-xs">
							<a class="pr-3" href="/run/{jobId}?workspace={$workspaceStore}" target="_blank"
								>{jobId.substring(24)}</a
							>
						</td>
						<td class="text-xs">
							{#if status === 'running'}
								<Badge color="yellow" baseClass="!px-1.5">
									<Play size={14} />
								</Badge>
							{:else}
								<Badge baseClass="!px-1.5">
									<Hourglass size={14} />
								</Badge>
							{/if}
						</td>
						<td class="text-xs">{scheduledFor}</td>
					</tr>
				{/each}
			</tbody>
		</TableCustom>

		<div slot="actions" class="flex gap-1">
			<Button
				color="red"
				disabled={cancellingInProgress === true || queuedJobs.length === 0}
				on:click={scaleToZero}
			>
				{#if cancellingInProgress}
					<Loader2 class="animate-spin" /> Stopping jobs
				{:else}
					Scale down to 0
				{/if}
			</Button>
		</div>
	</DrawerContent>
</Drawer>
