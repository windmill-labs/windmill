<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import AIChatMessage from '$lib/components/copilot/chat/AIChatMessage.svelte'
	import ToolGroupDisplay from '$lib/components/copilot/chat/ToolGroupDisplay.svelte'
	import ArtifactBody from '$lib/components/copilot/chat/artifacts/ArtifactBody.svelte'
	import { setChatViewHost } from '$lib/components/copilot/chat/chatViewHost'
	import { readOnlyChatViewHost } from '$lib/components/copilot/chat/readOnlyChatViewHost.svelte'
	import { chatItemKey, groupToolRuns } from '$lib/components/copilot/chat/toolGroups'
	import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
	import type { SessionSnapshot } from './sessionSharing'
	import { untrack } from 'svelte'

	interface Props {
		snapshot: SessionSnapshot
		workspace: string
	}

	let { snapshot, workspace }: Props = $props()

	let openArtifactId: string | undefined = $state(undefined)
	let drawer: Drawer | undefined = $state(undefined)

	// Built once: the page remounts this component for another share.
	const initial = untrack(() => ({ snapshot, workspace }))
	setChatViewHost(
		readOnlyChatViewHost({
			messages: initial.snapshot.messages,
			workspace: initial.workspace,
			artifacts: initial.snapshot.artifacts.map((a) => ({
				...a,
				sessionId: '',
				createdAt: 0,
				updatedAt: 0
			})),
			openArtifact: (id) => {
				openArtifactId = id
				drawer?.openDrawer()
			}
		})
	)

	const messages = $derived(snapshot.messages)
	const chatItems = $derived(groupToolRuns(messages))
	const openArtifact = $derived(snapshot.artifacts.find((a) => a.id === openArtifactId))

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

<div class="flex flex-col pb-2">
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

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title={openArtifact?.name ?? 'Artifact'} on:close={() => drawer?.closeDrawer()}>
		{#if openArtifact}
			<ArtifactBody content={openArtifact.content} source={openArtifact.kind !== 'md'} />
		{:else}
			<span class="text-xs text-secondary">This artifact was not part of the share.</span>
		{/if}
	</DrawerContent>
</Drawer>
