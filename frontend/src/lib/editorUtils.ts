

export function editorConfig(model: any, code: string, lang: string, automaticLayout: boolean) {
    return {
        model,
        value: code,
        language: lang,
        automaticLayout,
        readOnly: false,
        fixedOverflowWidgets: true,
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
        }
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
        default:
            return 'unknown'
    }
}

export const updateOptions = { tabSize: 2, insertSpaces: true }