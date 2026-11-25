<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type { Snippet } from 'svelte'
	import type { PersistedArtifact } from '$lib/components/copilot/chat/artifacts/artifactsDB'
	import SharedSessionStatusBar from './SharedSessionStatusBar.svelte'
	import SharedSessionPreview from './SharedSessionPreview.svelte'
	import AIChatMessage from '$lib/components/copilot/chat/AIChatMessage.svelte'
	import ToolGroupDisplay from '$lib/components/copilot/chat/ToolGroupDisplay.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { BAR_TOP_PAD } from '$lib/components/pageHeaderRegistry.svelte'
	import { sendUserToast } from '$lib/toast'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import { setUntrustedImages } from '$lib/components/copilot/chat/untrustedImages'
	import { readOnlyChatViewHost } from '$lib/components/copilot/chat/readOnlyChatViewHost.svelte'
	import { chatItemKey, groupToolRuns } from '$lib/components/copilot/chat/toolGroups'
	import { chatGroupingTools } from '$lib/components/copilot/chat/groupingTools'
	import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
	import type { SessionSnapshot } from './sessionSharing'
	import { untrack } from 'svelte'

	interface Props {
		snapshot: SessionSnapshot
		workspace: string
		author: string
		/** See SharedSessionStatusBar. */
		editsWorkspace?: string
		readsDrafts: boolean
		draftsLockedReason?: string
		draftsSignInHref?: string
		/** Under the status bar, at the foot of the chat column. */
		footer?: Snippet
		/** At the right end of the preview's tab strip while the panel is open. */
		previewActions?: Snippet
	}

	let {
		snapshot,
		workspace,
		author,
		editsWorkspace,
		readsDrafts,
		draftsLockedReason,
		draftsSignInHref,
		footer,
		previewActions
	}: Props = $props()

	setUntrustedImages()

	let openIds: string[] = $state([])
	let activeId: string | undefined = $state(undefined)
	let chatSize = $state(100)
	let previewSize = $state(0)
	let previewWidth = $state(0)
	const collapsed = $derived(previewSize === 0)
	// As in a live session, the open panel owns the right edge from the top: the page header
	// stops where it starts and floats over the chat column, which pads itself to clear it.
	const headerInset = $derived(collapsed || previewWidth === 0 ? undefined : previewWidth)

	// Built once: the page remounts this component for another share.
	const initial = untrack(() => ({ snapshot, workspace }))
	const artifacts: PersistedArtifact[] = initial.snapshot.artifacts.map((a) => ({
		...a,
		sessionId: '',
		createdAt: 0,
		updatedAt: a.updatedAt ?? 0
	}))
	setChatViewHost(
		readOnlyChatViewHost({
			messages: initial.snapshot.messages,
			workspace: initial.workspace,
			artifacts,
			openArtifact: (id) => showArtifact(id)
		})
	)

	function setPreview(size: number) {
		previewSize = size
		chatSize = 100 - size
	}

	function showArtifact(id: string) {
		if (!artifacts.some((a) => a.id === id)) {
			sendUserToast('This artifact was not part of the share.', true)
			return
		}
		if (!openIds.includes(id)) openIds = [...openIds, id]
		activeId = id
		if (collapsed) setPreview(50)
	}

	function closeTab(id: string) {
		const at = openIds.indexOf(id)
		openIds = openIds.filter((t) => t !== id)
		if (activeId === id) activeId = openIds[Math.min(at, openIds.length - 1)]
		if (openIds.length === 0) setPreview(0)
	}

	const openTabs = $derived(openIds.flatMap((id) => artifacts.find((a) => a.id === id) ?? []))

	const messages = $derived(snapshot.messages)
	const chatItems = $derived(groupToolRuns(messages, chatGroupingTools()))

	// An answer's copy row shows once per turn, on its last answer, as in the live chat.
	const showsAnswerActions = $derived.by(() => {
		const shows: boolean[] = new Array(messages.length).fill(false)
		const answeredLater = new Set<string | undefined>()
		for (let i = messages.length - 1; i >= 0; i--) {
			const message = messages[i]
			if (message.role === 'user' || message.role === 'summary') {
				answeredLater.clear()
			} else if (message.role === 'assistant' && message.content) {
				const run = message.jobId ?? message.stepName
				shows[i] = !answeredLater.has(run)
				answeredLater.add(run)
			}
		}
		return shows
	})
</script>

{#snippet messageRow(message: DisplayMessage, messageIndex: number, inGroup: boolean)}
	<AIChatMessage
		{message}
		{messageIndex}
		availableContext={[]}
		editingMessageIndex={null}
		isLast={messageIndex === messages.length - 1}
		{inGroup}
		showAnswerActions={showsAnswerActions[messageIndex]}
	/>
{/snippet}
{#snippet groupRow(message: DisplayMessage, messageIndex: number)}
	{@render messageRow(message, messageIndex, true)}
{/snippet}

<PageHeaderContent barRightInset={headerInset} />

<!-- The live session's layout: the chat column, and beside it a preview panel that opens with the
     first artifact and collapses when its last tab closes. -->
<div class="flex h-full min-h-0 flex-row" use:splitterPointerCapture>
	<Splitpanes
		horizontal={false}
		class="session-splitter min-h-0 flex-1 {collapsed ? 'splitter-off' : ''}"
	>
		<Pane
			bind:size={chatSize}
			minSize={25}
			class="flex min-h-0 flex-col {headerInset === undefined ? '' : BAR_TOP_PAD}"
		>
			<div class="min-h-0 flex-1 overflow-y-auto">
				<!-- Fills the column so the bar rests at the bottom under a short transcript, where the
				     live session keeps it above the composer, and sticks there under a long one. -->
				<div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-7 pt-4">
					<div class="flex flex-1 flex-col pb-2">
						{#each chatItems as item (chatItemKey(item))}
							{#if item.kind === 'group'}
								<div class="mb-1 min-w-0 text-sm px-2 text-primary">
									<div class="px-[1px]">
										<ToolGroupDisplay group={item} entry={groupRow} />
									</div>
								</div>
							{:else}
								{@render messageRow(item.message, item.index, false)}
							{/if}
						{/each}
					</div>

					<!-- The transcript only links plan documents, so every other artifact is reached
					     from here. -->
					<div class="sticky bottom-0 bg-surface pb-3 pt-1">
						<SharedSessionStatusBar
							edits={snapshot.edits}
							{editsWorkspace}
							{author}
							{readsDrafts}
							{draftsLockedReason}
							{draftsSignInHref}
							{artifacts}
							openArtifact={(a) => showArtifact(a.id)}
						/>
						{@render footer?.()}
					</div>
				</div>
			</div>
		</Pane>
		<Pane
			bind:size={previewSize}
			minSize={collapsed ? 0 : 30}
			maxSize={collapsed ? 0 : 75}
			class="flex min-h-0 flex-col"
		>
			{#if openTabs.length > 0}
				<div class="flex min-h-0 flex-1 flex-col" bind:clientWidth={previewWidth}>
					<SharedSessionPreview
						tabs={openTabs}
						{activeId}
						onSelect={(id) => (activeId = id)}
						onClose={closeTab}
						onReorder={(ids) => (openIds = ids)}
						onCollapse={() => setPreview(0)}
						actions={previewActions}
					/>
				</div>
			{/if}
		</Pane>
	</Splitpanes>
</div>
