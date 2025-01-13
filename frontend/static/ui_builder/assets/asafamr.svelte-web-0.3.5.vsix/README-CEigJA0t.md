# Svelte VSCode Web Extension

This is an unofficial port of the Svelte VSCode extension to VSCode web.
Web extensions can run in both standalone vscode and browser-based web vscode (used in e.g. github.dev, github1s.com, vscode.dev...).
[Web extensions have some limitations](https://code.visualstudio.com/api/extension-guides/web-extensions).

## Supported features

- All builtin language features should be working - syntax highlighting, definition navigation, auto completions...
- Typescript 4.4.4 bundled in with autocompletion across files
- Prettier formatter

## Unsupported features
Templating engines (pug, cofeescript...) and styling engines (sass, less...) aren't currently supported. 
Importing Svelte components from typescript isn't supported either.

<br>
<br>

See it in action in github.dev [here](https://github.dev/asafamr/svelte-web-preview) (Install the recommended extension when asked)

<br>
<br>

This is still a work in progress, feel free to create issues in the project [repo](https://github.com/asafamr/svelte-vscode-web).


repo readme:

![E2E Tests](https://github.com/asafamr/svelte-vscode-web/actions/workflows/tests.yml/badge.svg)

I started from the web extension starter template but switched to esbuild.

The official language tools repo is vendored ~~but currently unmodified~~ and has some small changes marked with a `WEBEXT` comment

Typescript support is preconfigured

Marketplace extension url: https://marketplace.visualstudio.com/items?itemName=asafamr.svelte-web

<br/>
<br/>
<br/>

bootstrap with 

```bash
npm install
npm run bootstrap
```

start esbuild watch with `npm run dev-watch`

then either:

run in vscode with `F5` (debug with electron dev tools - ctrl+shift+I) or

run in browser with `npm run run-in-browser` (debug with browser dev tools)


compile minified with `npm run compile` - this also writes some bundle size info to txt files


LICENSE: MIT