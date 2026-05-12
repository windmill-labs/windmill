<script lang="ts">
	import { ScriptService, type NewScript, type NewScriptWithDraft, DraftService } from '$lib/gen'

	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	let topHash = page.url.searchParams.get('topHash') ?? undefined

	let hash = page.url.searchParams.get('hash') ?? undefined

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	const draftPath = hash ? '' : (page.params.path ?? '')
	const scriptHandle = UserDraft.use<EditableScript>('script', draftPath)

	let initialPath: string = $state('')

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: NewScriptWithDraft | undefined = $state(undefined)
	let fullyLoaded = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	async function loadScript(): Promise<void> {
		fullyLoaded = false
		if (hash) {
			const scriptByHash = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash
			})
			savedScript = structuredClone($state.snapshot(scriptByHash)) as NewScriptWithDraft
			scriptHandle.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const scriptWithDraft = await ScriptService.getScriptByPathWithDraft({
				workspace: $workspaceStore!,
				path: page.params.path ?? ''
			})
			savedScript = structuredClone($state.snapshot(scriptWithDraft))

			const localDraft = scriptHandle.draft
			const backendDraft = scriptWithDraft.draft
				? ({ ...scriptWithDraft.draft } as EditableScript)
				: undefined

			// Compute the fully-baked initial value once so the assignment
			// below is a single write — otherwise post-load mutations like
			// `parent_hash = ...` would count as a second write under
			// useLocalStorageValue's saveInitialValue=false contract and get
			// persisted before the user has touched anything.
			const baseline = (backendDraft ?? (scriptWithDraft as EditableScript)) as EditableScript
			const bakedBaseline: EditableScript = {
				...baseline,
				parent_hash: topHash ?? scriptWithDraft.hash
			}

			if (localDraft != undefined) {
				// Local autosave wins; offer a diff against the latest saved
				// version (backend draft if any, otherwise deployed) so the
				// user can discard it.
				const reference = backendDraft ?? scriptWithDraft
				const referenceClean = cleanValueProperties(reference)
				const localClean = cleanValueProperties(localDraft)
				if (orderedJsonStringify(referenceClean) === orderedJsonStringify(localClean)) {
					// Local matches the saved version — silently drop it and use the saved one.
					scriptHandle.draft = bakedBaseline
				} else {
					sendUserToast('Script loaded from local autosave', false, [
						{
							label: 'Discard local autosave',
							callback: () => {
								scriptHandle.draft = bakedBaseline
							}
						},
						{
							label: 'Show diff',
							callback: async () => {
								diffDrawer?.openDrawer()
								diffDrawer?.setDiff({
									mode: 'simple',
									original: referenceClean,
									current: localClean,
									title: `${backendDraft ? 'Latest saved draft' : 'Deployed'} <> Autosave`,
									button: {
										text: 'Discard autosave',
										onClick: () => {
											scriptHandle.draft = bakedBaseline
										}
									}
								})
							}
						}
					])
				}
			} else if (backendDraft) {
				scriptHandle.draft = bakedBaseline
				if (scriptHandle.draft?.['primary_schedule']) {
					savedPrimarySchedule = scriptHandle.draft['primary_schedule']
					scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
				}
				scriptBuilder?.setDraftTriggers(scriptHandle.draft.draft_triggers)

				if (!scriptWithDraft.draft_only) {
					const reloadAction = async () => {
						await DraftService.deleteDraft({
							workspace: $workspaceStore!,
							kind: 'script',
							path: scriptHandle.draft!.path
						})
						UserDraft.remove('script', draftPath)
						goto(`/scripts/edit/${scriptHandle.draft!.path}`)
						loadScript()
					}
					const deployed = cleanValueProperties(scriptWithDraft)
					const draft = cleanValueProperties(scriptHandle.draft)
					sendUserToast('Script loaded from latest saved draft', false, [
						{
							label: 'Discard draft reset to deployed version',
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
				scriptHandle.draft = bakedBaseline
			}
		}

		if (scriptHandle.draft) {
			initialPath = scriptHandle.draft.path
			scriptBuilder?.setDraftTriggers(scriptHandle.draft.draft_triggers)
			scriptBuilder?.setCode(scriptHandle.draft.content)
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
		UserDraft.remove('script', draftPath)
		goto(`/scripts/edit/${savedScript.draft.path}`)
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
		UserDraft.remove('script', draftPath)
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDraft} {restoreDeployed} />
{#if scriptHandle.draft}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		bind:script={scriptHandle.draft}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={page.url.searchParams}
		onDeploy={(e) => {
			UserDraft.remove('script', draftPath)
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSaveInitial={(e) => {
			goto(`/scripts/edit/${e.path}`)
		}}
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
	>
		<UnsavedConfirmationModal
			{diffDrawer}
			getInitialAndModifiedValues={scriptBuilder?.getInitialAndModifiedValues}
		/>
	</ScriptBuilder>
{/if}
