<script lang="ts">
	import { ScriptService, type Script } from '../../../gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '../../../stores'
	import ScriptBuilder from '../../components/ScriptBuilder.svelte'

	const initialState = $page.url.searchParams.get('state')
	let scriptLoadedFromUrl = initialState != undefined ? JSON.parse(atob(initialState)) : undefined

	let script: Script = {
		hash: $page.params.hash,
		path: '',
		summary: '',
		content: '',
		created_by: '',
		created_at: '',
		archived: false,
		deleted: false,
		is_template: false,
		extra_perms: {}
	}

	let initialPath: string = ''
	let scriptBuilder: ScriptBuilder

	async function loadScript(): Promise<void> {
		script =
			scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.hash == script.hash
				? scriptLoadedFromUrl
				: await ScriptService.getScriptByHash({
						workspace: $workspaceStore!,
						hash: script.hash
				  })
		initialPath = script.path
		scriptBuilder.setCode(script)
	}

	$: {
		if ($workspaceStore && scriptBuilder) {
			loadScript()
		}
	}
</script>

<ScriptBuilder bind:this={scriptBuilder} {initialPath} {script} />
