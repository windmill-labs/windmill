<script lang="ts">
	import { Button, Drawer, DrawerContent } from './common'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FolderEditor from './FolderEditor.svelte'
	import { Save } from 'lucide-svelte'

	let {
		offset = 0,
		disableChatOffset = false,
		onSaved = undefined,
		workspace = undefined
	}: {
		offset?: number
		disableChatOffset?: boolean
		onSaved?: (name: string, created: boolean) => void
		/** Edit a folder of this workspace rather than the active one. */
		workspace?: string
	} = $props()

	let drawer: Drawer | undefined = $state()
	let mode: 'edit' | 'new' = $state('edit')
	let name: string = $state('')
	let canSave = $state(false)
	let unsaved = $state(false)
	let saving = $state(false)
	let confirmDiscardOpen = $state(false)
	let discarding = $state(false)
	let editor: { save: () => Promise<string | undefined> } | undefined = $state()
	// Bumped per open so the editor reloads its draft from the folder it is now
	// pointed at. Keying on `name` instead would remount on every keystroke of the
	// name field in `new` mode.
	let instance = $state(0)

	function open(nextMode: 'edit' | 'new', folderName: string): void {
		mode = nextMode
		name = folderName
		discarding = false
		confirmDiscardOpen = false
		instance++
		drawer?.openDrawer()
	}

	export function initEdit(folderName: string): void {
		open('edit', folderName)
	}

	export function initNew(initialName: string = ''): void {
		open('new', initialName)
	}

	/** The editor keeps its draft in memory only, so closing throws it away. */
	function requestClose() {
		// A save is already writing. `unsaved` only clears once it reloads, so closing here
		// would offer to discard changes the in-flight requests are busy persisting — and
		// confirming would close on that lie. Saving is the shorter wait; ignore the close.
		if (saving) return
		if (discarding || !unsaved) {
			drawer?.closeDrawer()
			return
		}
		confirmDiscardOpen = true
	}

	async function save() {
		saving = true
		const created = mode === 'new'
		try {
			const saved = await editor?.save()
			if (saved) {
				onSaved?.(saved, created)
				// The editor reloads its baseline after saving, but that lands a tick
				// later; close on our own authority rather than racing it.
				discarding = true
				// Belt and braces with the `saving` guard on the close paths: nothing that
				// asked to discard may outlive a save that then succeeded.
				confirmDiscardOpen = false
				drawer?.closeDrawer()
			}
		} finally {
			saving = false
		}
	}
</script>

<Drawer
	bind:this={drawer}
	{offset}
	{disableChatOffset}
	on:close={() => {
		// Escape and click-away close the drawer before asking. Reopening in the same
		// tick is how the flow's script editor drawer handles this too: the close
		// transition has not started, so nothing flickers.
		if (saving) {
			drawer?.openDrawer()
			return
		}
		if (!discarding && unsaved) {
			drawer?.openDrawer()
			confirmDiscardOpen = true
		}
	}}
>
	<DrawerContent
		title={mode === 'new' ? 'Create folder' : `Folder ${name}`}
		on:close={requestClose}
	>
		{#key instance}
			<FolderEditor
				bind:this={editor}
				bind:name
				{mode}
				{workspace}
				onCanSaveChange={(v) => (canSave = v)}
				onUnsavedChange={(v) => (unsaved = v)}
			/>
		{/key}
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				disabled={!canSave}
				loading={saving}
				on:click={save}
			>
				{mode === 'new' ? 'Create' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<ConfirmationModal
	open={confirmDiscardOpen}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	onCanceled={() => (confirmDiscardOpen = false)}
	onConfirmed={() => {
		confirmDiscardOpen = false
		discarding = true
		drawer?.closeDrawer()
	}}
>
	<span> Are you sure you want to discard the changes you have made to this folder? </span>
</ConfirmationModal>
