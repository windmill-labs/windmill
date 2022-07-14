# Changelog


## [1.19.0](https://github.com/windmill-labs/windmill/compare/v1.18.0...v1.19.0) (2022-07-14)


### Features

* add DISABLE_NSJAIL mode ([1943585](https://github.com/windmill-labs/windmill/commit/19435851de0c18fc876a3bd00f3d9153f2719d9b))


### Bug Fixes

* add new ca-certificates folders for nsjail ([2eac1ef](https://github.com/windmill-labs/windmill/commit/2eac1ef363b209bb298dcbe7aafb7282ddd2b87a))
* **frontend:** add arbitrary scopes to connect an app ([372b14e](https://github.com/windmill-labs/windmill/commit/372b14e158bcb10bcfb07d231afeca5cc780661d))
* write job arguments to file ([#199](https://github.com/windmill-labs/windmill/issues/199)) ([9a6db75](https://github.com/windmill-labs/windmill/commit/9a6db758c15915f5f0027b1d270d621f91b7ae30))

## [1.18.0](https://github.com/windmill-labs/windmill/compare/v1.17.1...v1.18.0) (2022-07-13)


### Features

* account part II, handle refresh tokens, clarify oauth UI ([#196](https://github.com/windmill-labs/windmill/issues/196)) ([8403fbb](https://github.com/windmill-labs/windmill/commit/8403fbbc02076bb37dc82b2d26685957b13d036b))


### Bug Fixes

* **frontend:** fix path group refresh & create variable path reset ([6a341f5](https://github.com/windmill-labs/windmill/commit/6a341f5dc343df3df6491f8026e87632979faace))

## [1.17.1](https://github.com/windmill-labs/windmill/compare/v1.17.0...v1.17.1) (2022-07-08)


### Bug Fixes

* **backend:** set error content-type to text ([cf2dfd7](https://github.com/windmill-labs/windmill/commit/cf2dfd7fe74956d68bdc26dc47557ea6a0ed1ce4))
* **deno-client:** fix stringify ([5b89abe](https://github.com/windmill-labs/windmill/commit/5b89abe28283238a282da8920580a72f25e5a360))
* **frontend:** change lsp behavior ([d6e0817](https://github.com/windmill-labs/windmill/commit/d6e0817dc4fe54efd9346698c0ccb39057921d9b))
* **frontend:** connect an app resource creation ([e400dcc](https://github.com/windmill-labs/windmill/commit/e400dccedd88e3f5e3a9b0ec52fc9883d60c959b))
* **frontend:** connect an app resource creation ([68c5318](https://github.com/windmill-labs/windmill/commit/68c5318d16c85a01822570c113a4f33c539dc8bf))
* **frontend:** current hash link ([22eef8a](https://github.com/windmill-labs/windmill/commit/22eef8afab9143bb5b110db8c76e024604106051))
* **frontend:** fix sendRequest ([5da9819](https://github.com/windmill-labs/windmill/commit/5da9819ca5ce15ef4de9cf4a84affbd581383483))
* **frontend:** reload editor when language changes for in-flow editor ([72c7890](https://github.com/windmill-labs/windmill/commit/72c7890427736eeeb9a872bf0efd1acc906efd63))
* **frontend:** sveltekit prerender enabled -> default ([635873a](https://github.com/windmill-labs/windmill/commit/635873a96a586ad8e936526f4f4ebf679519e7fc))
* in-flow script editor fixes ([466f6b3](https://github.com/windmill-labs/windmill/commit/466f6b339acf70351814c32b8f31d80b8ff1c1b5))
* in-flow script editor fixes ([5853dfd](https://github.com/windmill-labs/windmill/commit/5853dfd85dca3c80b0edfb58b2866948af8011d5))
* remove unnecessary v8 snapshot ([d3904fd](https://github.com/windmill-labs/windmill/commit/d3904fd3ebde3a200ccc157a8532dfe1435ae16d))

## [1.17.0](https://github.com/windmill-labs/windmill/compare/v1.16.1...v1.17.0) (2022-07-05)


### Features

* in-flow editor mvp ([330b373](https://github.com/windmill-labs/windmill/commit/330b373c24f21b4d9a9b2903e8f1c60ee784ea89))

## [1.16.1](https://github.com/windmill-labs/windmill/compare/v1.16.0...v1.16.1) (2022-07-05)


### Bug Fixes

* bump all backend deps by breaking cycling through not using oauth2 ([e4a6378](https://github.com/windmill-labs/windmill/commit/e4a637860133e78cb1675173ccf3ff45e4b08c09))
* oauth logins used incorrect scope ([1dcba67](https://github.com/windmill-labs/windmill/commit/1dcba67a1f607faabcdfa6f7e94d280c66dd6470))
* trace errors body ([d092c62](https://github.com/windmill-labs/windmill/commit/d092c622c4efadb1e2799f7dbbe03f825f2b364d))


## [1.16.0](https://github.com/windmill-labs/windmill/compare/v1.15.1...v1.16.0) (2022-07-02)


### Features

* OAuth "Connect an App" ([#155](https://github.com/windmill-labs/windmill/issues/155)) ([3636866](https://github.com/windmill-labs/windmill/commit/3636866dda8b2e14d61c99a76f0a4e5fa6a37123))


### Bug Fixes

* add gitlab to connects ([d4e7c9e](https://github.com/windmill-labs/windmill/commit/d4e7c9e171cd02a7aa0846b43c127720260600b5))
* diverse frontend fixes

## [1.15.1](https://github.com/windmill-labs/windmill/compare/v1.15.0...v1.15.1) (2022-06-29)


### Bug Fixes

* databaseUrlFromResource uses proper database field ([6954580](https://github.com/windmill-labs/windmill/commit/69545808012fa4f5080ec58cf3dff2961a327117))

## [1.15.0](https://github.com/windmill-labs/windmill/compare/v1.14.6...v1.15.0) (2022-06-29)


### Features

* Flows Property picker component + Dynamic type inference ([#129](https://github.com/windmill-labs/windmill/issues/129)) ([44b4acf](https://github.com/windmill-labs/windmill/commit/44b4acf4bcfa0c372a9938a9b97d31cceedd9ad9))

## [1.14.6](https://github.com/windmill-labs/windmill/compare/v1.14.5...v1.14.6) (2022-06-27)


### Bug Fixes

* add databaseUrlFromResource to deno ([2659e9d](https://github.com/windmill-labs/windmill/commit/2659e9d62b88c2127c969becbc3a61ed2f118069))

## [1.14.5](https://github.com/windmill-labs/windmill/compare/v1.14.4...v1.14.5) (2022-06-27)


### Bug Fixes

* index.ts -> mod.ts ([d41913a](https://github.com/windmill-labs/windmill/commit/d41913a440b2034de59437488edc85e38c956d5f))
* insert getResource proper parenthesis ([e07b5d4](https://github.com/windmill-labs/windmill/commit/e07b5d4f30ea79a99caac4fb63a9ab1f17eaaf74))

## [1.14.4](https://github.com/windmill-labs/windmill/compare/v1.14.3...v1.14.4) (2022-06-27)


### Bug Fixes

* windmill deno package index.ts -> mod.ts ([8c0acac](https://github.com/windmill-labs/windmill/commit/8c0acac212d742acee8b7ff0cf6b93cce4187c19))

## [1.14.3](https://github.com/windmill-labs/windmill/compare/v1.14.2...v1.14.3) (2022-06-27)


### Bug Fixes

* internal state for script triggers v3 ([31445d7](https://github.com/windmill-labs/windmill/commit/31445d7182a910eab9d699760f2a86ca23d556a4))
* internal state for script triggers v3 ([22c6347](https://github.com/windmill-labs/windmill/commit/22c6347d8a74d94dc18109390ff5c347a2732823))
* internal state for script triggers v4 ([63a7401](https://github.com/windmill-labs/windmill/commit/63a7401f248cc37951bbea4dcaedaa6497d6f0b1))

## [1.14.2](https://github.com/windmill-labs/windmill/compare/v1.14.1...v1.14.2) (2022-06-27)


### Bug Fixes

* internal state for script triggers v2 ([f9eedc3](https://github.com/windmill-labs/windmill/commit/f9eedc31ed6e5d7e0a8a26633cca9965ac3b6a05))

## [1.14.1](https://github.com/windmill-labs/windmill/compare/v1.14.0...v1.14.1) (2022-06-27)


### Bug Fixes

* internal state for script triggers v1 ([6321311](https://github.com/windmill-labs/windmill/commit/6321311112dfa3ef09447f41847b248c0e0dcb46))

## [1.14.0](https://github.com/windmill-labs/windmill/compare/v1.13.0...v1.14.0) (2022-06-27)


### Features

* add tesseract bin to worker image ([6de9697](https://github.com/windmill-labs/windmill/commit/6de9697d955a06cfb9c64fdb501b4dfa1bb597ad))
* deno run with --unstable ([4947661](https://github.com/windmill-labs/windmill/commit/4947661b1d91867c022bb8a10a4be3e91f69352c))
* internal state for script triggers mvp ([dcdb989](https://github.com/windmill-labs/windmill/commit/dcdb989adb8350974289a0c8d2239b245a6e0d41))


### Bug Fixes

* change default per page to 100 ([fdf95a0](https://github.com/windmill-labs/windmill/commit/fdf95a065e83d733ab6a0f02edb4af16c0a1dfb9))
* deno exit after result logging ([6c622bc](https://github.com/windmill-labs/windmill/commit/6c622bcc32473361e1f7cb1ea7b0b508929bc1b8))
* improve error handling ([f98f642](https://github.com/windmill-labs/windmill/commit/f98f6429c1e646c0a836f2f73a03a803aa655583))
* improve error handling ([2efaf21](https://github.com/windmill-labs/windmill/commit/2efaf2191551c1406618c6d60bd37ca6eff84560))
* schemaPicker does not display editor by default ([fc0c38f](https://github.com/windmill-labs/windmill/commit/fc0c38ffad18a9ceda44cb8406736c14ba4eb4c2))
* smart assistant reload ([bb946ed](https://github.com/windmill-labs/windmill/commit/bb946ed5519f59adc559d6959c56e61403389c9d))

## [1.13.0](https://github.com/windmill-labs/windmill/compare/v1.12.0...v1.13.0) (2022-06-22)


### Features

* better type narrowing for list and array types ([276319d](https://github.com/windmill-labs/windmill/commit/276319d99240dbca5bcc74a1142d99ca823c4da2))


### Bug Fixes

* fix webhook path for flows ([906f740](https://github.com/windmill-labs/windmill/commit/906f740a0ddce26743e4669af7a101613131a17c))
* make email constraint case insensitive ([6dc90a3](https://github.com/windmill-labs/windmill/commit/6dc90a390643fcf6116289596ca1c3149d326797))

## [1.12.0](https://github.com/windmill-labs/windmill/compare/v1.11.0...v1.12.0) (2022-06-14)


### Bug Fixes

* more flexible ResourceType MainArgSignature parser ([359ef15](https://github.com/windmill-labs/windmill/commit/359ef15fa2a9024507a71f2c656373925fba3ebe))
* rename ResourceType -> Resource ([28b5671](https://github.com/windmill-labs/windmill/commit/28b56714023ea69a20f003e08f6c40de64202ac5))

## [1.11.0](https://github.com/windmill-labs/windmill/compare/v1.10.1...v1.11.0) (2022-06-13)


### Features

* add DISABLE_NUSER for older kernels ([cce46f9](https://github.com/windmill-labs/windmill/commit/cce46f94404ac5c10407e430fff8cdec3bd7fb2d))
* add ResourceType<'name'> as deno signature arg type ([f1ee5f3](https://github.com/windmill-labs/windmill/commit/f1ee5f3130cb7b753ccc3ee62169c5e4a8ef7b8b))


### Bug Fixes

* force c_ prefix for adding resource type ([9f235c4](https://github.com/windmill-labs/windmill/commit/9f235c404ed62b54a73451b9f9dbddd8f013120d))
* **frontend:** loadItems not called in script picker ([a59b927](https://github.com/windmill-labs/windmill/commit/a59b92706b24a07cc14288620a9bcdb9402bd134))

## [1.10.1](https://github.com/windmill-labs/windmill/compare/v1.10.0...v1.10.1) (2022-06-12)


### Bug Fixes

* python-client verify ssl ([295e28f](https://github.com/windmill-labs/windmill/commit/295e28fd43ef07b739d2c7c85b0ae6819f7d7434))

## [1.10.0](https://github.com/windmill-labs/windmill/compare/v1.9.0...v1.10.0) (2022-06-11)


### Features

* alpha hub integration + frontend user store fixes + script client base_url fix ([1a61d50](https://github.com/windmill-labs/windmill/commit/1a61d50076b295fe97e48c2a621dff30802152b1))

## [1.9.0](https://github.com/windmill-labs/windmill/compare/v1.8.6...v1.9.0) (2022-06-05)


### Features

* update postgres 13->14 in docker-compose ([479a12f](https://github.com/windmill-labs/windmill/commit/479a12f33ca26bfd1b67bcdd24a64ca26cc6bebe))


### Bug Fixes

* remove annoying transitions for scripts and flows ([f2348b5](https://github.com/windmill-labs/windmill/commit/f2348b5526bb8197519685cb57049f74c6f3a11d))

### [1.8.6](https://github.com/windmill-labs/windmill/compare/v1.8.5...v1.8.6) (2022-05-18)


### Bug Fixes

* re-release ([d31cd3c](https://github.com/windmill-labs/windmill/commit/d31cd3c52c1b46e821da261f22d0aec872b61fb2))

### [1.8.5](https://github.com/windmill-labs/windmill/compare/v1.8.4...v1.8.5) (2022-05-18)


### Bug Fixes

* language field broke flow too ([33fed8e](https://github.com/windmill-labs/windmill/commit/33fed8e04d3abbde371535ecb6e7ba15d103db92))

### [1.8.4](https://github.com/windmill-labs/windmill/compare/v1.8.3...v1.8.4) (2022-05-18)


### Bug Fixes

* scripts run was broken due to 1.7 and 1.8 changes. This fix it ([7564d2c](https://github.com/windmill-labs/windmill/commit/7564d2cb1e7f600ede22f333a02a537df381d829))

### [1.8.3](https://github.com/windmill-labs/windmill/compare/v1.8.2...v1.8.3) (2022-05-18)


### Bug Fixes

* clean exported deno-client api ([605c2b4](https://github.com/windmill-labs/windmill/commit/605c2b4d11bf072332a38f0c3e24cf6cc9ec7e65))

### [1.8.2](https://github.com/windmill-labs/windmill/compare/v1.8.1...v1.8.2) (2022-05-18)


### Bug Fixes

* deno client ([563ba3e](https://github.com/windmill-labs/windmill/commit/563ba3e7f763279a93f619933ac35a1dec3f727a))
* deno lsp client ([3eed59f](https://github.com/windmill-labs/windmill/commit/3eed59fcb1b172ab13f65c9a0caa0545f5ed91da))
* deno lsp uses wss instead of ws ([865d728](https://github.com/windmill-labs/windmill/commit/865d728224bed55fe4a2c1905ff2b8c15f4bbe17))
* starting deno script is now async ([7365a8e](https://github.com/windmill-labs/windmill/commit/7365a8e87bdb1f879eb92125a9e6378a1636637e))

### [1.8.1](https://github.com/windmill-labs/windmill/compare/v1.8.0...v1.8.1) (2022-05-17)


### Bug Fixes

* frontend dependencies update ([f793bc4](https://github.com/windmill-labs/windmill/commit/f793bc46d98349a5fea56c7911b6e0720b2b117c))

## [1.8.0](https://github.com/windmill-labs/windmill/compare/v1.7.0...v1.8.0) (2022-05-17)


### Features

* Typescript support for scripts (alpha) ([2e1d430](https://github.com/windmill-labs/windmill/commit/2e1d43033f3ad6dbe86338b7a41da7b1120a5ffc))

## [1.7.0](https://github.com/windmill-labs/windmill/compare/v1.6.1...v1.7.0) (2022-05-14)


### Features

* self host github oauth ([#46](https://github.com/windmill-labs/windmill/issues/46)) ([5b413d7](https://github.com/windmill-labs/windmill/commit/5b413d7e045d09dc5c5916cb22d82438ec6c92ad))


### Bug Fixes

* better error message when saving script ([02c8bea](https://github.com/windmill-labs/windmill/commit/02c8bea0840e492c31ccb8ddd1e5ae9676a534b1))

### [1.6.1](https://github.com/windmill-labs/windmill/compare/v1.6.0...v1.6.1) (2022-05-10)


### Bug Fixes

* also store and display "started at" for completed jobs ([#33](https://github.com/windmill-labs/windmill/issues/33)) ([2c28031](https://github.com/windmill-labs/windmill/commit/2c28031e44453740ad8c4b7e3c248173eab34b9c))

## 1.6.0 (2022-05-10)

### Features

* superadmin settings ([7a51f84](https://www.github.com/windmill-labs/windmill/commit/7a51f842f01e17c4d230c060fa0de558553ad3ed))
* user settings is now at workspace level ([a130806](https://www.github.com/windmill-labs/windmill/commit/a130806e1929267ee40ca443e3dac6e1a5d80da3))


### Bug Fixes

* display more than default 30 workspaces as superadmin ([55b5695](https://www.github.com/windmill-labs/windmill/commit/55b5695673912ffe040d3011c020b1002b4e3268))

## [1.5.0](https://www.github.com/windmill-labs/windmill/v1.5.0) (2022-05-02)
