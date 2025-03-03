import type { Script } from "$lib/gen"

export type ScriptBuilderFunctionExports = {
    setPreviewArgs: (args: Record<string, any>) => void
    runPreview: () => Promise<string>
    setCode: (code: string, language?: Script["language"]) => void
    getCode: () => string
}