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
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'

	let initialState = window.location.hash != '' ? window.location.hash.slice(1) : undefined
	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	let topHash = $page.url.searchParams.get('topHash') ?? undefined

	let hash = $page.url.searchParams.get('hash') ?? undefined

	let scriptLoadedFromUrl = initialState != undefined ? decodeState(initialState) : undefined

	let script: (NewScript & { draft_triggers?: Trigger[] }) | undefined = $state(undefined)

	let initialPath: string = $state('')

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let reloadAction: () => Promise<void> = async () => {}

	let savedScript: NewScriptWithDraft | undefined = $state(undefined)
	let fullyLoaded = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

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
								diffDrawer?.openDrawer()
								diffDrawer?.setDiff({
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
				savedScript = structuredClone($state.snapshot(scriptByHash)) as NewScriptWithDraft
				script = { ...scriptByHash, parent_hash: hash, lock: undefined }
			} else {
				const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
					workspace: $workspaceStore!,
					path: $page.params.path
				})
				savedScript = structuredClone($state.snapshot(scriptWithDraft))
				if (scriptWithDraft.draft != undefined) {
					script = scriptWithDraft.draft
					scriptBuilder?.setDraftTriggers(script.draft_triggers)
					if (script['primary_schedule']) {
						savedPrimarySchedule = script['primary_schedule']
						scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
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
									diffDrawer?.openDrawer()
									diffDrawer?.setDiff({
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
			scriptBuilder?.setDraftTriggers(script.draft_triggers)
			scriptBuilder?.setCode(script.content)
			if (topHash) {
				script.parent_hash = topHash
			}
		}
		fullyLoaded = true
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadScript())
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDraft() {
		if (!savedScript || !savedScript.draft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		goto(`/scripts/edit/${savedScript.draft.path}`)
		scriptLoadedFromUrl = undefined
		loadScript()
	}

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		if (savedScript.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'script',
				path: savedScript.path
			})
		}
		goto(`/scripts/edit/${savedScript.path}`)
		scriptLoadedFromUrl = undefined
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
{#if script}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		bind:script
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={$page.url.searchParams}
		onDeploy={(e) => {
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSaveInitial={(e) => {
			goto(`/scripts/edit/${e.path}`)
		}}
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		replaceStateFn={(path) => {
			replaceState(path, $page.state)
		}}
	>
		<UnsavedConfirmationModal
			{diffDrawer}
			getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
		/>
	</ScriptBuilder>
{/if}
