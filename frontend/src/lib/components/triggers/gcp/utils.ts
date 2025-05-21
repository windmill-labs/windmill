import { GcpTriggerService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveGcpTriggerFromCfg(
	initialPath: string,
	cfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	try {
		const requestBody = {
			gcp_resource_path: cfg.gcp_resource_path,
			subscription_mode: cfg.subscription_mode,
			subscription_id: cfg.subscription_id,
			delivery_type: cfg.delivery_type,
			delivery_config: cfg.delivery_config,
			base_endpoint: cfg.base_endpoint,
			topic_id: cfg.topic_id,
			path: cfg.path,
			script_path: cfg.script_path,
			enabled: cfg.enabled,
			is_flow: cfg.is_flow
		}
		if (edit) {
			await GcpTriggerService.updateGcpTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`GCP Pub/Sub trigger ${cfg.path} updated`)
		} else {
			await GcpTriggerService.createGcpTrigger({
				workspace: workspace,
				requestBody: {
					...requestBody,
					enabled: true
				}
			})
			sendUserToast(`GCP Pub/Sub trigger ${cfg.path} created`)
		}

		if (!get(usedTriggerKinds).includes('gcp')) {
			usedTriggerKinds.update((t) => [...t, 'gcp'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
