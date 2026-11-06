<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		open: boolean
		onSkip: () => void
		onSetAccount: () => void
		onCancel: () => void
	}
	let { open, onSkip, onSetAccount, onCancel }: Props = $props()
</script>

<ConfirmationModal
	{open}
	title="Default superadmin account still active"
	confirmationText="Skip anyway"
	onConfirmed={onSkip}
	onCanceled={onCancel}
>
	<div class="flex flex-col gap-4">
		<span>
			The default account <b>admin@windmill.dev</b> / <b>changeme</b> is still active. Anyone who can
			reach this instance can log in with it as superadmin. You can skip now and replace it later from
			the instance settings.
		</span>
		<div class="flex">
			<Button variant="default" unifiedSize="sm" onClick={onSetAccount}>Set the account now</Button>
		</div>
	</div>
</ConfirmationModal>
