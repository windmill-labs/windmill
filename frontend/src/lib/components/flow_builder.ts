export type FlowBuilderProps = {
    initialPath?: string
    pathStoreInit?: string | undefined
    newFlow: boolean
    selectedId: string | undefined
    initialArgs?: Record<string, any>
    loading?: boolean
    flowStore: StateStore<OpenFlow>
    flowStateStore: Writable<FlowState>
    savedFlow?: FlowWithDraftAndDraftTriggers | undefined
    diffDrawer?: DiffDrawer | undefined
    customUi?: FlowBuilderWhitelabelCustomUi
    disableAi?: boolean
    disabledFlowInputs?: boolean
    savedPrimarySchedule?: ScheduleTrigger | undefined // used to set the primary schedule in the legacy primaryScheduleStore
    version?: number | undefined
    setSavedraftCb?: ((cb: () => void) => void) | undefined
    draftTriggersFromUrl?: Trigger[] | undefined
    selectedTriggerIndexFromUrl?: number | undefined
    children?: import('svelte').Snippet
    loadedFromHistoryFromUrl?: {
        flowJobInitial: boolean | undefined
        stepsState: Record<string, stepState>
    }
    noInitial?: boolean
    onSaveInitial?: ({ path, id }: { path: string; id: string }) => void
    onSaveDraft?: ({
        path,
        savedAtNewPath,
        newFlow
    }: {
        path: string
        savedAtNewPath: boolean
        newFlow: boolean
    }) => void
    onSaveDraftError?: ({ error }: { error: any }) => void
    onSaveDraftOnlyAtNewPath?: ({ path, selectedId }: { path: string; selectedId: string }) => void
    onDeploy?: ({ path }: { path: string }) => void
    onDeployError?: ({ error }: { error: any }) => void
    onDetails?: ({ path }: { path: string }) => void
    onHistoryRestore?: () => void
}
    