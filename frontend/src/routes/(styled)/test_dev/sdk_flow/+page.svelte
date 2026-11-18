<script lang="ts">
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowWrapper from '$lib/components/FlowWrapper.svelte'

	// Auth (workspace + token + user) is wired by the shared TestDevHeader in the
	// test_dev layout. Without a workspace FlowWrapper can't acquire a UserDraft
	// handle, so autosave and its indicator stay off until you log in there.

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
