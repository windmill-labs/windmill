<script lang="ts">
	import { Alert, Badge, Button } from '$lib/components/common'
	import type { AIProviderModel, WorkspaceItemDiff } from '$lib/gen'
	import { copilotInfo, copilotSessionModel } from '$lib/aiStore'
	import Select from '$lib/components/select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { Loader2, MessageSquarePlus, Sparkles, WandSparkles, X } from 'lucide-svelte'
	import OpenInSessionButton, {
		type OpenInSessionSource
	} from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { buildCompareUrl } from '$lib/components/copilot/chat/global/pageNavigation'
	import type { SessionTarget } from '$lib/components/sessions/sessionState.svelte'
	import { reviewDeployment, type AiReviewFinding, type AiReviewResult } from './aiReview'

	interface Props {
		diffs: WorkspaceItemDiff[]
		forkWorkspaceId: string
		parentWorkspaceId: string
		/** Where the review runs: the items selected for deploy, the request form, or an open request. */
		source: 'selection' | 'request_form' | 'open_request'
		/** Absent before the request exists: findings are then only shown, for the author to act on. */
		onPost?: (finding: AiReviewFinding) => Promise<void>
	}

	let { diffs, forkWorkspaceId, parentWorkspaceId, source, onPost }: Props = $props()

	let running = $state(false)
	let result = $state<AiReviewResult | undefined>(undefined)
	let error = $state<string | undefined>(undefined)
	let postingId = $state<string | undefined>(undefined)
	let abortController: AbortController | undefined

	const modelKey = (m: AIProviderModel) => `${m.provider}:${m.model}`
	// Local to this review: picking a model here must not change the chat's own selection,
	// which is only the default until something is picked.
	let pickedModelKey = $state<string | undefined>(undefined)
	let chatModel = $derived(
		$copilotSessionModel ?? $copilotInfo.defaultModel ?? $copilotInfo.aiModels[0]
	)
	let selectedModelKey = $derived(pickedModelKey ?? (chatModel && modelKey(chatModel)))

	async function run() {
		abortController?.abort()
		const controller = new AbortController()
		abortController = controller
		running = true
		error = undefined
		result = undefined
		logFeatureUsage('deployment_request', 'ai_review', { key: source })
		try {
			const review = await reviewDeployment(diffs, forkWorkspaceId, parentWorkspaceId, controller, {
				model: $copilotInfo.aiModels.find((m) => modelKey(m) === selectedModelKey)
			})
			if (!controller.signal.aborted) result = review
		} catch (e: any) {
			if (!controller.signal.aborted) error = e?.message ?? String(e)
		} finally {
			if (abortController === controller) running = false
		}
	}

	$effect(() => () => abortController?.abort())

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

	const SESSION_TARGET_KINDS: string[] = ['script', 'flow', 'raw_app']

	// The session is pinned to the fork: the fixes belong in the workspace under review, and a
	// session left to pick its own would start in whatever workspace the user last used.
	function fixSource(findings: AiReviewFinding[]): OpenInSessionSource {
		const lines = findings.map(
			(f) => `- [${f.severity}] ${f.kind && f.path ? `${f.kind} ${f.path}: ` : ''}${f.comment}`
		)
		const one = findings.length === 1 ? findings[0] : undefined
		const common = {
			workspaceId: forkWorkspaceId,
			seedPrompt: `An AI review of this workspace's changes, before deploying them to ${parentWorkspaceId}, found ${one ? 'this issue' : 'these issues'}. Fix ${one ? 'it' : 'them'} here:\n\n${lines.join('\n')}`,
			autoSend: true,
			beforeOpen: () =>
				logFeatureUsage('deployment_request', 'ai_fix_in_session', {
					key: one ? 'finding' : 'all'
				})
		}
		if (one && SESSION_TARGET_KINDS.includes(one.kind)) {
			return { ...common, target: { kind: one.kind as SessionTarget['kind'], path: one.path } }
		}
		return {
			...common,
			page: () => buildCompareUrl({ workspace_id: forkWorkspaceId, mode: 'fork' })
		}
	}
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
			{#if $copilotInfo.aiModels.length > 1}
				<div class="w-56">
					<Select
						size="sm"
						items={$copilotInfo.aiModels.map((m) => ({ label: m.model, value: modelKey(m) }))}
						bind:value={() => selectedModelKey, (v) => (pickedModelKey = v)}
					/>
				</div>
			{/if}
			{#if diffs.length === 0}
				<span class="text-xs text-secondary">
					{source === 'selection' ? 'Select items to review.' : 'No changes to review.'}
				</span>
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
			{#if result.findings.length > 1}
				<div class="self-start">
					<OpenInSessionButton
						source={fixSource(result.findings)}
						btnClasses={AIBtnClasses('default')}
						label="Fix all in AI session"
						tooltip="Open an AI session in the fork and fix every issue listed"
						btnProps={{ iconOnly: false, unifiedSize: 'xs', startIcon: { icon: WandSparkles } }}
					/>
				</div>
			{/if}
			{#if result.findings.length === 0}
				<div class="text-xs text-secondary">No issues left to review.</div>
			{:else if source === 'request_form'}
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
					<div class="text-xs text-primary whitespace-pre-wrap">{f.comment}</div>
					<div class="flex gap-2 self-end">
						<OpenInSessionButton
							source={fixSource([f])}
							btnClasses={AIBtnClasses('default')}
							label="Fix in AI session"
							tooltip="Open an AI session in the fork and fix this issue"
							btnProps={{ iconOnly: false, unifiedSize: 'xs', startIcon: { icon: WandSparkles } }}
						/>
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
