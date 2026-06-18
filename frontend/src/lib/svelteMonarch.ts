// taken from node_modules/monaco-editor/dev/vs/basic-languages/html/html.js
// modifications:
// 1: replaced 'monaco_editor_core_1.' with '' (added import)
// 2: replaced 'exports.' with 'export const '
// 3: replaced 'text/javascript' with 'text/typescript'
// 4: added basic moustache keywords

import { languages } from 'monaco-editor'
var EMPTY_ELEMENTS = [
	'area',
	'base',
	'br',
	'col',
	'embed',
	'hr',
	'img',
	'input',
	'keygen',
	'link',
	'menuitem',
	'meta',
	'param',
	'source',
	'track',
	'wbr'
]
export const conf = {
	wordPattern: /(-?\d*\.\d\w*)|([^\`\~\!\@\$\^\&\*\(\)\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\s]+)/g,
	comments: {
		blockComment: ['<!--', '-->']
	},

	brackets: [
		['<!--', '-->'],
		['<', '>'],
		['{', '}'],
		['(', ')']
	],
	autoClosingPairs: [
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '"', close: '"' },
		{ open: "'", close: "'" }
	],
	surroundingPairs: [
		{ open: '"', close: '"' },
		{ open: "'", close: "'" },
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '<', close: '>' }
	],
	onEnterRules: [
		{
			beforeText: new RegExp(
				'<(?!(?:' + EMPTY_ELEMENTS.join('|') + '))([_:\\w][_:\\w-.\\d]*)([^/>]*(?!/)>)[^<]*$',
				'i'
			),
			afterText: /^<\/([_:\w][_:\w-.\d]*)\s*>$/i,
			action: {
				indentAction: languages.IndentAction.IndentOutdent
			}
		},
		{
			beforeText: new RegExp(
				'<(?!(?:' + EMPTY_ELEMENTS.join('|') + '))(\\w[\\w\\d]*)([^/>]*(?!/)>)[^<]*$',
				'i'
			),
			action: { indentAction: languages.IndentAction.Indent }
		}
	],
	folding: {
		markers: {
			start: new RegExp('^\\s*<!--\\s*#region\\b.*-->'),
			end: new RegExp('^\\s*<!--\\s*#endregion\\b.*-->')
		}
	}
}
export const language = {
	defaultToken: '',
	tokenPostfix: '.html',
	ignoreCase: true,

	moustkeys: [
		'#if',
		'/if',
		':else if',
		':else',
		'as',
		'#each',
		'/each',
		'#await',
		':then',
		':catch',
		'/await',
		'#key',
		'/key',
		'@html',
		'@debug'
	],

	// The main tokenizer for our languages
	tokenizer: {
		root: [
			[/<!DOCTYPE/, 'metatag', '@doctype'],
			[/<!--/, 'comment', '@comment'],
			[/{/, 'delimiter', '@curly'],
			[/(<)((?:[\w\-]+:)?[\w\-]+)(\s*)(\/>)/, ['delimiter', 'tag', '', 'delimiter']],
			[/(<)(script)/, ['delimiter', { token: 'tag', next: '@script' }]],
			[/(<)(style)/, ['delimiter', { token: 'tag', next: '@style' }]],
			[/(<)((?:[\w\-]+:)?[\w\-]+)/, ['delimiter', { token: 'tag', next: '@otherTag' }]],
			[/(<\/)((?:[\w\-]+:)?[\w\-]+)/, ['delimiter', { token: 'tag', next: '@otherTag' }]],
			[/</, 'delimiter'],
			[/[^<{]+/] // text
		],
		doctype: [
			[/[^>]+/, 'metatag.content'],
			[/>/, 'metatag', '@pop']
		],
		comment: [
			[/-->/, 'comment', '@pop'],
			[/[^-]+/, 'comment.content'],
			[/./, 'comment.content']
		],
		curly: [
			[/[ \t\r\n]+/],
			[/{/, 'delimiter', '@push'],
			[
				/[^}\s]+/,
				{
					cases: {
						'@moustkeys': 'keyword',
						'@default': 'tag'
					}
				}
			],
			[/}/, 'delimiter', '@pop']
		],
		otherTag: [
			[/\/?>/, 'delimiter', '@pop'],
			[/"([^"]*)"/, 'attribute.value'],
			[/'([^']*)'/, 'attribute.value'],
			[/[\w\-]+/, 'attribute.name'],
			[/=/, 'delimiter'],
			[/[ \t\r\n]+/] // whitespace
		],
		// -- BEGIN <script> tags handling
		// After <script
		script: [
			[/type/, 'attribute.name', '@scriptAfterType'],
			[/"([^"]*)"/, 'attribute.value'],
			[/'([^']*)'/, 'attribute.value'],
			[/[\w\-]+/, 'attribute.name'],
			[/=/, 'delimiter'],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@scriptEmbedded',
					nextEmbedded: 'text/typescript'
				}
			],
			[/[ \t\r\n]+/],
			[/(<\/)(script\s*)(>)/, ['delimiter', 'tag', { token: 'delimiter', next: '@pop' }]]
		],
		// After <script ... type
		scriptAfterType: [
			[/=/, 'delimiter', '@scriptAfterTypeEquals'],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@scriptEmbedded',
					nextEmbedded: 'text/typescript'
				}
			],
			[/[ \t\r\n]+/],
			[/<\/script\s*>/, { token: '@rematch', next: '@pop' }]
		],
		// After <script ... type =
		scriptAfterTypeEquals: [
			[
				/"([^"]*)"/,
				{
					token: 'attribute.value',
					switchTo: '@scriptWithCustomType.$1'
				}
			],
			[
				/'([^']*)'/,
				{
					token: 'attribute.value',
					switchTo: '@scriptWithCustomType.$1'
				}
			],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@scriptEmbedded',
					nextEmbedded: 'text/typescript'
				}
			],
			[/[ \t\r\n]+/],
			[/<\/script\s*>/, { token: '@rematch', next: '@pop' }]
		],
		// After <script ... type = $S2
		scriptWithCustomType: [
			[
				/>/,
				{
					token: 'delimiter',
					next: '@scriptEmbedded.$S2',
					nextEmbedded: '$S2'
				}
			],
			[/"([^"]*)"/, 'attribute.value'],
			[/'([^']*)'/, 'attribute.value'],
			[/[\w\-]+/, 'attribute.name'],
			[/=/, 'delimiter'],
			[/[ \t\r\n]+/],
			[/<\/script\s*>/, { token: '@rematch', next: '@pop' }]
		],
		scriptEmbedded: [
			[/<\/script/, { token: '@rematch', next: '@pop', nextEmbedded: '@pop' }],
			[/[^<]+/, '']
		],
		// -- END <script> tags handling
		// -- BEGIN <style> tags handling
		// After <style
		style: [
			[/type/, 'attribute.name', '@styleAfterType'],
			[/"([^"]*)"/, 'attribute.value'],
			[/'([^']*)'/, 'attribute.value'],
			[/[\w\-]+/, 'attribute.name'],
			[/=/, 'delimiter'],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@styleEmbedded',
					nextEmbedded: 'text/css'
				}
			],
			[/[ \t\r\n]+/],
			[/(<\/)(style\s*)(>)/, ['delimiter', 'tag', { token: 'delimiter', next: '@pop' }]]
		],
		// After <style ... type
		styleAfterType: [
			[/=/, 'delimiter', '@styleAfterTypeEquals'],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@styleEmbedded',
					nextEmbedded: 'text/css'
				}
			],
			[/[ \t\r\n]+/],
			[/<\/style\s*>/, { token: '@rematch', next: '@pop' }]
		],
		// After <style ... type =
		styleAfterTypeEquals: [
			[
				/"([^"]*)"/,
				{
					token: 'attribute.value',
					switchTo: '@styleWithCustomType.$1'
				}
			],
			[
				/'([^']*)'/,
				{
					token: 'attribute.value',
					switchTo: '@styleWithCustomType.$1'
				}
			],
			[
				/>/,
				{
					token: 'delimiter',
					next: '@styleEmbedded',
					nextEmbedded: 'text/css'
				}
			],
			[/[ \t\r\n]+/],
			[/<\/style\s*>/, { token: '@rematch', next: '@pop' }]
		],
		// After <style ... type = $S2
		styleWithCustomType: [
			[
				/>/,
				{
					token: 'delimiter',
					next: '@styleEmbedded.$S2',
					nextEmbedded: '$S2'
				}
			],
			[/"([^"]*)"/, 'attribute.value'],
			[/'([^']*)'/, 'attribute.value'],
			[/[\w\-]+/, 'attribute.name'],
			[/=/, 'delimiter'],
			[/[ \t\r\n]+/],
			[/<\/style\s*>/, { token: '@rematch', next: '@pop' }]
		],
		styleEmbedded: [
			[/<\/style/, { token: '@rematch', next: '@pop', nextEmbedded: '@pop' }],
			[/[^<]+/, '']
		]
		// -- END <style> tags handling
	}
}
