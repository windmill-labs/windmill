<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import DefaultSuperadminSkipModal from '$lib/components/DefaultSuperadminSkipModal.svelte'
	import {
		isDefaultSuperadminActive,
		REPLACE_DEFAULT_SUPERADMIN_HREF
	} from '$lib/defaultSuperadmin'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'

	// /user/* pages sit outside the (logged) layout, so a `?workspace=` param is not read there.
	function selectAdminsWorkspace(): void {
		$workspaceStore = 'admins'
	}

	let showSkipWarning = $state(false)

	async function skip() {
		if (await isDefaultSuperadminActive()) {
			showSkipWarning = true
		} else {
			goto('/user/workspaces')
		}
	}
</script>

<CenteredModal title="Welcome to Windmill">
	<p class="text-center text-secondary mt-4 mb-4">
		Configure your instance settings to get started. You can use the quick setup for essential
		settings or the advanced setup for full control.
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="default" unifiedSize="md" onClick={skip}>Skip</Button>
		<div class="flex items-center gap-2">
			<Button
				variant="default"
				unifiedSize="md"
				href="/user/instance_settings?mode=full"
				onClick={selectAdminsWorkspace}>Advanced setup</Button
			>
			<Button
				variant="accent"
				unifiedSize="md"
				href="/user/instance_settings"
				onClick={selectAdminsWorkspace}>Quick setup</Button
			>
		</div>
	</div>
</CenteredModal>

<DefaultSuperadminSkipModal
	open={showSkipWarning}
	onSkip={() => {
		showSkipWarning = false
		goto('/user/workspaces')
	}}
	onSetAccount={() => {
		showSkipWarning = false
		selectAdminsWorkspace()
		goto(REPLACE_DEFAULT_SUPERADMIN_HREF)
	}}
	onCancel={() => (showSkipWarning = false)}
/>
