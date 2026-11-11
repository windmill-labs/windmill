import type { WorkspaceWebhookEventType } from '$lib/gen'

export const webhookEventPresets = [
	{
		label: 'Scripts',
		events: ['CreateScript', 'UpdateScript', 'DeleteScript', 'DeleteScriptPath']
	},
	{ label: 'Flows', events: ['CreateFlow', 'UpdateFlow', 'ArchiveFlow', 'DeleteFlow'] },
	{ label: 'Apps', events: ['CreateApp', 'UpdateApp', 'DeleteApp'] },
	{ label: 'Resources', events: ['CreateResource', 'UpdateResource', 'DeleteResource'] },
	{
		label: 'Resource types',
		events: ['CreateResourceType', 'UpdateResourceType', 'DeleteResourceType']
	},
	{ label: 'Variables', events: ['CreateVariable', 'UpdateVariable', 'DeleteVariable'] },
	{ label: 'Folders', events: ['CreateFolder', 'UpdateFolder', 'DeleteFolder'] },
	{ label: 'Tokens', events: ['TokenExpiringSoon', 'TokenExpired'] }
] as const satisfies readonly { label: string; events: readonly WorkspaceWebhookEventType[] }[]

type PresetEvent = (typeof webhookEventPresets)[number]['events'][number]

/** Fails to compile when an event type belongs to no preset: add it to one above. */
export const presetsCoverEveryEventType: [Exclude<WorkspaceWebhookEventType, PresetEvent>] extends [
	never
]
	? true
	: never = true
