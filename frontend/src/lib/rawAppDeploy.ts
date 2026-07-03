/**
 * Deploy a raw app (code-based app) from its server-side draft. Raw apps can't
 * be deployed through the normal AppService.updateApp/createApp path: their
 * source `files` must be bundled to js/css and saved via the raw-app endpoints.
 *
 * This mirrors how the global AI chat deploys raw apps
 * (`copilot/chat/global/core.ts` → deployDraft, case 'app'): read the item with
 * its draft, normalise to an AppDraftValue, recompute the policy, bundle the
 * files, then createAppRaw/updateAppRaw. The source→AppDraftValue projection is
 * shared via `rawAppDraftValue` so the two deploy paths can't drift.
 */
import { get } from 'svelte/store'
import { AppService } from '$lib/gen'
import type { Policy } from '$lib/gen'
import { userStore } from '$lib/stores'
import { bundleRawAppDraft } from '$lib/components/copilot/chat/global/rawAppBundlerBridge'
import type { AppDraftValue } from '$lib/components/copilot/chat/global/workspaceItems'
import { updateRawAppPolicy } from '$lib/components/raw_apps/rawAppPolicy'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { appSourceToDraftValue } from '$lib/components/raw_apps/rawAppDraftValue'

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
	// `rawApp: true` so a never-deployed raw app (no `app` row) resolves to
	// the raw_app draft kind server-side instead of 404ing.
	const app = await AppService.getAppByPath({ workspace, path, getDraft: true, rawApp: true })
	const draft = (app as any).draft
	// Deploy at the draft's intended path. A raw-app draft carries the user-typed
	// path in `draft_path` (a never-deployed app is parked at a synthetic
	// `u/{user}/draft_{uuid}` storage key); the URL `path` below stays that storage
	// key. Falls back to `path` for an unrenamed draft on a deployed app.
	const targetPath = draft?.draft_path ?? draft?.path ?? path
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
					custom_path: isAdmin ? (value.custom_path ?? '') : undefined,
					// Preserve the policy's on_behalf_of: this draft-deploy path has no
					// on-behalf-of selector, so without the flag the backend resets it to
					// the deploying user (gated server-side by can_preserve_on_behalf_of).
					preserve_on_behalf_of: policy.on_behalf_of ? true : undefined
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
					custom_path: value.custom_path,
					// Preserve the policy's on_behalf_of (see update branch above).
					preserve_on_behalf_of: policy.on_behalf_of ? true : undefined
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	}
}
