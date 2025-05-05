<script lang="ts">
	import { ScriptService, type NewScript, type NewScriptWithDraft, DraftService } from '$lib/gen'

	import { page } from '$app/stores'
	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { decodeState, cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { replaceState } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { GetInitialAndModifiedValues } from '$lib/components/common/confirmationModal/unsavedTypes'
	import type { Trigger } from '$lib/components/triggers/utils'

	let initialState = window.location.hash != '' ? window.location.hash.slice(1) : undefined
	let initialArgs = {}

	if ($initialArgsStore) {
		initialArgs = $initialArgsStore
		$initialArgsStore = undefined
	}
	let topHash = $page.url.searchParams.get('topHash') ?? undefined

	let hash = $page.url.searchParams.get('hash') ?? undefined

	let scriptLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	let script: NewScript | undefined = undefined

	let initialPath: string = ''

	let scriptBuilder: ScriptBuilder | undefined = undefined

	let reloadAction: () => Promise<void> = async () => {}

	let savedScript: NewScriptWithDraft | undefined = undefined
	let fullyLoaded = false

	let savedPrimarySchedule: ScheduleTrigger | undefined = scriptLoadedFromUrl?.primarySchedule

	let savedDraftTriggers: Trigger[] = []

	async function loadScript(): Promise<void> {
		fullyLoaded = false
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
				savedScript = structuredClone(scriptByHash) as NewScriptWithDraft
				script = { ...scriptByHash, parent_hash: hash, lock: undefined }
			} else {
				const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: $page.params.path
				})
				savedScript = structuredClone(scriptWithDraft)
				if (scriptWithDraft.draft != undefined) {
					script = scriptWithDraft.draft
					if (script['primary_schedule']) {
						savedPrimarySchedule = script['primary_schedule']
						scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
					}
					if (script['draft_triggers']) {
						savedDraftTriggers = script['draft_triggers']
						scriptBuilder?.setDraftTriggers(savedDraftTriggers)
					}
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
		fullyLoaded = true
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

	let getInitialAndModifiedValues: GetInitialAndModifiedValues = undefined
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
{#if script}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		{script}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		{savedDraftTriggers}
		searchParams={$page.url.searchParams}
		on:deploy={(e) => {
			let newHash = e.detail
			goto(`/scripts/get/${newHash}?workspace=${$workspaceStore}`)
		}}
		bind:getInitialAndModifiedValues
		on:saveInitial={(e) => {
			let path = e.detail
			goto(`/scripts/edit/${path}`)
		}}
		on:seeDetails={(e) => {
			let path = e.detail
			goto(`/scripts/get/${path}?workspace=${$workspaceStore}`)
		}}
		replaceStateFn={(path) => {
			replaceState(path, $page.state)
		}}
	>
		<UnsavedConfirmationModal {diffDrawer} {getInitialAndModifiedValues} />
	</ScriptBuilder>
{/if}
