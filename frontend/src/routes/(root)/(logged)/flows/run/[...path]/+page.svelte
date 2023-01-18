<script lang="ts">
	import { page } from '$app/stores'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDaysAgo,
		emptyString,
		sendUserToast
	} from '$lib/utils'
	import { FlowService, type Flow, JobService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { Button, Kbd, Skeleton } from '$lib/components/common'
	import { faEye, faPlay, faScroll } from '@fortawesome/free-solid-svg-icons'
	import SharedBadge from '$lib/components/SharedBadge.svelte'

	const path = $page.params.path
	let flow: Flow | undefined
	let runForm: RunForm | undefined
	let isValid = true
	let can_write = false

	async function loadFlow() {
		try {
			if (path) {
				flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
				can_write =
					flow.workspace_id == $workspaceStore && canWrite(flow.path, flow.extra_perms!, $userStore)
			} else {
				sendUserToast(`Failed to fetch flow path from URL`, true)
			}
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not load flow: ${err}`, true)
		}
	}

	let loading = false

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		let run = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path,
			invisibleToOwner,
			requestBody: args,
			scheduledFor
		})
		goto('/run/' + run + '?workspace=' + $workspaceStore)
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}
	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<CenteredPage>
	{#if flow}
		<div class="flex flex-row flex-wrap justify-between gap-4 mb-6">
			<div class="w-full">
				<div class="flex flex-col mt-6 mb-2 w-full">
					<div
						class="flex flex-row-reverse w-full flex-wrap md:flex-nowrap justify-between gap-x-1"
					>
						<div class="flex flex-row">
							<div>
								<Button
									startIcon={{ icon: faEye }}
									disabled={flow == undefined}
									btnClasses="mr-4"
									variant="border"
									href="/flows/get/{flow?.path}">View flow</Button
								>
							</div>
							<div>
								<Button
									startIcon={{ icon: faPlay }}
									disabled={runForm == undefined || !isValid}
									on:click={() => runForm?.run()}>Run <Kbd class="ml-2">Ctrl+Enter</Kbd></Button
								>
							</div>
						</div>
						<div class="flex flex-col">
							<h1 class="break-words py-2 mr-2">
								{defaultIfEmptyString(flow.summary, flow.path)}
							</h1>
							{#if !emptyString(flow.summary)}
								<h2 class="font-bold pb-4">{flow.path}</h2>
							{/if}
						</div></div
					>
					<div class="flex items-center gap-2">
						<span class="text-sm text-gray-500">
							{#if flow}
								Edited {displayDaysAgo(flow.edited_at || '')} by {flow.edited_by || 'unknown'}
							{/if}
						</span>

						<SharedBadge canWrite={can_write} extraPerms={flow?.extra_perms ?? {}} />
					</div>
				</div>
			</div>
			{#if !emptyString(flow.description)}
				<div class="prose text-sm box max-w-6xl w-full mt-8">
					{defaultIfEmptyString(flow.description, 'No description')}
				</div>
			{/if}
		</div>
		<RunForm
			{loading}
			autofocus
			bind:this={runForm}
			bind:isValid
			detailed={false}
			runnable={flow}
			runAction={runFlow}
		/>
	{:else}
		<Skeleton layout={[2, [3], 1, [2], 4, [4], 3, [8]]} />
	{/if}
</CenteredPage>
