/**
 * Deploy a raw app (code-based app) from its server-side draft. Raw apps can't
 * be deployed through the normal AppService.updateApp/createApp path: their
 * source `files` must be bundled to js/css and saved via the raw-app endpoints.
 *
 * This mirrors how the global AI chat deploys raw apps
 * (`copilot/chat/global/core.ts` → deployDraft, case 'app'): read the item with
 * its draft, normalise to an AppDraftValue, recompute the policy, bundle the
 * files, then createAppRaw/updateAppRaw. The two pure transforms
 * (appSourceToDraftValue / normalizeRawAppData) are re-implemented here to avoid
 * importing the heavy chat module.
 */
import { get } from 'svelte/store'
import { AppService } from '$lib/gen'
import type { Policy } from '$lib/gen'
import { userStore } from '$lib/stores'
import { bundleRawAppDraft } from '$lib/components/copilot/chat/global/rawAppBundlerBridge'
import type { AppDraftValue } from '$lib/components/copilot/chat/global/workspaceItems'
import { updateRawAppPolicy } from '$lib/components/raw_apps/rawAppPolicy'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'

function normalizeRawAppData(value: Record<string, any>): AppDraftValue['data'] {
	if (value.data?.creation) {
		return {
			tables: value.data.tables ?? [],
			datatable: value.data.creation.datatable,
			schema: value.data.creation.schema
		}
	}
	if (value.data) return value.data
	if (value.datatables) return { ...DEFAULT_RAW_APP_DATA, tables: value.datatables }
	if (value.dataTableRefs) return { ...DEFAULT_RAW_APP_DATA, tables: value.dataTableRefs }
	return { ...DEFAULT_RAW_APP_DATA }
}

function appSourceToDraftValue(app: any, fallback?: any): AppDraftValue {
	const value = (app.value ?? {}) as Record<string, any>
	return {
		summary: app.summary ?? '',
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: normalizeRawAppData(value),
		policy: app.policy ?? fallback?.policy,
		custom_path: app.custom_path ?? fallback?.custom_path
	}
}

/**
 * Promote a raw app's draft to deployed. Throws on failure (caller wraps into a
 * DeployResult). The matching draft row is deleted server-side by the raw-app
 * create/update handler, like the other deploy paths.
 */
export async function deployRawAppDraft(
	workspace: string,
	path: string,
	deploymentMessage?: string
): Promise<void> {
	const app = await AppService.getAppByPathWithDraft({ workspace, path })
	const draft = (app as any).draft
	// Honor a renamed draft path; the URL `path` below stays the existing item key.
	const targetPath = draft?.path ?? path
	const value = appSourceToDraftValue(draft ?? app, app)

	const policy = (await updateRawAppPolicy(
		value.runnables as any,
		value.policy as any
	)) as NonNullable<AppDraftValue['policy']> & Policy
	if (!policy.execution_mode) {
		policy.execution_mode = 'publisher'
	}

	const bundle = await bundleRawAppDraft({ workspace, files: value.files })

	const rawAppValue = {
		files: value.files,
		runnables: value.runnables,
		data: value.data ?? { ...DEFAULT_RAW_APP_DATA }
	}
	const summary = value.summary ?? ''

	if (await AppService.existsApp({ workspace, path })) {
		// custom_path changes require admin. Mirror RawAppEditorHeader's update path:
		// admins send the draft's value (`''` to clear), non-admins send undefined so
		// the backend ignores it and preserves the existing route — otherwise a
		// non-admin deploying a draft for an app that has a custom route would hit
		// RequireAdmin (the deployed custom_path is sent via the appSourceToDraftValue
		// fallback even when unchanged).
		const isAdmin = !!(get(userStore)?.is_admin || get(userStore)?.is_super_admin)
		await AppService.updateAppRaw({
			workspace,
			path,
			formData: {
				app: {
					path: targetPath,
					value: rawAppValue,
					summary,
					policy,
					deployment_message: deploymentMessage,
					custom_path: isAdmin ? (value.custom_path ?? '') : undefined
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	} else {
		await AppService.createAppRaw({
			workspace,
			formData: {
				app: {
					path: targetPath,
					value: rawAppValue,
					summary,
					policy,
					deployment_message: deploymentMessage,
					custom_path: value.custom_path
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	}
}
