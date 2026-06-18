// Register Vue language
import { languages } from 'monaco-editor'

const wordPattern = new RegExp(
	'(-?\\d*\\.\\d\\w*)|([^\\`\\~\\!\\@\\#\\%\\^\\&\\*\\(\\)\\=\\+\\[\\{\\]\\}\\\\\\|\\;\\:\\"\\,\\.\\<\\>\\/\\?\\s]+)',
	'g'
)

// Regex for auto indentation
const indentationRules = {
	increaseIndentPattern: new RegExp('<(?!\\/(?:template|style|script))([\\w:\\-]+)[^>]*>[^<]*$'),
	decreaseIndentPattern: new RegExp('^<\\/[\\w:\\-]+>$')
}

// Regex for enter rules
const enterRules = [
	{
		beforeText: new RegExp(
			'<(?!(?:area|base|br|col|embed|hr|img|input|keygen|link|menuitem|meta|param|source|track|wbr))([_:\\w][_:\\w-.]*)([^/>]*(?!/.)>)[^<]*$',
			'i'
		),
		afterText: new RegExp('^<\\/([_:\\w][_:\\w-.]*)>\\s*$', 'i'),
		action: { indentAction: languages.IndentAction.IndentOutdent }
	},
	{
		beforeText: new RegExp(
			'<(?!(?:area|base|br|col|embed|hr|img|input|keygen|link|menuitem|meta|param|source|track|wbr))([_:\\w][_:\\w-.]*)([^/>]*(?!/.)>)[^<]*$',
			'i'
		),
		action: { indentAction: languages.IndentAction.Indent }
	}
]

export const conf = {
	wordPattern,
	indentationRules,
	onEnterRules: enterRules,
	// ... rest of the configuration remains the same
	autoClosingPairs: [
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '"', close: '"' },
		{ open: "'", close: "'" },
		{ open: '`', close: '`' },
		{ open: '<!--', close: '-->' },
		{ open: '<template', close: '</template>' },
		{ open: '<script', close: '</script>' },
		{ open: '<style', close: '</style>' }
	],
	surroundingPairs: [
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '"', close: '"' },
		{ open: "'", close: "'" },
		{ open: '`', close: '`' },
		{ open: '<', close: '>' }
	],
	comments: {
		lineComment: '//',
		blockComment: ['/*', '*/']
	},
	brackets: [
		['{', '}'],
		['[', ']'],
		['(', ')'],
		['<', '>']
	]
}

// Define Vue tokens and language rules
export const language = {
	defaultToken: '',
	tokenPostfix: '.vue',

	exponent: /[eE][\-+]?\d+/,
	// Regular expressions for different token types
	brackets: [
		{ open: '{', close: '}', token: 'delimiter.curly' },
		{ open: '[', close: ']', token: 'delimiter.square' },
		{ open: '(', close: ')', token: 'delimiter.parenthesis' },
		{ open: '<', close: '>', token: 'delimiter.angle' }
	],

	keywords: [
		'template',
		'script',
		'style',
		'props',
		'setup',
		'emit',
		'computed',
		'ref',
		'reactive',
		'watch',
		'watchEffect',
		'onMounted',
		'onUpdated',
		'onUnmounted',
		'defineProps',
		'defineEmits',
		'defineExpose',
		'withDefaults'
	],

	typeKeywords: ['boolean', 'number', 'string', 'object', 'array'],

	operators: [
		'=',
		'>',
		'<',
		'!',
		'~',
		'?',
		':',
		'==',
		'<=',
		'>=',
		'!=',
		'&&',
		'||',
		'++',
		'--',
		'+',
		'-',
		'*',
		'/',
		'&',
		'|',
		'^',
		'%',
		'<<',
		'>>',
		'>>>',
		'+=',
		'-=',
		'*=',
		'/=',
		'&=',
		'|=',
		'^=',
		'%=',
		'<<=',
		'>>=',
		'>>>='
	],

	// Symbols that can be used in identifiers
	symbols: /[=><!~?:&|+\-*\/\^%]+/,
	escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

	// The main tokenizer
	tokenizer: {
		root: [
			// Template block
			[/<template.*>/, { token: 'tag', next: '@template' }],
			// Script block
			[/<script.*>/, { token: 'tag', next: '@script' }],
			// Style block
			[/<style.*>/, { token: 'tag', next: '@style' }],
			// Other content
			{ include: '@whitespace' },
			{ include: '@tags' }
		],

		// Template section
		template: [
			[/<\/template>/, { token: 'tag', next: '@pop' }],
			[/{{/, { token: 'delimiter.bracket', next: '@templateExpression' }],
			[/<\/?[\w\-:.]+/, 'tag'],
			[/\s+/, ''],
			[/[^<{]+/, 'html']
		],

		// Script section
		script: [[/<\/script>/, { token: 'tag', next: '@pop' }], { include: '@javascript' }],

		// Style section
		style: [[/<\/style>/, { token: 'tag', next: '@pop' }], { include: '@css' }],

		// Template expressions (inside {{ }})
		templateExpression: [
			[/}}/, { token: 'delimiter.bracket', next: '@pop' }],
			[/[a-zA-Z_]\w*/, 'identifier'],
			{ include: '@javascript' }
		],

		// JavaScript rules
		javascript: [
			[/[{}]/, 'delimiter.bracket'],
			[/[\[\]]/, 'delimiter.square'],
			[/[()]/, 'delimiter.parenthesis'],
			[
				/[a-zA-Z_]\w*/,
				{
					cases: {
						'@keywords': 'keyword',
						'@typeKeywords': 'type',
						'@default': 'identifier'
					}
				}
			],
			[/[<>](?!@symbols)/, 'tag'],
			[
				/@symbols/,
				{
					cases: {
						'@operators': 'operator',
						'@default': ''
					}
				}
			],
			[/\d+\.\d*(@exponent)?/, 'number.float'],
			[/\.\d+(@exponent)?/, 'number.float'],
			[/\d+@exponent/, 'number.float'],
			[/\d+/, 'number'],
			[/[;,.]/, 'delimiter'],
			[/"([^"\\]|\\.)*$/, 'string.invalid'],
			[/'([^'\\]|\\.)*$/, 'string.invalid'],
			[/"/, 'string', '@string_double'],
			[/'/, 'string', '@string_single']
		],

		// CSS rules
		css: [
			[/[{}]/, 'delimiter.bracket'],
			[/[\[\]]/, 'delimiter.square'],
			[/[()]/, 'delimiter.parenthesis'],
			[/[-a-zA-Z_][\w-]*/, 'attribute.name'],
			[/(url\()([^)]]*)(\))/, ['tag', 'string', 'tag']],
			[/[@.][-a-zA-Z_][\w-]*/, 'tag'],
			[/[<>](?!@symbols)/, 'tag'],
			[/#[-a-zA-Z_][\w-]*/, 'tag'],
			[/\d+\.\d*(@exponent)?/, 'number.float'],
			[/\.\d+(@exponent)?/, 'number.float'],
			[/\d+@exponent/, 'number.float'],
			[/\d+/, 'number'],
			[/[;,.]/, 'delimiter']
		],

		whitespace: [
			[/[ \t\r\n]+/, 'white'],
			[/\/\*/, 'comment', '@comment'],
			[/\/\/.*$/, 'comment']
		],

		comment: [
			[/[^\/*]+/, 'comment'],
			[/\*\//, 'comment', '@pop'],
			[/[\/*]/, 'comment']
		],

		string_double: [
			[/[^\\"]+/, 'string'],
			[/@escapes/, 'string.escape'],
			[/\\./, 'string.escape.invalid'],
			[/"/, 'string', '@pop']
		],

		string_single: [
			[/[^\\']+/, 'string'],
			[/@escapes/, 'string.escape'],
			[/\\./, 'string.escape.invalid'],
			[/'/, 'string', '@pop']
		],

		tags: [[/<\/?[\w\-:.]+/, 'tag']]
	}
}
