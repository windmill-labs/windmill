import { languages } from 'monaco-editor/esm/vs/editor/editor.api'
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
        lineDecorationsWidth: 15,
        lineNumbersMinChars: 2,
        scrollbar: { alwaysConsumeMouseWheel: false },
        folding: false,
        scrollBeyondLastLine: false,
        minimap: {
            enabled: false
        },
        lightbulb: {
            enabled: true
        },
        suggest: {
            showKeywords: false,
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
        default:
            return 'unknown'
    }
}

export const updateOptions = { tabSize: 2, insertSpaces: true }

export function convertKind(kind: string): any {
    switch (kind) {
        case Kind.primitiveType:
        case Kind.keyword:
            return languages.CompletionItemKind.Keyword;
        case Kind.variable:
        case Kind.localVariable:
            return languages.CompletionItemKind.Variable;
        case Kind.memberVariable:
        case Kind.memberGetAccessor:
        case Kind.memberSetAccessor:
            return languages.CompletionItemKind.Field;
        case Kind.function:
        case Kind.memberFunction:
        case Kind.constructSignature:
        case Kind.callSignature:
        case Kind.indexSignature:
            return languages.CompletionItemKind.Function;
        case Kind.enum:
            return languages.CompletionItemKind.Enum;
        case Kind.module:
            return languages.CompletionItemKind.Module;
        case Kind.class:
            return languages.CompletionItemKind.Class;
        case Kind.interface:
            return languages.CompletionItemKind.Interface;
        case Kind.warning:
            return languages.CompletionItemKind.File;
    }

    return languages.CompletionItemKind.Property;
}

class Kind {
    public static unknown: string = '';
    public static keyword: string = 'keyword';
    public static script: string = 'script';
    public static module: string = 'module';
    public static class: string = 'class';
    public static interface: string = 'interface';
    public static type: string = 'type';
    public static enum: string = 'enum';
    public static variable: string = 'var';
    public static localVariable: string = 'local var';
    public static function: string = 'function';
    public static localFunction: string = 'local function';
    public static memberFunction: string = 'method';
    public static memberGetAccessor: string = 'getter';
    public static memberSetAccessor: string = 'setter';
    public static memberVariable: string = 'property';
    public static constructorImplementation: string = 'constructor';
    public static callSignature: string = 'call';
    public static indexSignature: string = 'index';
    public static constructSignature: string = 'construct';
    public static parameter: string = 'parameter';
    public static typeParameter: string = 'type parameter';
    public static primitiveType: string = 'primitive type';
    public static label: string = 'label';
    public static alias: string = 'alias';
    public static const: string = 'const';
    public static let: string = 'let';
    public static warning: string = 'warning';
}

export function createDocumentationString(details: any): string {
    let documentationString = displayPartsToString(details.documentation);
    if (details.tags) {
        for (const tag of details.tags) {
            documentationString += `\n\n${tagToString(tag)}`;
        }
    }
    return documentationString;
}


function tagToString(tag: any): string {
    let tagLabel = `*@${tag.name}*`;
    if (tag.name === 'param' && tag.text) {
        const [paramName, ...rest] = tag.text;
        tagLabel += `\`${paramName.text}\``;
        if (rest.length > 0) tagLabel += ` — ${rest.map((r) => r.text).join(' ')}`;
    } else if (Array.isArray(tag.text)) {
        tagLabel += ` — ${tag.text.map((r) => r.text).join(' ')}`;
    } else if (tag.text) {
        tagLabel += ` — ${tag.text}`;
    }
    return tagLabel;
}

export function displayPartsToString(displayParts: any | undefined): string {
    if (displayParts) {
        return displayParts.map((displayPart) => displayPart.text).join('');
    }
    return '';
}
