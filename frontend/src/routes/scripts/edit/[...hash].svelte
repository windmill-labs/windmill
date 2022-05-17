<script lang="ts">
	import { ScriptService, type Script } from '../../../gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '../../../stores'
	import ScriptBuilder from '../../components/ScriptBuilder.svelte'

	const initialState = $page.url.searchParams.get('state')
	let scriptLoadedFromUrl = initialState != undefined ? JSON.parse(atob(initialState)) : undefined

	let script: Script | undefined

	let initialPath: string = ''

	async function loadScript(): Promise<void> {
		script =
			scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.hash == $page.params.hash
				? scriptLoadedFromUrl
				: await ScriptService.getScriptByHash({
						workspace: $workspaceStore!,
						hash: $page.params.hash
				  })
		initialPath = script!.path
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}
</script>

{#if script}
	<ScriptBuilder {initialPath} {script} />
{/if}
