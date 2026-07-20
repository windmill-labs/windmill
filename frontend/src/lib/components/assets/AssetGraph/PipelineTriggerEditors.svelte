<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { NativeTriggerKind } from './types'
	import {
		EmailTriggerService,
		GcpTriggerService,
		KafkaTriggerService,
		MqttTriggerService,
		NatsTriggerService,
		PostgresTriggerService,
		ScheduleService,
		SqsTriggerService
	} from '$lib/gen'
	import KafkaTriggerEditor from '$lib/components/triggers/kafka/KafkaTriggerEditor.svelte'
	import MqttTriggerEditor from '$lib/components/triggers/mqtt/MqttTriggerEditor.svelte'
	import NatsTriggerEditor from '$lib/components/triggers/nats/NatsTriggerEditor.svelte'
	import PostgresTriggerEditor from '$lib/components/triggers/postgres/PostgresTriggerEditor.svelte'
	import SqsTriggerEditor from '$lib/components/triggers/sqs/SqsTriggerEditor.svelte'
	import GcpTriggerEditor from '$lib/components/triggers/gcp/GcpTriggerEditor.svelte'
	import EmailTriggerEditor from '$lib/components/triggers/email/EmailTriggerEditor.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import WebhookEditor from '$lib/components/triggers/webhook/WebhookEditor.svelte'
	import { setTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'

	// Owns the native-trigger drawer wiring for the pipeline canvas: the nine
	// editor instances, the create/edit dispatch by kind, and the delete
	// confirmation flow. The page drives it imperatively via `bind:this` so
	// the draft-guard toasts (which depend on the page's drafts map) stay in
	// the page while the editor plumbing lives here.
	//
	// `mountTriggerEditors` gates the eight editor instances: they're
	// edit-mode-only on the canvas, so the page unmounts them off edit mode
	// (matching the previous `{#if mode === 'edit'}` wrapper). The webhook
	// editor stays mounted in every mode — its node is clickable in view mode
	// too (informational endpoint URLs/token).
	type Props = { onUpdate: () => void; mountTriggerEditors: boolean; workspace?: string }
	let { onUpdate, mountTriggerEditors, workspace: triggerWorkspace }: Props = $props()

	// Register the trigger-workspace resolver for the whole editor subtree (the
	// nine editors + the delete handler below). See triggerWorkspace.ts.
	setTriggerWorkspace(() => triggerWorkspace ?? $workspaceStore)

	let kafkaEditor: KafkaTriggerEditor | undefined = $state()
	let mqttEditor: MqttTriggerEditor | undefined = $state()
	let natsEditor: NatsTriggerEditor | undefined = $state()
	let postgresEditor: PostgresTriggerEditor | undefined = $state()
	let sqsEditor: SqsTriggerEditor | undefined = $state()
	let gcpEditor: GcpTriggerEditor | undefined = $state()
	let emailEditor: EmailTriggerEditor | undefined = $state()
	let scheduleEditor: ScheduleEditor | undefined = $state()
	let webhookEditor: WebhookEditor | undefined = $state()

	// Open the create-trigger drawer for `kind`, pre-filling `script_path`.
	// Caller is responsible for the draft guard (a trigger row can only point
	// at a deployed script).
	export function openNew(kind: NativeTriggerKind, scriptPath: string) {
		switch (kind) {
			case 'schedule':
				return scheduleEditor?.openNew(false, scriptPath, undefined, scriptPath)
			case 'kafka':
				return kafkaEditor?.openNew(false, scriptPath)
			case 'mqtt':
				return mqttEditor?.openNew(false, scriptPath)
			case 'nats':
				return natsEditor?.openNew(false, scriptPath)
			case 'postgres':
				return postgresEditor?.openNew(false, scriptPath)
			case 'sqs':
				return sqsEditor?.openNew(false, scriptPath)
			case 'gcp':
				return gcpEditor?.openNew(false, scriptPath)
			case 'email':
				return emailEditor?.openNew(false, scriptPath)
			// webhook has no dedicated editor.
			default:
				return
		}
	}

	export function openEdit(kind: NativeTriggerKind, triggerPath: string, scriptPath: string) {
		switch (kind) {
			case 'schedule':
				return scheduleEditor?.openEdit(triggerPath, false, scriptPath)
			case 'kafka':
				return kafkaEditor?.openEdit(triggerPath, false, scriptPath)
			case 'mqtt':
				return mqttEditor?.openEdit(triggerPath, false, scriptPath)
			case 'nats':
				return natsEditor?.openEdit(triggerPath, false, scriptPath)
			case 'postgres':
				return postgresEditor?.openEdit(triggerPath, false, scriptPath)
			case 'sqs':
				return sqsEditor?.openEdit(triggerPath, false, scriptPath)
			case 'gcp':
				return gcpEditor?.openEdit(triggerPath, false, scriptPath)
			case 'email':
				return emailEditor?.openEdit(triggerPath, false, scriptPath)
			default:
				return
		}
	}

	// Open the webhook drawer (endpoint URLs + token flow). Caller owns the
	// draft guard.
	export function openWebhook(scriptPath: string) {
		webhookEditor?.openDrawer(scriptPath, false)
	}

	// Delete-trigger confirmation state. Kebab → Delete opens the standard
	// ConfirmationModal; the actual delete is dispatched from onConfirmed so
	// the dialog stays consistent with the rest of the app.
	let triggerDeleteTarget = $state<{ kind: NativeTriggerKind; path: string } | undefined>(undefined)
	let triggerDeleteLoading = $state(false)
	let triggerDeleteOpen = $derived(triggerDeleteTarget != undefined)

	export function requestDelete(kind: NativeTriggerKind, triggerPath: string) {
		triggerDeleteTarget = { kind, path: triggerPath }
	}

	async function confirmDeleteAttachedTrigger() {
		const workspace = triggerWorkspace ?? $workspaceStore
		if (!triggerDeleteTarget || !workspace) return
		const { kind, path: triggerPath } = triggerDeleteTarget
		triggerDeleteLoading = true
		try {
			switch (kind) {
				case 'schedule':
					await ScheduleService.deleteSchedule({ workspace, path: triggerPath })
					break
				case 'kafka':
					await KafkaTriggerService.deleteKafkaTrigger({ workspace, path: triggerPath })
					break
				case 'mqtt':
					await MqttTriggerService.deleteMqttTrigger({ workspace, path: triggerPath })
					break
				case 'nats':
					await NatsTriggerService.deleteNatsTrigger({ workspace, path: triggerPath })
					break
				case 'postgres':
					await PostgresTriggerService.deletePostgresTrigger({ workspace, path: triggerPath })
					break
				case 'sqs':
					await SqsTriggerService.deleteSqsTrigger({ workspace, path: triggerPath })
					break
				case 'gcp':
					await GcpTriggerService.deleteGcpTrigger({ workspace, path: triggerPath })
					break
				case 'email':
					await EmailTriggerService.deleteEmailTrigger({ workspace, path: triggerPath })
					break
				default:
					return
			}
			sendUserToast(`Deleted ${kind} trigger "${triggerPath}"`)
			triggerDeleteTarget = undefined
			onUpdate()
		} catch (e: any) {
			sendUserToast(
				`Could not delete ${kind} trigger "${triggerPath}": ${e?.body ?? e?.message ?? String(e)}`,
				true
			)
		} finally {
			triggerDeleteLoading = false
		}
	}
</script>

{#if mountTriggerEditors}
	<ConfirmationModal
		open={triggerDeleteOpen}
		loading={triggerDeleteLoading}
		title={triggerDeleteTarget ? `Delete ${triggerDeleteTarget.kind} trigger` : 'Delete trigger'}
		confirmationText="Delete"
		onConfirmed={confirmDeleteAttachedTrigger}
		onCanceled={() => {
			if (!triggerDeleteLoading) triggerDeleteTarget = undefined
		}}
	>
		{#if triggerDeleteTarget}
			<p>
				Delete <code class="font-mono">{triggerDeleteTarget.path}</code>? The
				<code class="font-mono">// on {triggerDeleteTarget.kind}</code> annotation on the script stays
				— the trigger will read as missing on the canvas until you recreate it or remove the annotation.
			</p>
		{/if}
	</ConfirmationModal>

	<!-- Native trigger editors mounted off-screen. Each only renders its
	     inner drawer when `open=true` (set by openNew/openEdit), so this
	     adds ~zero render cost while idle. `onUpdate` refreshes the graph
	     so the new trigger row replaces the red missing placeholder.
	     Edit-mode only: every entry point (create/edit/delete trigger) is
	     gated off the canvas outside edit mode. -->
	<KafkaTriggerEditor bind:this={kafkaEditor} {onUpdate} />
	<MqttTriggerEditor bind:this={mqttEditor} {onUpdate} />
	<NatsTriggerEditor bind:this={natsEditor} {onUpdate} />
	<PostgresTriggerEditor bind:this={postgresEditor} {onUpdate} />
	<SqsTriggerEditor bind:this={sqsEditor} {onUpdate} />
	<GcpTriggerEditor bind:this={gcpEditor} {onUpdate} />
	<EmailTriggerEditor bind:this={emailEditor} {onUpdate} />
	<ScheduleEditor bind:this={scheduleEditor} {onUpdate} />
{/if}
<!-- Webhook drawer stays mounted in every mode — the webhook trigger node
     is clickable in view mode too (informational: endpoint URLs/token). -->
<WebhookEditor bind:this={webhookEditor} />
