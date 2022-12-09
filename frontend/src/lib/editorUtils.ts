

export function editorConfig(model: any, code: string, lang: string, automaticLayout: boolean, fixedOverflowWidgets: boolean) {
    return {
        model,
        value: code,
        language: lang,
        automaticLayout,
        readOnly: false,
        fixedOverflowWidgets,
        autoDetectHighContrast: true,
        //lineNumbers: 'off',
        //lineDecorationsWidth: 0,
        lineNumbersMinChars: 4,
        scrollbar: { alwaysConsumeMouseWheel: false },
        lineNumbers: (ln) => '<span class="pr-4 text-gray-400">' + ln + '</span>',
        folding: false,
        scrollBeyondLastLine: false,
        minimap: {
            enabled: false
        },
        lightbulb: {
            enabled: true
        },
        'bracketPairColorization.enabled': true,
        matchBrackets: 'always' as 'always',
    }
}

export function createHash() {
    return (Math.random() + 1).toString(36).substring(2)
}

export function langToExt(lang: string): string {
    switch (lang) {
        case 'javascript':
            return 'js'
        case 'json':
            return 'json'
        case 'sql':
            return 'sql'
        case 'typescript':
            return 'ts'
        case 'python':
            return 'py'
        case 'go':
            return 'go'
        case 'bash':
            return 'sh'
        default:
            return 'unknown'
    }
}

export const updateOptions = { tabSize: 2, insertSpaces: true }