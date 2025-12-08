<script lang="ts">
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import TriggerSuspendedJobsModal from './TriggerSuspendedJobsModal.svelte'

	interface Props {
		suspendedJobsModal: TriggerSuspendedJobsModal | null
	}

	let { suspendedJobsModal }: Props = $props()
</script>

<Alert type="warning" title="Trigger is in suspended mode">
	<div class="flex flex-col gap-2 items-start">
		When a trigger is in suspended mode, it will continue to accept payloads and queue jobs, but
		those jobs will not run automatically. You can review the list of suspended jobs, and resume or
		cancel them individually.
		<Button
			on:click={() => {
				suspendedJobsModal?.openModal('none')
			}}
			variant="accent"
		>
			See suspended jobs
		</Button>
	</div>
</Alert>
