<script lang="ts">
	import { Alert } from '$lib/components/common'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
</script>

<AlignWrapper verticalAlignment="center" horizontalAlignment="center">
	<Alert
		title="Deprecated component"
		documentationLink="https://www.windmill.dev/docs/apps/app_configuration_settings/flow_status"
		tooltip="See documentation of the new component:"
		type="error"
	>
		This component is deprecated and has been removed. Please use the Flow status by Job Id
		component instead.
	</Alert>
</AlignWrapper>
