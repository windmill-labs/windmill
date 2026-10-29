<!--
@component
The Danger zone of the assistant settings modal: workspace-wide switches an admin reaches from the
chat itself. Each one is also in the workspace's AI settings, which stay the place to undo it.
-->
<script lang="ts">
	import { Section } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { getAiChatManager } from './aiChatManagerContext'

	let { ws }: { ws: string } = $props()

	const aiChatManager = getAiChatManager()

	let confirming = $state(false)
	let saving = $state(false)

	// Until a staged fork's first send creates it, `ws` is the parent: hiding here would hide AI
	// for every member of the parent. Read at click, since the fork commits mid-session.
	function openConfirm() {
		const parent = aiChatManager.sessionContextResolver?.()?.pendingForkOf
		if (parent !== undefined) {
			sendUserToast(
				`This session has not created its workspace yet, so AI sessions would be hidden in "${parent}" instead. Send a message first.`,
				true
			)
			return
		}
		confirming = true
	}

	async function hideSessions() {
		saving = true
		try {
			// The endpoint replaces the whole config, so the stored one goes back with the flag set.
			const stored = (await WorkspaceService.getSettings({ workspace: ws })).ai_config ?? {}
			await WorkspaceService.editCopilotConfig({
				workspace: ws,
				requestBody: { ...stored, copilot_disabled: true }
			})
			// Every AI entry point read the config when it mounted; a reload is what drops them all.
			window.location.assign('/')
		} catch (e) {
			sendUserToast(`Could not hide AI sessions: ${e}`, true)
			saving = false
			confirming = false
		}
	}
</script>

<Section label="Danger zone" description="Settings that apply to every member of this workspace.">
	<div
		class="flex items-start justify-between gap-4 rounded-md border border-red-300 dark:border-red-800 p-3"
	>
		<div class="flex flex-col gap-1 min-w-0">
			<span class="text-xs font-semibold text-emphasis">Hide AI sessions</span>
			<span class="text-xs text-secondary">
				Hides AI sessions and every other AI assistant button from all members of
				<span class="font-mono">{ws}</span>. AI agent steps in flows keep working. Turn it back on
				in the workspace settings, under Windmill AI.
			</span>
		</div>
		<Button
			variant="default"
			destructive
			unifiedSize="sm"
			btnClasses="shrink-0"
			onClick={openConfirm}
		>
			Hide AI sessions
		</Button>
	</div>
</Section>

<ConfirmationModal
	open={confirming}
	title="Hide AI sessions"
	confirmationText="Hide for everyone"
	loading={saving}
	onConfirmed={() => void hideSessions()}
	onCanceled={() => (confirming = false)}
>
	<span class="text-xs text-primary">
		Every member of <span class="font-semibold">{ws}</span> loses AI sessions and the AI assistant buttons.
		The page reloads on Home.
	</span>
</ConfirmationModal>
