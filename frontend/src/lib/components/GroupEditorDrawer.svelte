<script lang="ts">
	import { Button, Drawer, DrawerContent } from './common'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import GroupEditor from './GroupEditor.svelte'
	import { Save } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	let {
		offset = 0,
		disableChatOffset = false,
		onSaved = undefined
	}: {
		offset?: number
		disableChatOffset?: boolean
		onSaved?: (name: string, created: boolean) => void | Promise<void>
	} = $props()

	let drawer: Drawer | undefined = $state()
	let mode: 'edit' | 'new' = $state('edit')
	let name: string = $state('')
	let canSave = $state(false)
	let unsaved = $state(false)
	// A `new` drawer whose group has been created but whose member calls then failed stays
	// open on the edit path. Calling it Create there would offer to create what exists.
	let exists = $state(false)
	let saving = $state(false)
	let confirmDiscardOpen = $state(false)
	let discarding = $state(false)
	let editor: { save: () => Promise<{ name: string; created: boolean } | undefined> } | undefined =
		$state()
	// Bumped per open so the editor reloads its draft from the group it is now pointed at.
	// Keying on `name` instead would remount on every keystroke of the name field in `new` mode.
	let instance = $state(0)

	function open(nextMode: 'edit' | 'new', groupName: string): void {
		mode = nextMode
		name = groupName
		discarding = false
		confirmDiscardOpen = false
		exists = nextMode === 'edit'
		// The remounted editor reports these on its first effect, which is a tick away. Until
		// then the header would carry the last group's answers.
		canSave = false
		unsaved = false
		instance++
		drawer?.openDrawer()
	}

	export function initEdit(groupName: string): void {
		open('edit', groupName)
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
		try {
			const saved = await editor?.save()
			if (saved) {
				// Callers reload a list here. Called from inside the chain, not before it, so a
				// synchronous throw is caught too — thrown out of `save()` it would skip the
				// close below and strand the drawer open on a group that did save.
				void Promise.resolve()
					.then(() => onSaved?.(saved.name, saved.created))
					.catch((e) => sendUserToast(e?.body ?? String(e), true))
				// The editor reloads its baseline after saving, but that lands a tick later;
				// close on our own authority rather than racing it.
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
		// Escape and click-away close the drawer before asking. Reopening in the same tick is
		// how the flow's script editor drawer handles this too: the close transition has not
		// started, so nothing flickers.
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
	<DrawerContent title={exists ? `Group ${name}` : 'Create group'} on:close={requestClose}>
		<!-- `save()` snapshots the draft and then awaits several requests. An edit landing in
		     that window would not be in the snapshot, and the drawer closes on success — so it
		     would be lost without ever being offered as unsaved. `inert` keeps the form from
		     taking one. -->
		<div inert={saving} class={saving ? 'opacity-60 transition-opacity' : 'transition-opacity'}>
			{#key instance}
				<GroupEditor
					bind:this={editor}
					bind:name
					{mode}
					onCanSaveChange={(v) => (canSave = v)}
					onUnsavedChange={(v) => (unsaved = v)}
					onExistsChange={(v) => (exists = v)}
				/>
			{/key}
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				disabled={!canSave}
				loading={saving}
				on:click={save}
			>
				{exists ? 'Save' : 'Create'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<!-- `alwaysPortal`: this drawer is opened from inside another drawer (the folder editor, itself
     reachable from a resource or variable form), and that outer drawer is a stacking context
     this dialog cannot climb out of on z-index alone. Left in place it paints under the drawer
     whose unsaved changes it is asking about, which leaves that drawer impossible to close. -->
<ConfirmationModal
	alwaysPortal
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
	<span> Are you sure you want to discard the changes you have made to this group? </span>
</ConfirmationModal>
