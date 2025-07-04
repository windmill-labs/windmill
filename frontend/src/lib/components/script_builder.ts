export interface  ScriptBuilderProps {
    script: NewScript & { draft_triggers?: Trigger[] }
    fullyLoaded?: boolean
    initialPath?: string
    template?: 'docker' | 'bunnative' | 'script'
    initialArgs?: Record<string, any>
    lockedLanguage?: boolean
    showMeta?: boolean
    neverShowMeta?: boolean
    diffDrawer?: DiffDrawer | undefined
    savedScript?: NewScriptWithDraftAndDraftTriggers | undefined
    searchParams?: URLSearchParams
    disableHistoryChange?: boolean
    replaceStateFn?: (url: string) => void
    customUi?: ScriptBuilderWhitelabelCustomUi
    savedPrimarySchedule?: ScheduleTrigger | undefined
    functionExports?: ((exports: ScriptBuilderFunctionExports) => void) | undefined
    children?: import('svelte').Snippet
    onDeploy?: (e: { path: string; hash: string }) => void
    onDeployError?: (e: { path: string; error: any }) => void
    onSaveInitial?: (e: { path: string; hash: string }) => void
    onHistoryRestore?: () => void
    onSaveDraftOnlyAtNewPath?: (e: { path: string }) => void
    onSaveDraft?: (e: { path: string; savedAtNewPath: boolean; script: NewScript }) => void
    onSeeDetails?: (e: { path: string }) => void
    onSaveDraftError?: (e: { path: string; error: any }) => void
}