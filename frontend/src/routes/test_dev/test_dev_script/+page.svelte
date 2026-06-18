<script lang="ts">
	import Dev from '$lib/components/Dev.svelte'

	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	loadUser()

	async function loadUser() {
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}
	}
</script>

<!-- <ScriptWrapper {script} neverShowMeta={true} {customUi} /> -->

<Dev
	initial={{
		type: 'script',
		script: {
			path: 'u/admin/foo',
			language: 'bun',
			content: `export async function main() {
			console.log("Hello, world!")
			return
		}`
		}
	}}
/>
