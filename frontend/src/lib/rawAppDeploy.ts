/**
 * Deploy a raw app (code-based app), from its server-side draft or from an
 * explicit value. Raw apps can't be deployed through the normal
 * AppService.updateApp/createApp path: their source `files` must be bundled to
 * js/css and saved via the raw-app endpoints.
 *
 * This mirrors how the global AI chat deploys raw apps
 * (`copilot/chat/global/core.ts` → deployDraft, case 'app'): read the item with
 * its draft, normalise to an AppDraftValue, recompute the policy, bundle the
 * files, then createAppRaw/updateAppRaw. The source→AppDraftValue projection is
 * shared via `rawAppDraftValue` so the two deploy paths can't drift.
 */
import { AppService } from '$lib/gen'
import type { Policy } from '$lib/gen'
import { bundleRawAppDraft } from '$lib/components/copilot/chat/global/rawAppBundlerBridge'
import type { AppDraftValue } from '$lib/components/copilot/chat/global/workspaceItems'
import { updateRawAppPolicy } from '$lib/components/raw_apps/rawAppPolicy'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import {
	appSourceToDraftValue,
	normalizeRawAppData
} from '$lib/components/raw_apps/rawAppDraftValue'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'

/**
 * Deploy an explicit raw-app value — one the user edited as JSON, or one
 * restored from a previous version — onto a deployed app.
 */
export async function deployRawAppValue({
	workspace,
	path,
	value,
	summary,
	policy: currentPolicy,
	deploymentMessage,
	allowKindChange
}: {
	workspace: string
	path: string
	value: any
	summary?: string
	policy?: Policy
	deploymentMessage?: string
	/** Let this deploy turn a low-code app into a raw one. Off for every ordinary
	 * deploy — see the backend's `allow_kind_change`. */
	allowKindChange?: boolean
}): Promise<void> {
	// The value often comes straight out of a `$state` field, and the bundler
	// runs in an iframe: postMessage refuses to clone a state proxy.
	const plainValue = (stateSnapshot(value) ?? {}) as Record<string, any>
	const files = (plainValue.files ?? {}) as Record<string, string>
	const runnables = plainValue.runnables ?? {}
	// The value carries the runnables, so the policy's triggerables have to be
	// recomputed from it or the deployed app can't call what it now contains.
	const policy = (await updateRawAppPolicy(runnables, currentPolicy)) as Policy
	// Who can open the app is set from the Share dialog: the deployed one stands.
	const row = await AppService.getAppLiteByPath({ workspace, path })
	policy.execution_mode = row.policy.execution_mode ?? 'publisher'

	const bundle = await bundleRawAppDraft({ workspace, files })

	await AppService.updateAppRaw({
		workspace,
		path,
		formData: {
			app: {
				// Through `normalizeRawAppData`, like every other deploy path: an old
				// version can still carry the pre-`data` datatable shapes.
				value: { files, runnables, data: normalizeRawAppData(plainValue) },
				summary: summary ?? '',
				policy,
				deployment_message: deploymentMessage,
				// Absent keeps the custom URL, which the Share dialog sets.
				custom_path: undefined,
				preserve_on_behalf_of: policy.on_behalf_of ? true : undefined,
				allow_kind_change: allowKindChange || undefined
			},
			js: bundle.js,
			css: bundle.css
		}
	})
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
		// Who can open the app and its custom URL are set from the Share dialog: the deployed
		// mode stands over the draft's, and an absent custom path keeps the deployed one.
		policy.execution_mode = app.policy?.execution_mode ?? policy.execution_mode
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
					custom_path: undefined,
					labels: value.labels,
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
					custom_path: value.custom_path ?? undefined,
					labels: value.labels,
					// Preserve the policy's on_behalf_of (see update branch above).
					preserve_on_behalf_of: policy.on_behalf_of ? true : undefined
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	}
}
