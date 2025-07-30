<script lang="ts">
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowWrapper from '$lib/components/FlowWrapper.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { writable } from 'svelte/store'

	loadUser()

	async function loadUser() {
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}
	}

	let flowStore = $state({
		val: {
			summary: '',
			value: { modules: [] },
			path: 'u/admin/foo',
			edited_at: '',
			edited_by: '',
			archived: false,
			extra_perms: {},
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {},
				required: [],
				type: 'object'
			}
		}
	})

	let flowStateStore = writable({})

	let customUi: FlowBuilderWhitelabelCustomUi = {
		tagLabel: 'agent',
		// disableAi: true
	}
</script>

<!-- <ScriptWrapper {script} neverShowMeta={true} {customUi} /> -->

<FlowWrapper
	disableAi
	pathStoreInit="u/foo/bar"
	{customUi}
	selectedId={undefined}
	newFlow
	{flowStore}
	{flowStateStore}
></FlowWrapper>
