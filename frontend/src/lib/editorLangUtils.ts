export function langToExt(lang: string): string {
    switch (lang) {
        case 'tsx':
            return 'tsx'
        case 'javascript':
            return 'js'
        case 'jsx':
            return 'js'
        case 'bunnative':
            return 'ts'
        case 'json':
            return 'json'
        case 'sql':
            return 'sql'
        case 'yaml':
            return 'yaml'
        case 'typescript':
            return 'ts'
        case 'python':
            return 'py'
        case 'go':
            return 'go'
        case 'bash':
            return 'sh'
        case 'powershell':
            return 'ps1'
        case 'php':
            return 'php'
        case 'rust':
            return 'rs'
        case 'deno':
            return 'ts'
        case 'nativets':
            return 'ts'
        case 'graphql':
            return 'gql'
        case 'css':
            return 'css'
        case 'ansible':
            return 'yml'
        case 'csharp':
            return 'cs'
        case 'nu':
            return 'nu'
        case 'java':
            return 'java'
        case 'svelte':
            return 'svelte'
        case 'vue':
            return 'vue'
        default:
            return 'unknown'
    }
}

export function extToLang(ext: string) {
    switch (ext) {
        case 'tsx':
            return 'typescript'
        case 'ts':
            return 'typescript'
        case 'js':
            return 'javascript'
        case 'jsx':
            return 'javascript'
        case 'json':
            return 'json'
        case 'sql':
            return 'sql'
        case 'yaml':
            return 'yaml'
        case 'py':
            return 'python'
        case 'go':
            return 'go'
        case 'sh':
            return 'bash'
        case 'ps1':
            return 'powershell'
        case 'php':
            return 'php'
        case 'rs':
            return 'rust'
        case 'gql':
            return 'graphql'
        case 'css':
            return 'css'
        case 'yml':
            return 'ansible'
        case 'cs':
            return 'csharp'
        case 'svelte':
            return 'svelte'
        case 'vue':
            return 'vue'
        case 'nu':
            return 'nu'
        case 'java':
            return 'java'
        case 'rb':
            return 'ruby'
        // for related places search: ADD_NEW_LANG
        default:
            return 'unknown'
    }
}

export function createHash() {
    return (Math.random() + 1).toString(36).substring(2)
}

export function createLongHash() {
    return 'h' + Math.random().toString(36).substring(2) + Math.random().toString(36).substring(2)
}
