<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { Moon, Sun } from 'lucide-svelte'
	import ChipPreview from './ChipPreview.svelte'

	let darkMode: boolean = $state(false)

	function toggleTheme() {
		if (!document.documentElement.classList.contains('dark')) {
			document.documentElement.classList.add('dark')
			window.localStorage.setItem('dark-mode', 'dark')
		} else {
			document.documentElement.classList.remove('dark')
			window.localStorage.setItem('dark-mode', 'light')
		}
	}

	const LONG_TEXT =
		'Could you also make sure the retry policy applies to every step of the flow, including the failure handler, and double-check that the exponential backoff caps at five minutes so we never end up hammering the upstream API when it is already struggling under load?'

	const SAMPLE_CONTEXT = [
		{
			type: 'code' as const,
			title: 'main.ts',
			content: 'export async function main() {}',
			lang: 'bun' as const
		},
		{ type: 'db' as const, title: 'postgres_main' }
	]
</script>

<DarkModeObserver bind:darkMode />

<main class="min-h-screen bg-surface-secondary p-8 flex flex-col gap-8">
	<header class="flex items-center justify-between max-w-3xl">
		<div>
			<h1 class="text-xl font-semibold text-emphasis">Queued message chip</h1>
			<p class="text-xs text-secondary">
				Each input below has its own AIChatManager frozen with <code>loading = true</code>, so the
				chip stays visible. Typing + Enter queues live (appends), the X restores to the input.
			</p>
		</div>
		<Button
			variant="default"
			startIcon={{ icon: darkMode ? Sun : Moon }}
			onclick={toggleTheme}
			title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
		>
			{darkMode ? 'Light mode' : 'Dark mode'}
		</Button>
	</header>

	<div class="flex flex-row flex-wrap gap-8 items-start">
		<ChipPreview label="Short, rich input (GLOBAL mode)" queued="Now write a haiku about ice" />
		<ChipPreview
			label="Multi-line queued message (appended lines)"
			queued={'queued line one\nqueued line two\nqueued line three'}
		/>
		<ChipPreview label="Long text (truncation)" queued={LONG_TEXT} />
		<ChipPreview
			label="With a draft in the input (X prepends)"
			queued="Add error handling to the second step"
			draft="and also rename the flow"
		/>
		<ChipPreview
			label="Plain textarea (NAVIGATOR mode)"
			mode={AIMode.NAVIGATOR}
			queued="Open the schedules page"
		/>
		<ChipPreview
			label="With @context badges"
			queued="Refactor it to use a transaction"
			context={SAMPLE_CONTEXT}
		/>
		<ChipPreview
			label="With fork bar (session)"
			queued="Deploy the flow once you are done"
			bars={['fork']}
		/>
		<ChipPreview
			label="Fork + draft bars + context (kitchen sink)"
			queued={'queued line one\nqueued line two'}
			bars={['fork', 'draft']}
			context={SAMPLE_CONTEXT}
			draft="and a draft in the input"
		/>
	</div>
</main>
