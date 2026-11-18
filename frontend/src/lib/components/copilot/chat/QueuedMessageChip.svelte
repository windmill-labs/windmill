<script lang="ts">
	import { Button, Kbd } from '$lib/components/common'
	import { FileText, X } from 'lucide-svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import { contextElementKey } from './context'
	import { getChatViewHost } from './chatViewHost'
	import { isMac } from '$lib/utils'

	interface Props {
		/** The composer's textarea has focus. */
		composerFocused: boolean
		onFocusComposer: () => void
	}

	let { composerFocused, onFocusComposer }: Props = $props()

	// The single message typed while a turn was streaming, waiting to be
	// auto-sent when the turn finishes (or, marked with Ctrl/⌘+Enter, into the
	// turn once its running tools finish). Rendered above the whole input stack
	// (session bars, context badges, textarea) so it reads as "next in the
	// conversation". Pressing Enter again appends another line to it; its X (or
	// ArrowUp in the empty input) removes it and restores its content into the
	// input so nothing is lost, and a click on the card focuses the input.
	const chatHost = getChatViewHost()

	type Hint = 'sendsMidTurn' | 'stop' | 'midTurn' | 'edit'

	// Only while a turn runs: Esc does nothing once it has finished, and the queue is
	// about to auto-send anyway. Key hints show only while the composer has focus, the
	// only place ↑ and Ctrl/⌘+Enter act.
	const hints = $derived.by(() => {
		const list: Hint[] = []
		if (!chatHost.loading) return list
		if (chatHost.queuedSendsMidTurn) {
			list.push('sendsMidTurn')
		} else if (composerFocused) {
			if (chatHost.stopSendsQueuedMessage) list.push('stop')
			if (chatHost.supportsMidTurnMessages) list.push('midTurn')
		}
		if (composerFocused) list.push('edit')
		return list
	})

	// A press on the card leaves focus in the composer: a blur would hide the hint row
	// and move the card under the pointer before the click lands, so a click on a
	// context badge would miss its preview popover.
	function keepComposerFocus(e: MouseEvent) {
		e.preventDefault()
	}
</script>

<!-- Attachment-only and context-only queues have empty text; without their
     image / file / badge row the queued draft would be invisible —
     undismissable, then auto-sent as a surprise turn. Context badges render
     here only for context-ONLY queues: text queues pin the same chips, but
     those stay visible in the composer, and repeating them would read as two
     selections. -->
{#if chatHost.queuedMessage || chatHost.queuedImages.length > 0 || chatHost.queuedFiles.length > 0 || chatHost.queuedBlobs.length > 0 || (chatHost.queuedContext?.length ?? 0) > 0}
	<!-- The body and the X are sibling buttons (an X inside a clickable chip would be a
	     nested interactive control, invalid ARIA). -->
	<div
		class="mb-1 rounded-md bg-surface-input px-3 py-2 opacity-80 hover:opacity-100"
		role="presentation"
		onmousedown={keepComposerFocus}
	>
		<div class="flex flex-row items-start gap-1">
			{#if chatHost.queuedMessage || chatHost.queuedImages.length > 0 || chatHost.queuedFiles.length > 0 || chatHost.queuedBlobs.length > 0}
				<button
					type="button"
					class="min-w-0 grow text-left cursor-pointer"
					title={chatHost.queuedMessage}
					aria-label="Focus the message input"
					onclick={onFocusComposer}
				>
					{#if chatHost.queuedImages.length > 0}
						<div class="flex flex-row flex-wrap gap-1 {chatHost.queuedMessage ? 'mb-1' : ''}">
							{#each chatHost.queuedImages as image, i (i)}
								<img
									src={image.dataUrl}
									alt={image.name ?? 'queued image'}
									class="h-6 w-6 object-cover rounded border border-border-light"
								/>
							{/each}
						</div>
					{/if}
					{#if chatHost.queuedFiles.length > 0}
						<div class="flex flex-row flex-wrap gap-1 {chatHost.queuedMessage ? 'mb-1' : ''}">
							{#each chatHost.queuedFiles as file, i (i)}
								<span
									class="flex flex-row items-center gap-1 px-1.5 rounded border border-border-light text-2xs text-secondary max-w-36"
									title={file.name}
								>
									<FileText size={10} class="shrink-0" />
									<span class="truncate min-w-0">{file.name}</span>
								</span>
							{/each}
						</div>
					{/if}
					{#if chatHost.queuedBlobs.length > 0}
						<!-- Blobs are the same chip as files: a host that forwards bytes verbatim
						     queues them here instead, and a queue of them alone must still show. -->
						<div class="flex flex-row flex-wrap gap-1 {chatHost.queuedMessage ? 'mb-1' : ''}">
							{#each chatHost.queuedBlobs as blob, i (i)}
								<span
									class="flex flex-row items-center gap-1 px-1.5 rounded border border-border-light text-2xs text-secondary max-w-36"
									title={blob.name}
								>
									<FileText size={10} class="shrink-0" />
									<span class="truncate min-w-0">{blob.name}</span>
								</span>
							{/each}
						</div>
					{/if}
					{#if chatHost.queuedMessage}
						<p class="text-xs !font-normal text-secondary whitespace-pre-wrap line-clamp-2">
							{chatHost.queuedMessage}
						</p>
					{/if}
				</button>
			{:else if chatHost.queuedContext?.length}
				<!-- Context badges are interactive themselves (popover preview), so a
				     context-only queue gets a plain row instead of the clickable body —
				     nesting the badges in it would be invalid ARIA. The X (and ArrowUp
				     in the empty input) still restores the queue. -->
				<div class="min-w-0 grow flex flex-row flex-wrap gap-1" data-context-badges>
					{#each chatHost.queuedContext as element (contextElementKey(element))}
						<ContextElementBadge contextElement={element} compact />
					{/each}
				</div>
			{/if}
			<Button
				variant="subtle"
				unifiedSize="xs"
				iconOnly
				title="Remove queued message and put it back in the input"
				startIcon={{ icon: X }}
				onClick={() => {
					chatHost.dequeueMessage()
					onFocusComposer()
				}}
			/>
		</div>
		{#if hints.length > 0}
			<!-- One nowrap group per hint: the row wraps in a narrow panel, and must not
			     leave a key at the end of one line and its label on the next. -->
			<div class="mt-1 flex flex-row flex-wrap items-center gap-1 text-2xs text-hint">
				{#each hints as hint, i (hint)}
					<span class="flex items-center gap-1 whitespace-nowrap">
						{#if hint === 'sendsMidTurn'}
							Sends when the running tools finish
						{:else if hint === 'stop'}
							<Kbd small>Esc</Kbd> to stop and send
						{:else if hint === 'midTurn'}
							<Kbd small isModifier>{isMac() ? '⌘' : 'Ctrl'}</Kbd><Kbd small>Enter</Kbd> to send without
							stopping
						{:else}
							<Kbd small>↑</Kbd> to edit
						{/if}
						{#if i < hints.length - 1}·{/if}
					</span>
				{/each}
			</div>
		{/if}
	</div>
{/if}
