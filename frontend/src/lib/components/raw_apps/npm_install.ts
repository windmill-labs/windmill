export interface InstalledPackage {
    name: string
    version: string
    files: { path: string; content: string }[]
}
