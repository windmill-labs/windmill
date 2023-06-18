<script lang="ts">
	import { ScriptService, NewScript } from '$lib/gen'

	import { page } from '$app/stores'
	import { runFormStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'

	const initialState = $page.url.searchParams.get('state')
	let initialArgs = {}
	if ($runFormStore) {
		initialArgs = $runFormStore
		$runFormStore = undefined
	}
	let topHash = $page.url.searchParams.get('topHash') ?? undefined

	let hash = $page.url.searchParams.get('hash') ?? undefined

	let scriptLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	let script: NewScript | undefined = undefined

	let initialPath: string = ''

	let scriptBuilder: ScriptBuilder | undefined = undefined

	async function loadScript(): Promise<void> {
		if (scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.path == $page.params.path) {
			script = scriptLoadedFromUrl
			sendUserToast('Script loaded from latest autosave stored in the URL', false, [
				{
					label: 'Discard autosave and reload',
					callback: () => {
						scriptLoadedFromUrl = undefined
						goto(`/scripts/edit/${script!.path}`)
						loadScript()
					}
				}
			])
			topHash = scriptLoadedFromUrl.hash
		} else {
			if (hash) {
				const scriptByHash = await ScriptService.getScriptByHash({
					workspace: $workspaceStore!,
					hash
				})
				script = { ...scriptByHash, lock: undefined }
			} else {
				const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: $page.params.path
				})
				if (scriptWithDraft.draft != undefined) {
					script = scriptWithDraft.draft
					if (!scriptWithDraft.draft_only) {
						sendUserToast('Script loaded from latest saved draft', false, [
							{
								label: 'Ignore draft and load from latest deployed version',
								callback: () => {
									scriptLoadedFromUrl = undefined
									hash = scriptWithDraft.hash
									console.log(hash)
									goto(`/scripts/edit/${script!.path}`)
									loadScript()
								}
							}
						])
					}
				} else {
					script = scriptWithDraft
				}
				topHash = scriptWithDraft.hash
			}
		}
		// hash
		// ? await ScriptService.getScriptByHash({
		// 		workspace: $workspaceStore!,
		// 		hash: $page.params.hash
		//   })
		// : await ScriptService.getScriptByPathWithDraft({
		// 		workspace: $workspaceStore!,
		// 		path: $page.params.path
		//   })

		if (script) {
			initialPath = script.path
			scriptBuilder?.setCode(script.content)
			script.parent_hash = topHash
		}
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}
</script>

{#if script}
	<ScriptBuilder bind:this={scriptBuilder} {topHash} {initialPath} {script} {initialArgs} />
{/if}
