<script lang="ts">
	import { ScriptService, NewScript } from '$lib/gen'

	import { page } from '$app/stores'
	import { runFormStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'

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

	let reloadAction: () => void = () => {}

	async function loadScript(): Promise<void> {
		if (scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.path == $page.params.path) {
			script = scriptLoadedFromUrl
			reloadAction = () => {
				scriptLoadedFromUrl = undefined
				goto(`/scripts/edit/${script!.path}`)
				loadScript()
			}
			sendUserToast('Script loaded from latest autosave stored in the URL', false, [
				{
					label: 'Discard autosave and reload',
					callback: reloadAction
				},
				{
					label: 'Show diff',
					callback: async () => {
						diffDrawer.openDrawer()
						let remoteContent = await ScriptService.getScriptByPathWithDraft({
							workspace: $workspaceStore!,
							path: script!.path
						})
						diffDrawer.setDiff(
							remoteContent?.draft?.content ?? remoteContent.content,
							scriptLoadedFromUrl.content
						)
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
						reloadAction = () => {
							scriptLoadedFromUrl = undefined
							hash = scriptWithDraft.hash
							goto(`/scripts/edit/${script!.path}`)
							loadScript()
						}
						sendUserToast('Script loaded from latest saved draft', false, [
							{
								label: 'Ignore draft and load from latest deployed version',
								callback: reloadAction
							},
							{
								label: 'Show diff',
								callback: async () => {
									diffDrawer.openDrawer()
									let remoteContent = await ScriptService.getScriptByPath({
										workspace: $workspaceStore!,
										path: script!.path
									})
									diffDrawer.setDiff(remoteContent.content, script?.content ?? '')
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

	let diffDrawer: DiffDrawer
</script>

<DiffDrawer bind:this={diffDrawer} button={{ text: 'Revert', onClick: reloadAction }} />
{#if script}
	<ScriptBuilder bind:this={scriptBuilder} {topHash} {initialPath} {script} {initialArgs} />
{/if}
