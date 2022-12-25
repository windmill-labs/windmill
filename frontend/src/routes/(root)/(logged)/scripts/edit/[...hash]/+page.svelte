<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'

	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeArgs, decodeState } from '$lib/utils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'

	const initialState = $page.url.searchParams.get('state')
	const initialArgs = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	let scriptLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

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
		$dirtyStore = false
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}
</script>

{#if script}
	<ScriptBuilder {initialPath} {script} {initialArgs} />
{/if}
