<script lang="ts">
	import { ScriptService, NewScript, Script, type NewScriptWithDraft, DraftService } from '$lib/gen'

	import { page } from '$app/stores'
	import { runFormStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState, cleanScriptProperties } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { cloneDeep } from 'lodash'
	import { deepEqual } from 'fast-equals'
	import DiffScriptsDrawer from '$lib/components/DiffScriptsDrawer.svelte'

	const initialState = $page.url.hash != '' ? $page.url.hash.slice(1) : undefined
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

	let reloadAction: () => Promise<void> = async () => {}

	let savedScript: NewScriptWithDraft | Script | undefined = undefined

	async function loadScript(): Promise<void> {
		if (scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.path == $page.params.path) {
			script = scriptLoadedFromUrl
			reloadAction = async () => {
				scriptLoadedFromUrl = undefined
				goto(`/scripts/edit/${script!.path}`)
				loadScript()
			}

			async function compareAutosave() {
				let remoteContent = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: script!.path
				})
				savedScript = cloneDeep(remoteContent)

				const draftOrDeployed = cleanScriptProperties(remoteContent?.draft || remoteContent)
				const urlScript = cleanScriptProperties(scriptLoadedFromUrl)
				if (deepEqual(draftOrDeployed, urlScript)) {
					reloadAction()
				} else {
					sendUserToast('Script loaded from latest autosave stored in the URL', false, [
						{
							label: 'Discard browser stored autosave and reload',
							callback: reloadAction
						},
						{
							label: 'Show diff',
							callback: async () => {
								genericDiffDrawer.openDrawer()
								genericDiffDrawer.setDiff(
									draftOrDeployed,
									urlScript,
									`${remoteContent?.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`
								)
							}
						}
					])
				}
			}
			compareAutosave()
		} else {
			if (hash) {
				const scriptByHash = await ScriptService.getScriptByHash({
					workspace: $workspaceStore!,
					hash
				})
				savedScript = cloneDeep(scriptByHash)
				script = { ...scriptByHash, parent_hash: hash, lock: undefined }
			} else {
				const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: $page.params.path
				})
				savedScript = cloneDeep(scriptWithDraft)
				if (scriptWithDraft.draft != undefined) {
					script = scriptWithDraft.draft
					if (!scriptWithDraft.draft_only) {
						reloadAction = async () => {
							scriptLoadedFromUrl = undefined
							await DraftService.deleteDraft({
								workspace: $workspaceStore!,
								kind: 'script',
								path: script!.path
							})
							goto(`/scripts/edit/${script!.path}`)
							loadScript()
						}
						sendUserToast('Script loaded from latest saved draft', false, [
							{
								label: 'Discard draft and load from latest deployed version',
								callback: reloadAction
							},
							{
								label: 'Show diff',
								callback: async () => {
									genericDiffDrawer.openDrawer()
									let remoteContent = await ScriptService.getScriptByPathWithDraft({
										workspace: $workspaceStore!,
										path: script!.path
									})
									savedScript = cloneDeep(remoteContent)
									const deployed = cleanScriptProperties(remoteContent)
									const draft = cleanScriptProperties(script!)
									genericDiffDrawer.setDiff(deployed, draft, 'Deployed <> Draft')
								}
							}
						])
					}
				} else {
					script = scriptWithDraft
				}
				script.parent_hash = scriptWithDraft.hash
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
			if (topHash) {
				script.parent_hash = topHash
			}
		}
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}

	let genericDiffDrawer: DiffDrawer
	let scriptsDiffDrawer: DiffScriptsDrawer
</script>

<DiffDrawer bind:this={genericDiffDrawer} button={{ text: 'Revert', onClick: reloadAction }} />
<DiffScriptsDrawer bind:this={scriptsDiffDrawer} {loadScript} />
{#if script}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		{script}
		bind:savedScript
		{initialArgs}
		diffDrawer={scriptsDiffDrawer}
	/>
{/if}
