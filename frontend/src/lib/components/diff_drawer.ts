import type { Value } from "$lib/utils"

export type DiffDrawerDiff = 
| {
    mode: 'normal'
    deployed: Value
    draft: Value | undefined
    current: Value
    defaultDiffType?: 'deployed' | 'draft'
    button?: { text: string; onClick: () => void }
}
| {
    mode: 'simple'
    original: Value
    current: Value
    title: string
    button?: { text: string; onClick: () => void }
}

export interface DiffDrawerI {
    openDrawer: () => void
    closeDrawer: () => void
    setDiff: (diff: DiffDrawerDiff) => void
}