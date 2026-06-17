<script lang="ts">
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowWrapper from '$lib/components/FlowWrapper.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	// Test-dev routes don't go through the (logged) layout, so the workspace store
	// may be empty. Match what the React SDK does in main.tsx (and sdk_resource)
	// and set it explicitly — without a workspace the FlowWrapper can't acquire a
	// UserDraft handle, so autosave and its indicator stay off.
	let workspace = $state($workspaceStore ?? 'admins')

	$effect(() => {
		if (workspace && $workspaceStore !== workspace) {
			workspaceStore.set(workspace)
		}
	})

	$effect(() => {
		if ($workspaceStore && !$userStore) {
			getUserExt($workspaceStore).then((u) => ($userStore = u))
		}
	})

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

	let flowStateStore = $state({ val: {} })

	let customUi: FlowBuilderWhitelabelCustomUi = {
		tagLabel: 'agent',

		aiAgent: true
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
