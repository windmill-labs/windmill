# @stoplight/ordered-object-literal

## Install

Do not use it if you can use maps.

```sh
yarn add @stoplight/ordered-object-literal
```

or if npm is package manager of your choice

```sh
npm install @stoplight/ordered-object-literal --save
```

## Usage

### I want to create a new object

```js
import box from '@stoplight/ordered-object-literal';

const trackedObj = box({});
```

### I have an existing object

```js
import box from '@stoplight/ordered-object-literal';

const myObj = { 
  a: true,
  b: void 0,
};

const trackedObj = box(myObj);
// alternatively if you want to provide a custom orer
const trackedReversedObj = box(myObj, ['b', 'a']);
```

## LICENSE

[Apache License 2.0](https://github.com/stoplightio/ordered-object-literal/blob/master/LICENSE)
