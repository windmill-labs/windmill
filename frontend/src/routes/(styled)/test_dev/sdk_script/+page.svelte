<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import ScriptWrapper from '$lib/components/ScriptWrapper.svelte'
	import type { ScriptLang } from '$lib/gen'
	import type { ScriptBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'

	loadUser()

	async function loadUser() {
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}
	}
	let script = $state({
		summary: 'foo',
		path: 'u/admin/foo',
		description: 'foo',
		language: 'python3' as ScriptLang,
		content: 'def main():\n\tprint("Hello, World!")'
	})

	let customUi: ScriptBuilderWhitelabelCustomUi = {
		tagSelectPlaceholder: 'agent',
		settingsPanel: {
			metadata: {
				disableMute: true,
				disableScriptKind: true,
				disableAiFilling: true,

				languages: ['python3']
			},
			disableMetadata: true,
			disableRuntime: true,
			disableGeneratedUi: true,
			disableTriggers: true
		},
		previewPanel: {
			tagLabel: 'agent'
		}
	}
</script>

<ScriptWrapper disableAi {script} neverShowMeta={true} {customUi} />
