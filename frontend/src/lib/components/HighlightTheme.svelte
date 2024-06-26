<script lang="ts">
</script>

<style global lang="postcss">
	pre code.hljs {
		display: block;
		overflow-x: auto;
		padding: 1em;
		overflow: visible !important;
	}

	code.hljs {
		padding: 3px 5px;
	}

	.dark .hljs {
		background: #2e3440;
	}
	.dark .hljs,
	.dark .hljs-subst {
		color: #d8dee9;
	}
	.dark .hljs-selector-tag {
		color: #81a1c1;
	}
	.dark .hljs-selector-id {
		color: #8fbcbb;
		font-weight: bold;
	}
	.dark .hljs-selector-class {
		color: #8fbcbb;
	}
	.dark .hljs-selector-attr {
		color: #8fbcbb;
	}
	.dark .hljs-property {
		color: #88c0d0;
	}
	.dark .hljs-selector-pseudo {
		color: #88c0d0;
	}
	.dark .hljs-addition {
		background-color: rgba(163, 190, 140, 0.5);
	}
	.dark .hljs-deletion {
		background-color: rgba(191, 97, 106, 0.5);
	}
	.dark .hljs-built_in,
	.dark .hljs-type {
		color: #8fbcbb;
	}
	.dark .hljs-class {
		color: #8fbcbb;
	}
	.dark .hljs-function {
		color: #88c0d0;
	}
	.dark .hljs-title.hljs-function,
	.dark .hljs-function > .hljs-title {
		color: #88c0d0;
	}
	.dark .hljs-keyword,
	.dark .hljs-literal,
	.dark .hljs-symbol {
		color: #81a1c1;
	}
	.dark .hljs-number {
		color: #b48ead;
	}
	.dark .hljs-regexp {
		color: #ebcb8b;
	}
	.dark .hljs-string {
		color: #a3be8c;
	}
	.dark .hljs-title {
		color: #8fbcbb;
	}
	.dark .hljs-params {
		color: #d8dee9;
	}
	.dark .hljs-bullet {
		color: #81a1c1;
	}
	.dark .hljs-code {
		color: #8fbcbb;
	}
	.dark .hljs-emphasis {
		font-style: italic;
	}
	.dark .hljs-formula {
		color: #8fbcbb;
	}
	.dark .hljs-strong {
		font-weight: bold;
	}
	.dark .hljs-link:hover {
		-webkit-text-decoration: underline;
		text-decoration: underline;
	}
	.dark .hljs-quote {
		color: #4c566a;
	}
	.dark .hljs-comment {
		color: #4c566a;
	}
	.dark .hljs-doctag {
		color: #8fbcbb;
	}
	.dark .hljs-meta,
	.dark .hljs-meta .hljs-keyword {
		color: #5e81ac;
	}
	.dark .hljs-meta .hljs-string {
		color: #a3be8c;
	}
	.dark .hljs-attr {
		color: #8fbcbb;
	}
	.dark .hljs-attribute {
		color: #d8dee9;
	}
	.dark .hljs-name {
		color: #81a1c1;
	}
	.dark .hljs-section {
		color: #88c0d0;
	}
	.dark .hljs-tag {
		color: #81a1c1;
	}
	.dark .hljs-variable {
		color: #d8dee9;
	}
	.dark .hljs-template-variable {
		color: #d8dee9;
	}
	.dark .hljs-template-tag {
		color: #5e81ac;
	}
	.dark .language-abnf .hljs-attribute {
		color: #88c0d0;
	}
	.dark .language-abnf .hljs-symbol {
		color: #ebcb8b;
	}
	.dark .language-apache .hljs-attribute {
		color: #88c0d0;
	}
	.dark .language-apache .hljs-section {
		color: #81a1c1;
	}
	.dark .language-arduino .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-aspectj .hljs-meta {
		color: #d08770;
	}
	.dark .language-aspectj > .hljs-title {
		color: #88c0d0;
	}
	.dark .language-bnf .hljs-attribute {
		color: #8fbcbb;
	}
	.dark .language-clojure .hljs-name {
		color: #88c0d0;
	}
	.dark .language-clojure .hljs-symbol {
		color: #ebcb8b;
	}
	.dark .language-coq .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-cpp .hljs-meta .hljs-string {
		color: #8fbcbb;
	}
	.dark .language-css .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-css .hljs-keyword {
		color: #d08770;
	}
	.dark .language-diff .hljs-meta {
		color: #8fbcbb;
	}
	.dark .language-ebnf .hljs-attribute {
		color: #8fbcbb;
	}
	.dark .language-glsl .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-groovy .hljs-meta:not(:first-child) {
		color: #d08770;
	}
	.dark .language-haxe .hljs-meta {
		color: #d08770;
	}
	.dark .language-java .hljs-meta {
		color: #d08770;
	}
	.dark .language-ldif .hljs-attribute {
		color: #8fbcbb;
	}
	.dark .language-lisp .hljs-name {
		color: #88c0d0;
	}
	.dark .language-lua .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-moonscript .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-nginx .hljs-attribute {
		color: #88c0d0;
	}
	.dark .language-nginx .hljs-section {
		color: #5e81ac;
	}
	.dark .language-pf .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-processing .hljs-built_in {
		color: #88c0d0;
	}
	.dark .language-scss .hljs-keyword {
		color: #81a1c1;
	}
	.dark .language-stylus .hljs-keyword {
		color: #81a1c1;
	}
	.dark .language-swift .hljs-meta {
		color: #d08770;
	}
	.dark .language-vim .hljs-built_in {
		color: #88c0d0;
		font-style: italic;
	}
	.dark .language-yaml .hljs-meta {
		color: #d08770;
	}

	.hljs {
		color: #24292e;
		background: #ffffff;
	}
	.hljs-doctag,
	.hljs-keyword,
	.hljs-meta .hljs-keyword,
	.hljs-template-tag,
	.hljs-template-variable,
	.hljs-type,
	.hljs-variable.language_ {
		color: #d73a49;
	}
	.hljs-title,
	.hljs-title.class_,
	.hljs-title.class_.inherited__,
	.hljs-title.function_ {
		color: #6f42c1;
	}
	.hljs-attr,
	.hljs-attribute,
	.hljs-literal,
	.hljs-meta,
	.hljs-number,
	.hljs-operator,
	.hljs-variable,
	.hljs-selector-attr,
	.hljs-selector-class,
	.hljs-selector-id {
		color: #005cc5;
	}
	.hljs-regexp,
	.hljs-string,
	.hljs-meta .hljs-string {
		color: #032f62;
	}
	.hljs-built_in,
	.hljs-symbol {
		color: #e36209;
	}
	.hljs-comment,
	.hljs-code,
	.hljs-formula {
		color: #6a737d;
	}
	.hljs-name,
	.hljs-quote,
	.hljs-selector-tag,
	.hljs-selector-pseudo {
		color: #22863a;
	}
	.hljs-subst {
		color: #24292e;
	}
	.hljs-section {
		color: #005cc5;
		font-weight: bold;
	}
	.hljs-bullet {
		color: #735c0f;
	}
	.hljs-emphasis {
		color: #24292e;
		font-style: italic;
	}
	.hljs-strong {
		color: #24292e;
		font-weight: bold;
	}
	.hljs-addition {
		color: #22863a;
		background-color: #f0fff4;
	}
	.hljs-deletion {
		color: #b31d28;
		background-color: #ffeef0;
	}
</style>
