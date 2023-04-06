<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'

	import { page } from '$app/stores'
	import { runFormStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState } from '$lib/utils'

	const initialState = $page.url.searchParams.get('state')
	let initialArgs = {}
	if ($runFormStore) {
		initialArgs = $runFormStore
		$runFormStore = undefined
	}
	let topHash = $page.url.searchParams.get('topHash') ?? undefined

	let scriptLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	let script: Script | undefined

	let initialPath: string = ''

	let scriptBuilder: ScriptBuilder | undefined = undefined

	async function loadScript(): Promise<void> {
		script =
			scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.hash == $page.params.hash
				? scriptLoadedFromUrl
				: await ScriptService.getScriptByHash({
						workspace: $workspaceStore!,
						hash: $page.params.hash
				  })
		if (script) {
			initialPath = script.path
			scriptBuilder?.setCode(script.content)
		}
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}
</script>

{#if script}
	<ScriptBuilder bind:this={scriptBuilder} bind:topHash {initialPath} {script} {initialArgs} />
{/if}
