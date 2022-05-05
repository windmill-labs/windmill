<script lang="ts">
	import { page } from '$app/stores';
	import { sendUserToast } from '../../../utils';
	import { FlowService, type Flow, JobService } from '../../../gen';
	import { goto } from '$app/navigation';
	import { workspaceStore } from '../../../stores';
	import CenteredPage from '../../components/CenteredPage.svelte';
	import RunForm from '../../components/RunForm.svelte';
	import PageHeader from '../../components/PageHeader.svelte';

	const path = $page.params.path;
	let flow: Flow | undefined;

	async function loadFlow() {
		try {
			if (path) {
				flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path });
			} else {
				sendUserToast(`Failed to fetch flow path from URL`, true);
			}
		} catch (err) {
			console.error(err);
			sendUserToast(`Could not load flow: ${err}`, true);
		}
	}

	async function runFlow(scheduledForStr: string | undefined, args: Record<string, any>) {
		try {
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined;
			let run = await JobService.runFlowByPath({
				workspace: $workspaceStore!,
				path,
				requestBody: args,
				scheduledFor
			});
			sendUserToast(`Job <a href='/run/${run}'>${run}</a> was created.`);
			goto('/run/' + run);
		} catch (err) {
			sendUserToast(`Could not create job: ${err}`, true);
		}
	}

	$: {
		if ($workspaceStore) {
			loadFlow();
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Run flow {flow?.path ?? '...'}">
		<a href="/flows/get/{flow?.path}">View flow {flow?.path ?? ''} details</a>
	</PageHeader>
	<RunForm runnable={flow} runAction={runFlow} />
</CenteredPage>
