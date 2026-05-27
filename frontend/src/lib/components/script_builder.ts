import type { NewScript, Script } from '$lib/gen'
import type { AssetWithAltAccessType } from './assets/lib'
import type { ScriptBuilderWhitelabelCustomUi } from './custom_ui'
import type { DiffDrawerI } from './diff_drawer'
import type { ScriptBuilderFunctionExports } from './scriptBuilder'
import type { ScheduleTrigger } from './triggers'
import type { Trigger } from './triggers/utils'
import type { WorkspaceItem } from './workspacePicker'

export interface ScriptBuilderProps {
	script: NewScript & {
		draft_triggers?: Trigger[]
		assets?: AssetWithAltAccessType[]
	}
	disableAi?: boolean
	fullyLoaded?: boolean
	initialPath?: string
	template?:
		| 'docker'
		| 'bunnative'
		| 'claudesandbox'
		| 'wac_python'
		| 'wac_typescript'
		| 'ci_test_bun'
		| 'ci_test_python'
		| 'script'
	initialArgs?: Record<string, any>
	lockedLanguage?: boolean
	showMeta?: boolean
	neverShowMeta?: boolean
	diffDrawer?: DiffDrawerI | undefined
	savedScript?: Script | NewScript | undefined
	searchParams?: URLSearchParams
	disableHistoryChange?: boolean
	customUi?: ScriptBuilderWhitelabelCustomUi
	savedPrimarySchedule?: ScheduleTrigger | undefined
	functionExports?: ((exports: ScriptBuilderFunctionExports) => void) | undefined
	children?: import('svelte').Snippet
	onDeploy?: (e: { path: string; hash: string }) => void
	onDeployError?: (e: { path: string; error: any }) => void
	onHistoryRestore?: () => void
	onSeeDetails?: (e: { path: string }) => void
	onNavigate?: (item: WorkspaceItem) => void
}
