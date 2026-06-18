<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { browser } from '$app/environment'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { Sparkles, ArrowUp, Code2, LayoutDashboard, Clock } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { createSession } from '$lib/components/sessions/sessionState.svelte'
	import { getOrCreateRuntime } from '$lib/components/sessions/sessionRuntime.svelte'
	import type { LatestItem } from './homeFilter'

	let { latest = [] }: { latest?: LatestItem[] } = $props()

	const aiEnabled = isGlobalAiEnabled()

	let input = $state('')
	let submitting = $state(false)

	const EXAMPLE_PROMPTS = [
		'Create a workflow that triggers on a Discord message, checks for offensive language with an AI agent and possibly blocks them',
		'Build an app to browse and edit rows of my Postgres database',
		'Sync new Stripe customers into a Google Sheet every hour',
		'Summarize incoming support emails and post them to Slack',
		'Generate a weekly report from my analytics and email it to the team'
	]

	const reducedMotion = browser && window.matchMedia('(prefers-reduced-motion: reduce)').matches

	let placeholder = $state(reducedMotion ? EXAMPLE_PROMPTS[0] : '')
	let promptIndex = $state(0)

	// Typewriter placeholder: type a prompt, hold, delete, advance to the next.
	// Re-runs whenever promptIndex changes (set when a prompt finishes deleting).
	$effect(() => {
		if (reducedMotion || !aiEnabled) return
		const prompt = EXAMPLE_PROMPTS[promptIndex]
		let char = 0
		let deleting = false
		let timer: ReturnType<typeof setTimeout>
		const tick = () => {
			if (!deleting) {
				char++
				placeholder = prompt.slice(0, char)
				if (char >= prompt.length) {
					deleting = true
					timer = setTimeout(tick, 2800)
					return
				}
				timer = setTimeout(tick, 32)
			} else {
				char--
				placeholder = prompt.slice(0, char)
				if (char <= 0) {
					promptIndex = (promptIndex + 1) % EXAMPLE_PROMPTS.length
					return
				}
				timer = setTimeout(tick, 16)
			}
		}
		timer = setTimeout(tick, 500)
		return () => clearTimeout(timer)
	})

	async function submit() {
		const text = input.trim()
		if (!text || submitting) return
		submitting = true
		try {
			const session = createSession()
			const runtime = getOrCreateRuntime(session)
			// Fire the first message (beforeSend materialises + commits the session)
			// and navigate immediately so the session page renders the in-flight chat.
			void runtime.manager.sendRequest({ instructions: text })
			await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
		} finally {
			submitting = false
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			submit()
		}
	}

	function hrefFor(it: LatestItem): string {
		const ws = $workspaceStore
		if (it.type === 'script') {
			return it.draft_only
				? `${base}/scripts/edit/${it.path}`
				: `${base}/scripts/get/${it.hash}?workspace=${ws}`
		}
		if (it.type === 'flow') {
			return it.draft_only
				? `${base}/flows/edit/${it.path}`
				: `${base}/flows/get/${it.path}?workspace=${ws}`
		}
		const seg = it.raw_app ? '_raw' : ''
		return it.draft_only
			? `${base}/apps${seg}/edit/${it.path}`
			: `${base}/apps${seg}/get/${it.path}`
	}

	function relativeTime(t?: number): string {
		if (!t) return ''
		const diff = Date.now() - t
		const mins = Math.floor(diff / 60000)
		if (mins < 1) return 'just now'
		if (mins < 60) return `${mins}m ago`
		const hours = Math.floor(mins / 60)
		if (hours < 24) return `${hours}h ago`
		const days = Math.floor(hours / 24)
		if (days < 30) return `${days}d ago`
		const months = Math.floor(days / 30)
		if (months < 12) return `${months}mo ago`
		return `${Math.floor(months / 12)}y ago`
	}

	const typeColor: Record<LatestItem['type'], string> = {
		script: 'var(--color-gray-500, #6b7280)',
		flow: '#14b8a6',
		app: '#fb923c',
		raw_app: '#fb923c'
	}
</script>

{#if aiEnabled}
	<div class="w-full pt-6 pb-2">
		<div
			class="relative rounded-2xl border border-border bg-surface shadow-sm focus-within:border-blue-400 dark:focus-within:border-blue-500 transition-colors"
		>
			<div class="flex items-start gap-3 p-4">
				<div class="mt-1 text-blue-500 dark:text-blue-400 shrink-0">
					<Sparkles size={20} />
				</div>
				<textarea
					bind:value={input}
					onkeydown={onKeydown}
					rows={2}
					{placeholder}
					class="flex-1 resize-none bg-transparent outline-none text-primary placeholder:text-tertiary text-sm leading-relaxed min-h-[3rem] max-h-40"
				></textarea>
				<div class="shrink-0 self-end">
					<Button
						variant="accent"
						unifiedSize="md"
						iconOnly
						startIcon={{ icon: ArrowUp }}
						disabled={!input.trim() || submitting}
						on:click={submit}
						aiId="home-ai-new-session"
						aiDescription="Start a new AI session from the home chatbox"
					/>
				</div>
			</div>
			<div class="px-4 pb-2 -mt-1 text-2xs text-tertiary">
				Describe what you want to build — Windmill AI will create it for you. Press Enter to start.
			</div>
		</div>
	</div>
{/if}

{#if latest.length > 0}
	<div class="w-full pt-4 pb-2">
		<div class="flex items-center gap-2 mb-2 text-secondary">
			<Clock size={14} />
			<span class="text-xs font-semibold uppercase tracking-wide">Latest edited</span>
		</div>
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
			{#each latest as it (it.type + '/' + it.path)}
				<a
					href={hrefFor(it)}
					class="flex items-center gap-3 rounded-md border border-border bg-surface hover:bg-surface-hover transition-colors p-3 min-w-0"
				>
					<div class="shrink-0" style="color: {typeColor[it.type]}">
						{#if it.type === 'script'}
							<Code2 size={18} />
						{:else if it.type === 'flow'}
							<BarsStaggered size={18} />
						{:else}
							<LayoutDashboard size={18} />
						{/if}
					</div>
					<div class="min-w-0 flex-1">
						<div class="text-sm text-primary truncate font-medium">
							{it.summary || it.path}
						</div>
						<div class="text-2xs text-tertiary truncate">{it.path}</div>
					</div>
					<div class="shrink-0 text-2xs text-tertiary whitespace-nowrap">
						{relativeTime(it.time)}
					</div>
				</a>
			{/each}
		</div>
	</div>
{/if}
