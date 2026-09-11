<!--
@component
The Files & folders section of the assistant settings modal: everything linked to this
chat, whether it is readable, and the way to unlink it.

Attaching happens in the composer — this is where what is already attached is accounted
for, which is the one place the difference between "attached" and "readable" is visible.
-->
<script lang="ts">
	import { Button, ListRow, Section } from '$lib/components/common'
	import EmptyState from '$lib/components/common/emptyState/EmptyState.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { FileText, Folder, Paperclip, Unlink } from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { attachmentStatusLabel, countReadyAttachments } from './agentContext'

	let {
		count = $bindable(),
		blocksClose = $bindable()
	}: {
		/** Attachments the assistant can actually read, for the sidebar badge. */
		count: number
		/** True while the confirmation is up, so the modal leaves the keys to it. */
		blocksClose: boolean
	} = $props()

	const aiChatManager = getAiChatManager()

	let folders = $derived(aiChatManager.attachedFiles.folders)
	// Files dropped on a past message are listed alongside the session's own: the
	// composer's bar hides them because their chip lives on that message, but the
	// assistant reads them like any other row.
	let files = $derived([
		...aiChatManager.attachedFiles.standalone,
		...aiChatManager.attachedFiles.messageAttached
	])
	let messageScoped = $derived(
		new Set(aiChatManager.attachedFiles.messageAttached.map((f) => f.id))
	)
	let total = $derived(folders.length + files.length)
	let ready = $derived(countReadyAttachments(folders, files))

	let pendingDetach = $state<{ kind: 'file' | 'folder'; name: string } | undefined>(undefined)

	$effect(() => {
		count = ready
	})
	$effect(() => {
		blocksClose = pendingDetach !== undefined
	})

	function detach(target: { kind: 'file' | 'folder'; name: string }) {
		if (target.kind === 'folder') aiChatManager.attachedFiles.removeFolder(target.name)
		else aiChatManager.attachedFiles.removeFile(target.name)
		pendingDetach = undefined
	}
</script>

<Section
	label="Files & folders"
	description="Files and folders linked to this chat, which the assistant can open and search."
	class="flex flex-col gap-4"
>
	{#snippet action()}
		{#if total > 0}
			<!-- Both numbers when they differ: one count under a heading about what the
			     assistant can use would hide the rows it cannot open. -->
			<span class="shrink-0 text-2xs text-secondary tabular-nums">
				{ready === total ? `${total} usable` : `${ready} of ${total} usable`}
			</span>
		{/if}
	{/snippet}

	{#if total === 0}
		<EmptyState
			icon={Paperclip}
			title="Nothing attached"
			description="Drop a file or a folder on the chat, or use the paperclip in the composer, and the assistant can open and search it."
		/>
	{:else}
		<div class="flex flex-col gap-0.5">
			{#each folders as folder (folder.name)}
				{#snippet icon()}
					<Folder size={16} class="text-tertiary" />
				{/snippet}
				{#snippet title()}
					<span class="truncate leading-5">{folder.name}</span>
					{@const label = attachmentStatusLabel(folder.status)}
					{#if label}
						<span class="shrink-0 text-2xs font-normal text-hint">{label}</span>
					{/if}
				{/snippet}
				{#snippet subtitle()}
					{folder.files.length}
					{folder.files.length === 1 ? 'file' : 'files'}
				{/snippet}
				{#snippet trailing()}
					<Button
						unifiedSize="sm"
						variant="subtle"
						startIcon={{ icon: Unlink }}
						iconOnly
						title="Unlink folder"
						onClick={() => (pendingDetach = { kind: 'folder', name: folder.name })}
					/>
				{/snippet}
				<ListRow {icon} {title} {subtitle} {trailing} />
			{/each}
			{#each files as file (file.id ?? file.name)}
				{#snippet icon()}
					<FileText size={16} class="text-tertiary" />
				{/snippet}
				{#snippet title()}
					<span class="truncate leading-5">{file.name}</span>
					{@const label = attachmentStatusLabel(file.status)}
					{#if label}
						<span class="shrink-0 text-2xs font-normal text-hint">{label}</span>
					{/if}
				{/snippet}
				{#snippet subtitle()}
					Attached to a message
				{/snippet}
				{#snippet trailing()}
					<Button
						unifiedSize="sm"
						variant="subtle"
						startIcon={{ icon: Unlink }}
						iconOnly
						title="Unlink file"
						onClick={() => (pendingDetach = { kind: 'file', name: file.name })}
					/>
				{/snippet}
				<!-- A message-scoped row has no unlink here: those are rebuilt from the
				     transcript on every load, so `removeFile` refuses them and the chip on the
				     message is what actually drops one. -->
				<ListRow
					{icon}
					{title}
					subtitle={messageScoped.has(file.id) ? subtitle : undefined}
					trailing={messageScoped.has(file.id) ? undefined : trailing}
				/>
			{/each}
		</div>
	{/if}
</Section>

<ConfirmationModal
	open={pendingDetach !== undefined}
	title={pendingDetach?.kind === 'folder' ? 'Unlink folder' : 'Unlink file'}
	confirmationText="Unlink"
	onConfirmed={() => pendingDetach && detach(pendingDetach)}
	onCanceled={() => (pendingDetach = undefined)}
>
	<span class="text-xs text-primary">
		The assistant loses access to <span class="font-semibold">{pendingDetach?.name}</span>. Nothing
		is deleted from your disk, and you can attach it again from the composer.
	</span>
</ConfirmationModal>
