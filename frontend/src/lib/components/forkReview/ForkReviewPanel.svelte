<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { Badge, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import {
		WorkspaceService,
		type ForkReviewComment,
		type ForkReviewDeployer,
		type ForkReviewRequest
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { MessageSquare, Send, Plus, X, UserPlus, ArchiveX } from 'lucide-svelte'

	interface Props {
		forkWorkspaceId: string
		parentWorkspaceId: string
		currentUsername: string
		isAdmin: boolean
	}

	let { forkWorkspaceId, parentWorkspaceId, currentUsername, isAdmin }: Props = $props()

	let request = $state<ForkReviewRequest | null>(null)
	let loading = $state(true)
	let deployers = $state<ForkReviewDeployer[]>([])
	let showRequestDialog = $state(false)
	let selectedReviewers = $state<string[]>([])
	let reviewerToAdd = $state<string | undefined>(undefined)
	let newComment = $state('')
	let posting = $state(false)
	let replyParentId = $state<number | undefined>(undefined)
	let replyBody = $state('')

	let canCancel = $derived(request != null && (isAdmin || request.requested_by === currentUsername))

	async function load() {
		loading = true
		try {
			const res = await WorkspaceService.getOpenForkReviewRequest({ workspace: forkWorkspaceId })
			request = (res ?? null) as ForkReviewRequest | null
		} catch (e: any) {
			console.error('Failed to load open review request', e)
			request = null
		} finally {
			loading = false
		}
	}

	async function loadDeployers() {
		try {
			deployers = await WorkspaceService.listForkReviewDeployers({ workspace: forkWorkspaceId })
		} catch (e: any) {
			console.error('Failed to load deployers', e)
			deployers = []
		}
	}

	onMount(() => {
		load()
		loadDeployers()
	})

	$effect(() => {
		if (reviewerToAdd && !selectedReviewers.includes(reviewerToAdd)) {
			const v = reviewerToAdd
			untrack(() => {
				selectedReviewers = [...selectedReviewers, v]
				reviewerToAdd = undefined
			})
		}
	})

	async function createRequest() {
		if (selectedReviewers.length === 0) {
			sendUserToast('Pick at least one reviewer', true)
			return
		}
		posting = true
		try {
			request = await WorkspaceService.createForkReviewRequest({
				workspace: forkWorkspaceId,
				requestBody: { reviewers: selectedReviewers }
			})
			sendUserToast(`Review request sent to ${selectedReviewers.length} reviewer(s)`)
			showRequestDialog = false
			selectedReviewers = []
		} catch (e: any) {
			sendUserToast(`Failed to create review request: ${e.body || e.message}`, true)
		} finally {
			posting = false
		}
	}

	async function cancelRequest() {
		if (!request) return
		if (!confirm('Cancel the open review request? Reviewers will be notified.')) return
		try {
			await WorkspaceService.cancelForkReviewRequest({ workspace: forkWorkspaceId, id: request.id })
			sendUserToast('Review request cancelled')
			request = null
		} catch (e: any) {
			sendUserToast(`Failed to cancel: ${e.body || e.message}`, true)
		}
	}

	async function postComment(body: string, parentId?: number) {
		if (!request || !body.trim()) return
		posting = true
		try {
			const created = await WorkspaceService.createForkReviewComment({
				workspace: forkWorkspaceId,
				id: request.id,
				requestBody: { body, parent_id: parentId }
			})
			request = {
				...request,
				comments: [...request.comments, created as ForkReviewComment]
			}
			if (parentId == null) {
				newComment = ''
			} else {
				replyBody = ''
				replyParentId = undefined
			}
		} catch (e: any) {
			sendUserToast(`Failed to post comment: ${e.body || e.message}`, true)
		} finally {
			posting = false
		}
	}

	// Group top-level comments and their replies for rendering.
	let topLevel = $derived(request?.comments.filter((c) => c.parent_id == null) ?? [])
	function repliesOf(parent: ForkReviewComment): ForkReviewComment[] {
		if (!request) return []
		return request.comments.filter((c) => c.parent_id === parent.id)
	}

	function formatTimestamp(iso: string): string {
		return new Date(iso).toLocaleString()
	}

	export function refresh() {
		load()
	}
</script>

<div class="bg-surface-tertiary p-4 rounded-md border">
	<div class="flex items-center gap-2 mb-2">
		<MessageSquare size={16} />
		<h3 class="text-sm font-semibold">Review</h3>
		{#if request}
			<Badge color="green" size="xs">Open</Badge>
		{:else if !loading}
			<Badge color="gray" size="xs">No open request</Badge>
		{/if}
	</div>

	{#if loading}
		<div class="text-secondary text-sm p-2">Loading…</div>
	{:else if !request}
		<div class="text-secondary text-xs mb-2">
			Ask admins or members of <code>wm_deployers</code> in <b>{parentWorkspaceId}</b> to review this
			fork. They can leave comments, and one of them can merge when ready.
		</div>

		{#if !showRequestDialog}
			<Button
				variant="accent"
				size="xs"
				startIcon={{ icon: UserPlus }}
				onclick={() => {
					showRequestDialog = true
				}}
			>
				Request review
			</Button>
		{:else}
			<div class="flex flex-col gap-2 border rounded-md p-3 bg-surface">
				<div class="text-xs text-secondary">
					Pick one or more reviewers. They must be admins or members of <code>wm_deployers</code>
					in <b>{parentWorkspaceId}</b>.
				</div>
				<Select
					bind:value={reviewerToAdd}
					items={safeSelectItems(
						deployers
							.filter((d) => !selectedReviewers.includes(d.username))
							.map((d) => `${d.username} <${d.email}>`)
							.map((label) => ({
								label,
								value: label.split(' ')[0]
							}))
					)}
					placeholder="Add reviewer…"
				/>
				{#if selectedReviewers.length > 0}
					<div class="flex flex-wrap gap-1">
						{#each selectedReviewers as r (r)}
							<Badge color="blue">
								{r}
								<button
									type="button"
									class="ml-1 hover:text-red-600"
									onclick={() => {
										selectedReviewers = selectedReviewers.filter((s) => s !== r)
									}}
								>
									<X size={12} />
								</button>
							</Badge>
						{/each}
					</div>
				{/if}
				<div class="flex gap-2">
					<Button
						variant="accent"
						size="xs"
						startIcon={{ icon: Send }}
						disabled={posting || selectedReviewers.length === 0}
						onclick={createRequest}
					>
						Send request
					</Button>
					<Button
						variant="default"
						size="xs"
						onclick={() => {
							showRequestDialog = false
							selectedReviewers = []
						}}
					>
						Cancel
					</Button>
				</div>
			</div>
		{/if}
	{:else}
		<div class="flex flex-col gap-3">
			<div class="flex items-center justify-between flex-wrap gap-2 text-xs">
				<div class="flex items-center gap-2 flex-wrap">
					<span class="text-secondary">Requested by</span>
					<b>{request.requested_by}</b>
					<span class="text-secondary">at {formatTimestamp(request.requested_at)}</span>
				</div>
				{#if canCancel}
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: ArchiveX }}
						onclick={cancelRequest}
					>
						Cancel request
					</Button>
				{/if}
			</div>
			<div class="flex items-center gap-1 flex-wrap text-xs">
				<span class="text-secondary">Reviewers:</span>
				{#each request.reviewers as r (r.username)}
					<Badge color="indigo" size="xs">{r.username}</Badge>
				{/each}
			</div>

			<div class="border rounded-md bg-surface p-3 flex flex-col gap-3">
				{#if topLevel.length === 0}
					<div class="text-secondary text-xs">No comments yet. Be the first to leave feedback.</div>
				{/if}
				{#each topLevel as c (c.id)}
					<div class="border rounded-md p-2 flex flex-col gap-1" class:opacity-60={c.obsolete}>
						<div class="flex items-center justify-between gap-2 text-xs">
							<div class="flex items-center gap-2">
								<b>{c.author}</b>
								{#if c.anchor_kind && c.anchor_path}
									<Badge color="blue" size="xs">
										{c.anchor_kind}:{c.anchor_path}
									</Badge>
								{/if}
								{#if c.obsolete}
									<Badge color="gray" size="xs">obsolete</Badge>
								{/if}
							</div>
							<span class="text-tertiary text-2xs">{formatTimestamp(c.created_at)}</span>
						</div>
						<div class="text-sm whitespace-pre-wrap">{c.body}</div>

						{#each repliesOf(c) as r (r.id)}
							<div
								class="ml-4 border-l pl-2 flex flex-col gap-1"
								class:opacity-60={r.obsolete || c.obsolete}
							>
								<div class="flex items-center justify-between gap-2 text-xs">
									<b>{r.author}</b>
									<span class="text-tertiary text-2xs">{formatTimestamp(r.created_at)}</span>
								</div>
								<div class="text-sm whitespace-pre-wrap">{r.body}</div>
							</div>
						{/each}

						{#if replyParentId === c.id}
							<div class="ml-4 flex flex-col gap-1 mt-1">
								<TextInput bind:value={replyBody} inputProps={{ placeholder: 'Write a reply…' }} />
								<div class="flex gap-2">
									<Button
										size="xs"
										variant="accent"
										disabled={posting || !replyBody.trim()}
										onclick={() => postComment(replyBody, c.id)}
									>
										Reply
									</Button>
									<Button
										size="xs"
										variant="default"
										onclick={() => {
											replyParentId = undefined
											replyBody = ''
										}}
									>
										Cancel
									</Button>
								</div>
							</div>
						{:else}
							<button
								class="text-xs text-tertiary hover:text-secondary mt-1 self-start"
								onclick={() => {
									replyParentId = c.id
									replyBody = ''
								}}
							>
								Reply
							</button>
						{/if}
					</div>
				{/each}

				<div class="flex flex-col gap-2 border-t pt-2">
					<TextInput
						bind:value={newComment}
						inputProps={{ placeholder: 'Leave a general comment…' }}
					/>
					<div class="self-end">
						<Button
							size="xs"
							variant="accent"
							startIcon={{ icon: Plus }}
							disabled={posting || !newComment.trim()}
							onclick={() => postComment(newComment)}
						>
							Comment
						</Button>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
