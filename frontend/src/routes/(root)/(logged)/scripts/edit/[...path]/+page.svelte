<script lang="ts">
	import { ScriptService, NewScript, type NewScriptWithDraft, DraftService } from '$lib/gen'

	import { page } from '$app/stores'
	import { runFormStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState, cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { cloneDeep } from 'lodash'

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

	let savedScript: NewScriptWithDraft | undefined = undefined

	async function loadScript(): Promise<void> {
		if (scriptLoadedFromUrl != undefined && scriptLoadedFromUrl.path == $page.params.path) {
			script = scriptLoadedFromUrl
			reloadAction = async () => {
				scriptLoadedFromUrl = undefined
				goto(`/scripts/edit/${script!.path}`)
				loadScript()
			}

			async function compareAutosave() {
				savedScript = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: script!.path
				})

				const draftOrDeployed = cleanValueProperties(savedScript?.draft || savedScript)
				const urlScript = cleanValueProperties(scriptLoadedFromUrl)
				if (orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(urlScript)) {
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
								diffDrawer.openDrawer()
								diffDrawer.setDiff({
									mode: 'simple',
									original: draftOrDeployed,
									current: urlScript,
									title: `${savedScript?.draft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
									button: { text: 'Discard autosave', onClick: reloadAction }
								})
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
				savedScript = cloneDeep(scriptByHash) as NewScriptWithDraft
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
						const deployed = cleanValueProperties(scriptWithDraft)
						const draft = cleanValueProperties(script)
						sendUserToast('Script loaded from latest saved draft', false, [
							{
								label: 'Discard draft and load from latest deployed version',
								callback: reloadAction
							},
							{
								label: 'Show diff',
								callback: async () => {
									diffDrawer.openDrawer()
									diffDrawer.setDiff({
										mode: 'simple',
										original: deployed,
										current: draft,
										title: 'Deployed <> Draft',
										button: { text: 'Discard draft', onClick: reloadAction }
									})
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

	let diffDrawer: DiffDrawer

	async function restoreDraft() {
		if (!savedScript || !savedScript.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer.closeDrawer()
		goto(`/scripts/edit/${savedScript.draft.path}`)
		loadScript()
	}

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer.closeDrawer()
		if (savedScript.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'script',
				path: savedScript.path
			})
		}
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
{#if script}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		{script}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
	/>
{/if}
