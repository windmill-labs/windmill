<script lang="ts">
	import { onMount, untrack } from 'svelte'
	import { Badge, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import {
		WorkspaceService,
		type DeploymentRequest,
		type DeploymentRequestComment,
		type DeploymentRequestEligibleDeployer
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Send, Plus, X, ArchiveX } from 'lucide-svelte'

	interface Props {
		forkWorkspaceId: string
		parentWorkspaceId: string
		currentUsername: string
		isAdmin: boolean
		onStateChange?: (hasOpenRequest: boolean) => void
	}

	let { forkWorkspaceId, parentWorkspaceId, currentUsername, isAdmin, onStateChange }: Props =
		$props()

	let request = $state<DeploymentRequest | null>(null)
	let loading = $state(true)
	let eligibleDeployers = $state<DeploymentRequestEligibleDeployer[]>([])
	let showRequestDialog = $state(false)
	let selectedAssignees = $state<string[]>([])
	let assigneeToAdd = $state<string | undefined>(undefined)
	let newComment = $state('')
	let posting = $state(false)
	let replyParentId = $state<number | undefined>(undefined)
	let replyBody = $state('')

	let canCancel = $derived(request != null && (isAdmin || request.requested_by === currentUsername))

	async function load() {
		loading = true
		try {
			request = await WorkspaceService.getOpenDeploymentRequest({ workspace: forkWorkspaceId })
		} catch (e: any) {
			console.error('Failed to load open deployment request', e)
			request = null
		} finally {
			loading = false
			onStateChange?.(request != null)
		}
	}

	async function loadEligibleDeployers() {
		try {
			eligibleDeployers = await WorkspaceService.listDeploymentRequestEligibleDeployers({
				workspace: forkWorkspaceId
			})
		} catch (e: any) {
			console.error('Failed to load eligible deployers', e)
			eligibleDeployers = []
		}
	}

	onMount(() => {
		load()
		loadEligibleDeployers()
	})

	$effect(() => {
		if (assigneeToAdd && !selectedAssignees.includes(assigneeToAdd)) {
			const v = assigneeToAdd
			untrack(() => {
				selectedAssignees = [...selectedAssignees, v]
				assigneeToAdd = undefined
			})
		}
	})

	async function createRequest() {
		if (selectedAssignees.length === 0) {
			sendUserToast('Pick at least one assignee', true)
			return
		}
		posting = true
		try {
			request = await WorkspaceService.createDeploymentRequest({
				workspace: forkWorkspaceId,
				requestBody: { assignees: selectedAssignees }
			})
			sendUserToast(`Deployment request sent to ${selectedAssignees.length} assignee(s)`)
			showRequestDialog = false
			selectedAssignees = []
			onStateChange?.(true)
		} catch (e: any) {
			sendUserToast(`Failed to create deployment request: ${e.body || e.message}`, true)
		} finally {
			posting = false
		}
	}

	async function cancelRequest() {
		if (!request) return
		if (!confirm('Cancel the open deployment request? Assignees will be notified.')) return
		try {
			await WorkspaceService.cancelDeploymentRequest({
				workspace: forkWorkspaceId,
				id: request.id
			})
			sendUserToast('Deployment request cancelled')
			request = null
			onStateChange?.(false)
		} catch (e: any) {
			sendUserToast(`Failed to cancel: ${e.body || e.message}`, true)
		}
	}

	async function postComment(body: string, parentId?: number) {
		if (!request || !body.trim()) return
		posting = true
		try {
			const created = await WorkspaceService.createDeploymentRequestComment({
				workspace: forkWorkspaceId,
				id: request.id,
				requestBody: { body, parent_id: parentId }
			})
			request = {
				...request,
				comments: [...request.comments, created]
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

	let topLevel = $derived(request?.comments.filter((c) => c.parent_id == null) ?? [])
	function repliesOf(parent: DeploymentRequestComment): DeploymentRequestComment[] {
		if (!request) return []
		return request.comments.filter((c) => c.parent_id === parent.id)
	}

	function formatTimestamp(iso: string): string {
		return new Date(iso).toLocaleString()
	}

	// Has content to render inline in the deploy panel: dialog open,
	// request open, or still loading.
	let hasContent = $derived(loading || showRequestDialog || request != null)

	export function refresh() {
		load()
	}

	export function openRequestDialog() {
		showRequestDialog = true
	}

	export function isDialogOpen(): boolean {
		return showRequestDialog
	}

	export function hasOpenRequest(): boolean {
		return request != null
	}
</script>

{#if hasContent}
	<div class="flex flex-col gap-2 mt-3 pt-3 border-t">
		{#if loading}
			<div class="text-secondary text-xs">Loading deployment request…</div>
		{:else if !request}
			<!-- showRequestDialog == true -->
			<div class="flex flex-col gap-2 border rounded-md p-3 bg-surface">
				<div class="text-xs text-secondary">
					Pick one or more assignees. They must be admins or members of <code>wm_deployers</code>
					in <b>{parentWorkspaceId}</b>.
				</div>
				<Select
					bind:value={assigneeToAdd}
					items={safeSelectItems(
						eligibleDeployers
							.filter((d) => !selectedAssignees.includes(d.username))
							.map((d) => ({
								label: `${d.username} <${d.email}>`,
								value: d.username
							}))
					)}
					placeholder="Add assignee…"
				/>
				{#if selectedAssignees.length > 0}
					<div class="flex flex-wrap gap-1">
						{#each selectedAssignees as a (a)}
							<Badge color="blue">
								{a}
								<button
									type="button"
									class="ml-1 hover:text-red-600"
									onclick={() => {
										selectedAssignees = selectedAssignees.filter((s) => s !== a)
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
						disabled={posting || selectedAssignees.length === 0}
						onclick={createRequest}
					>
						Send request
					</Button>
					<Button
						variant="default"
						size="xs"
						onclick={() => {
							showRequestDialog = false
							selectedAssignees = []
						}}
					>
						Cancel
					</Button>
				</div>
			</div>
		{:else}
			<div class="flex items-center justify-between flex-wrap gap-2 text-xs">
				<div class="flex items-center gap-2 flex-wrap">
					<Badge color="green" size="xs">Deployment request open</Badge>
					<span class="text-secondary">by</span>
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
				<span class="text-secondary">Assignees:</span>
				{#each request.assignees as a (a.username)}
					<Badge color="indigo" size="xs">{a.username}</Badge>
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
		{/if}
	</div>
{/if}
