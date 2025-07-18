<script lang="ts">
	import EditableSchemaSdkWrapper from '$lib/components/schema/EditableSchemaSdkWrapper.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	loadUser()

	async function loadUser() {
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}
	}

	let schema = $state({
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {},
		required: [],
		type: 'object'
	})

	let customUi = {
		noAddPopover: false
		// disableAi: true
	}
</script>

<!-- <ScriptWrapper {script} neverShowMeta={true} {customUi} /> -->

<EditableSchemaSdkWrapper {customUi} {schema} />
