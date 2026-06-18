import type { Value } from "$lib/utils"

export type GetInitialAndModifiedValues = (() => SavedAndModifiedValue) | undefined

export type SavedAndModifiedValue = {
    savedValue: Value | undefined
    modifiedValue: Value | undefined
}