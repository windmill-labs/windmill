<script lang="ts">
	import { Alert, Badge, Button } from '$lib/components/common'
	import type { WorkspaceItemDiff } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { Loader2, MessageSquarePlus, Sparkles, X } from 'lucide-svelte'
	import { reviewDeployment, type AiReviewFinding, type AiReviewResult } from './aiReview'

	interface Props {
		diffs: WorkspaceItemDiff[]
		forkWorkspaceId: string
		parentWorkspaceId: string
		/** Absent before the request exists: findings are then only shown, for the author to act on. */
		onPost?: (finding: AiReviewFinding) => Promise<void>
	}

	let { diffs, forkWorkspaceId, parentWorkspaceId, onPost }: Props = $props()

	let running = $state(false)
	let result = $state<AiReviewResult | undefined>(undefined)
	let error = $state<string | undefined>(undefined)
	let postingId = $state<string | undefined>(undefined)
	let abortController: AbortController | undefined

	async function run() {
		abortController?.abort()
		const controller = new AbortController()
		abortController = controller
		running = true
		error = undefined
		result = undefined
		logFeatureUsage('deployment_request', 'ai_review', {
			key: onPost ? 'open_request' : 'before_request'
		})
		try {
			const review = await reviewDeployment(diffs, forkWorkspaceId, parentWorkspaceId, controller)
			if (!controller.signal.aborted) result = review
		} catch (e: any) {
			if (!controller.signal.aborted) error = e?.message ?? String(e)
		} finally {
			if (abortController === controller) running = false
		}
	}

	function stop() {
		abortController?.abort()
		running = false
	}

	function dismiss(finding: AiReviewFinding) {
		if (result) result.findings = result.findings.filter((f) => f.id !== finding.id)
	}

	async function post(finding: AiReviewFinding) {
		if (!onPost) return
		postingId = finding.id
		try {
			await onPost(finding)
			logFeatureUsage('deployment_request', 'ai_finding_posted', { key: finding.severity })
			dismiss(finding)
		} catch (e: any) {
			sendUserToast(`Failed to post comment: ${e?.body || e?.message}`, true)
		} finally {
			postingId = undefined
		}
	}

	const severityColor = { high: 'red', medium: 'yellow', low: 'gray' } as const
</script>

<div class="flex flex-col gap-2">
	<div class="flex items-center gap-2">
		{#if running}
			<Button variant="default" unifiedSize="sm" startIcon={{ icon: X }} onclick={stop}>
				Stop review
			</Button>
			<span class="flex items-center gap-1 text-xs text-secondary">
				<Loader2 size={12} class="animate-spin" />
				Reviewing {diffs.length} changed item{diffs.length === 1 ? '' : 's'}…
			</span>
		{:else}
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: Sparkles }}
				disabled={diffs.length === 0}
				onclick={run}
			>
				{result ? 'Review again with AI' : 'Review with AI'}
			</Button>
			{#if diffs.length === 0}
				<span class="text-xs text-secondary">No changes to review.</span>
			{/if}
		{/if}
	</div>

	{#if error}
		<Alert type="error" title="AI review failed" size="xs">{error}</Alert>
	{/if}

	{#if result}
		<div class="flex flex-col gap-2 border rounded-md p-2 bg-surface-secondary">
			{#if result.summary}
				<div class="text-xs text-secondary">{result.summary}</div>
			{/if}
			{#if result.unreadable.length > 0}
				<Alert type="warning" title="Not reviewed" size="xs">
					Could not load the contents of: {result.unreadable.join(', ')}
				</Alert>
			{/if}
			{#if result.omitted.length > 0}
				<div class="text-2xs text-secondary">
					Reviewed by name only, too large to include: {result.omitted.join(', ')}
				</div>
			{/if}
			{#if result.findings.length === 0}
				<div class="text-xs text-secondary">No issues left to review.</div>
			{:else if !onPost}
				<div class="text-2xs text-secondary">
					Fix these in your fork before sending, or send as is. Reviewers can run their own review.
				</div>
			{/if}
			{#each result.findings as f (f.id)}
				<div class="border rounded-md p-2 flex flex-col gap-1 bg-surface">
					<div class="flex items-center gap-2 flex-wrap text-xs">
						<Badge color={severityColor[f.severity]} size="xs">{f.severity}</Badge>
						{#if f.kind && f.path}
							<Badge color="blue" size="xs">{f.kind}:{f.path}</Badge>
						{/if}
					</div>
					<div class="text-sm whitespace-pre-wrap">{f.comment}</div>
					<div class="flex gap-2 self-end">
						{#if onPost}
							<Button
								variant="default"
								unifiedSize="xs"
								startIcon={{ icon: MessageSquarePlus }}
								disabled={postingId != undefined}
								loading={postingId === f.id}
								onclick={() => post(f)}
							>
								Post as comment
							</Button>
						{/if}
						<Button variant="subtle" unifiedSize="xs" onclick={() => dismiss(f)}>Dismiss</Button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
