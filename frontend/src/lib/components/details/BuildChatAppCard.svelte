<script lang="ts">
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import { LayoutDashboard } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { createChatRawApp } from './createRawAppFromScript'

	/** At the foot of the conversations sidebar of an agent's or a chat-mode flow's page: builds
	 *  a chat app over it. Shown only to those who may author raw apps. */
	let {
		kind,
		path,
		workspace,
		expanded,
		summary = undefined,
		schema = undefined,
		onNavigate = (url) => goto(url)
	}: {
		kind: 'agent' | 'flow'
		path: string
		workspace: string
		/** Whether the sidebar is open: a card when it is, an icon button on its collapsed strip. */
		expanded: boolean
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

{#snippet card(classes: string)}
	<div class="flex flex-col gap-2 p-3 {classes}">
		<span class="text-sm font-semibold text-emphasis">Share as an app</span>
		<span class="text-xs text-secondary">
			Turn this {kind} into a chat app you can share. Adapt its design to fit your needs.
		</span>
		<Button
			unifiedSize="sm"
			variant="default"
			startIcon={{ icon: LayoutDashboard, classes: 'text-orange-500' }}
			onClick={buildApp}
		>
			Build chat app
		</Button>
	</div>
{/snippet}

{#if canBuild}
	{#if expanded}
		{@render card('rounded-md border border-light bg-surface-tertiary')}
	{:else}
		<!-- Collapsed, the icon alone does not say what it builds: it opens the card. -->
		<Popover placement="right-end" closeButton={false}>
			{#snippet trigger()}
				<Button
					nonCaptureEvent
					unifiedSize="md"
					variant="subtle"
					iconOnly
					startIcon={{ icon: LayoutDashboard, classes: 'ml-[2px] text-orange-500' }}
					title="Share as an app"
					btnClasses="w-fit px-2"
				/>
			{/snippet}
			{#snippet content()}
				{@render card('w-60')}
			{/snippet}
		</Popover>
	{/if}
{/if}
