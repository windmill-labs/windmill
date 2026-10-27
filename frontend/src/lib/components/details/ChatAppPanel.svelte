<script lang="ts">
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { createChatRawApp } from './createRawAppFromScript'

	/** The Windmill app tab of an agent's or a chat-mode flow's page: builds a chat app over it. */
	let {
		kind,
		path,
		workspace,
		summary = undefined,
		schema = undefined,
		onNavigate = (url) => goto(url)
	}: {
		kind: 'agent' | 'flow'
		path: string
		workspace: string
		summary?: string
		schema?: Record<string, any>
		onNavigate?: (url: string) => void | Promise<void>
	} = $props()

	const editRights = useEditRights(() => workspace)
	let canBuild = $derived(editRights.roleCanAuthor('raw_app'))

	async function buildApp() {
		const app = createChatRawApp({ path, runType: kind, summary, schema })
		// The raw app editor reads the payload from sessionStorage on mount.
		sessionStorage.setItem('rawAppImport', JSON.stringify(app))
		await onNavigate('/apps_raw/add')
	}
</script>

<div class="flex flex-col gap-4 p-4">
	<span class="text-xs text-secondary">
		A chat app for your team, built on Windmill's chat SDK. It runs the {kind} as the app, so people
		need no access to the {kind}, and keeps their conversations in their browser. You can shape it
		like any other app.
	</span>
	<div class="flex flex-col items-start gap-1">
		<Button unifiedSize="sm" variant="default" disabled={!canBuild} onClick={buildApp}>
			Build an app for this {kind}
		</Button>
		{#if !canBuild}
			<span class="text-2xs text-hint">Operators cannot build apps in this workspace.</span>
		{/if}
	</div>
</div>
