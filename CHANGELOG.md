# Changelog


## [1.27.2](https://github.com/windmill-labs/windmill/compare/v1.27.1...v1.27.2) (2022-08-02)


### Bug Fixes

* **deno-client:** getResource can now fetch non-object values ([b128388](https://github.com/windmill-labs/windmill/commit/b128388cc652d4cd369a88b93985a2c051003abd))

## [1.27.1](https://github.com/windmill-labs/windmill/compare/v1.27.0...v1.27.1) (2022-08-02)


### Bug Fixes

* migrate to new style radio button ([893ee94](https://github.com/windmill-labs/windmill/commit/893ee941d72a7036f0ea272c49bbe5cd3eee64d5))

## [1.27.0](https://github.com/windmill-labs/windmill/compare/v1.26.3...v1.27.0) (2022-08-02)


### Features

* add primitive sql format ([#320](https://github.com/windmill-labs/windmill/issues/320)) ([9daff2a](https://github.com/windmill-labs/windmill/commit/9daff2a228791234a3dd70c0ee829e284daf1592))


### Bug Fixes

* prefer `COPY` over `ADD` ([#319](https://github.com/windmill-labs/windmill/issues/319)) ([24a7e46](https://github.com/windmill-labs/windmill/commit/24a7e46fe99d5a1f7d5b22334fa5f6ce76e82d94))
* typos ([#301](https://github.com/windmill-labs/windmill/issues/301)) ([9e84b45](https://github.com/windmill-labs/windmill/commit/9e84b458b139e86eb51dba9c5b228f141ca649b3))

## [1.26.3](https://github.com/windmill-labs/windmill/compare/v1.26.2...v1.26.3) (2022-08-01)


### Bug Fixes

* displaying which group you are a member of that gave you access to item ([1bd0269](https://github.com/windmill-labs/windmill/commit/1bd026924b8a3b01f7729b627f939d8af872a483))
* refresh jobs result when hopping from flow to flow ([c86abe6](https://github.com/windmill-labs/windmill/commit/c86abe6ae01efd519f67ead233ebddf39f1539c0))

## [1.26.2](https://github.com/windmill-labs/windmill/compare/v1.26.1...v1.26.2) (2022-07-31)


### Bug Fixes

* deno api generator now supports openflow ([5b548a0](https://github.com/windmill-labs/windmill/commit/5b548a0e71669aad90343e70f3f1c9dc3a6d4baf))

## [1.26.1](https://github.com/windmill-labs/windmill/compare/v1.26.0...v1.26.1) (2022-07-31)


### Bug Fixes

* encoding state now supports unicode including emojis ([6b61227](https://github.com/windmill-labs/windmill/commit/6b61227481422fe52384f6de8146388a8471ff60))

## [1.26.0](https://github.com/windmill-labs/windmill/compare/v1.25.0...v1.26.0) (2022-07-29)


### Features

* resource type picker in schema modal + proper initialization of raw javascript editor when applicable ([01bb107](https://github.com/windmill-labs/windmill/commit/01bb107a0f3e3899ec99718974b2484ab5978c92))


### Bug Fixes

* forloop flows unsoundness fix part I ([1b5ce32](https://github.com/windmill-labs/windmill/commit/1b5ce3243b364d02903072a9af5e15447622e9fb))
* small bar mode and editor nits ([4e3a02a](https://github.com/windmill-labs/windmill/commit/4e3a02a8e44e25e6b5402f732b9af6969d06dcc0))

## [1.25.0](https://github.com/windmill-labs/windmill/compare/v1.24.2...v1.25.0) (2022-07-29)


### Features

* base64 support in schema editor ([2cb6e6e](https://github.com/windmill-labs/windmill/commit/2cb6e6e7021819a9aa9618436abf2f0fa5b3587b))


### Bug Fixes

* update variable and resources now return error if nothing was updated ([0faabdb](https://github.com/windmill-labs/windmill/commit/0faabdbc40b049258b074c6c20c1406ca14b8481))

## [1.24.2](https://github.com/windmill-labs/windmill/compare/v1.24.1...v1.24.2) (2022-07-28)


### Bug Fixes

* get_variable refresh_token bug ([390e9b3](https://github.com/windmill-labs/windmill/commit/390e9b37fb201242ac6983c145c9de5b242f7a7b))
* if :path is not a valid path, do not even attempt to fetch it ([6dec447](https://github.com/windmill-labs/windmill/commit/6dec4479537164fe17bea7f88fd60b1d4f42e887))
* monaco editor fixes ([f255cc2](https://github.com/windmill-labs/windmill/commit/f255cc253fcf14850442e8d4bf64635287b88314))

## [1.24.1](https://github.com/windmill-labs/windmill/compare/v1.24.0...v1.24.1) (2022-07-27)


### Bug Fixes

* encrypt the refresh token ([a051c21](https://github.com/windmill-labs/windmill/commit/a051c2121a63983f6925ce2e3a9b9deb01df2f04))
* keep previous refresh token if no new ones were provided ([3feef73](https://github.com/windmill-labs/windmill/commit/3feef738dc145603576649a91f0ddc0e82215841))
* skip_failures is boolean not bool ([4ca71c1](https://github.com/windmill-labs/windmill/commit/4ca71c1e5da0132724ab4c9771f5fdc590b866f8))

## [1.24.0](https://github.com/windmill-labs/windmill/compare/v1.23.0...v1.24.0) (2022-07-27)


### Features

* Add flow input and current step in the prop picker ([#236](https://github.com/windmill-labs/windmill/issues/236)) ([6fbeeae](https://github.com/windmill-labs/windmill/commit/6fbeeae84a207be46490361788dad12918c37c4e))
* add google login v1 ([fc918a2](https://github.com/windmill-labs/windmill/commit/fc918a24ccf0ad19b81a3ebf630d0f04b56094c8))
* add schedule settable from pull flows ([caecbfd](https://github.com/windmill-labs/windmill/commit/caecbfd0d9eaadc38372ce7238ed6d3baf9ba6e3))
* prop picker functional for pull flows ([010acfe](https://github.com/windmill-labs/windmill/commit/010acfe7e365a838078f1a989b54f1539c8bf2e6))
* skip failures loop ([#258](https://github.com/windmill-labs/windmill/issues/258)) ([de3fe69](https://github.com/windmill-labs/windmill/commit/de3fe699089e2a28aa0032a57a9a03f35646b6ef))


### Bug Fixes

* audit logs ([ca4bed3](https://github.com/windmill-labs/windmill/commit/ca4bed34a65440cd790cae9cff19f40df22f92b8))
* **frontend:** badge google logo for login ([cfec7a9](https://github.com/windmill-labs/windmill/commit/cfec7a97b883dbf83bd9d0707bf015c2aaa4e517))
* **frontend:** badge needs a little right margin ([c846ed7](https://github.com/windmill-labs/windmill/commit/c846ed76c4102335a5a8aabceaa39d6b7906ef5a))
* **frontend:** display number field in flows ([a232895](https://github.com/windmill-labs/windmill/commit/a23289563deca70269bd73ec50f324db0b6df791))
* **frontend:** fork script from hub ([43cacc1](https://github.com/windmill-labs/windmill/commit/43cacc1a66b1e2322c0252c9d1ca954e893aaef8))
* **frontend:** get refresh token for google services ([2f0d8d5](https://github.com/windmill-labs/windmill/commit/2f0d8d5384fb4eea6a6d5e5e48fd242f8d0c40fa))
* **frontend:** get refresh token for google services ([8dfe688](https://github.com/windmill-labs/windmill/commit/8dfe688a6a2388cecb1460913a25ab49ec297b1b))
* **frontend:** get refresh token for google services ([a2c5dc1](https://github.com/windmill-labs/windmill/commit/a2c5dc18a38045cbefc7d4b86d786a3c8fcb3ca8))
* import from JSON load schemas ([88dd7b0](https://github.com/windmill-labs/windmill/commit/88dd7b0abbd1a0469fc949c8045f61ddc304701d))
* multiple UI fixes ([a334029](https://github.com/windmill-labs/windmill/commit/a33402978720470530baecf51c2d17ecafd13ab0))
* multiple UI fixes ([904f0f3](https://github.com/windmill-labs/windmill/commit/904f0f3e69034421d524a66e0c4697ff42d89efe))

## [1.23.0](https://github.com/windmill-labs/windmill/compare/v1.22.0...v1.23.0) (2022-07-25)


### Features

* add editor bar to inline scripts of flows ([7a6a2c9](https://github.com/windmill-labs/windmill/commit/7a6a2c982daef9aa80e34aa6cbd4889a3c5ec807))
* **backend:** do not require visibility on job to see job if in possesion of uuid ([b054229](https://github.com/windmill-labs/windmill/commit/b05422963b27d74de8bb6d3be18538d57a71cfe7))
* **frontend:** deeper integration with the hub ([bb58eba](https://github.com/windmill-labs/windmill/commit/bb58eba2b521aef67b91cfc23f3ddcc8a001e18f))
* **frontend:** title everywhere ([38987c6](https://github.com/windmill-labs/windmill/commit/38987c6068c4cc2d9accbc368a67362e74adcabf))
* hub flows integration ([62777b7](https://github.com/windmill-labs/windmill/commit/62777b7a7888b3456f7f864cbb1acd887b172adc))


### Bug Fixes

* display websocket status in flow inline editor ([9e9138e](https://github.com/windmill-labs/windmill/commit/9e9138e4eeaea962dbb149ad4c1450572f025bc5))
* do not redirect to /user on /user namespace ([d95128e](https://github.com/windmill-labs/windmill/commit/d95128e68190fa6f75871f579de906ce82619524))
* **oauth2:** add google clients ([bc650b0](https://github.com/windmill-labs/windmill/commit/bc650b0ade1d378f815ee01da480a63ddd4501f1))
* static is undefined by default instead of being empty '' ([fc65162](https://github.com/windmill-labs/windmill/commit/fc651629c7977b5221dbb101f515766b23af9274))

## [1.22.0](https://github.com/windmill-labs/windmill/compare/v1.21.1...v1.22.0) (2022-07-22)


### Features

* add delete schedule ([f6d6934](https://github.com/windmill-labs/windmill/commit/f6d69345841f2ec0d06dc32b59840009982c55f2))
* **backend:** check of no path conflict between flow and flow's primary schedules ([c346339](https://github.com/windmill-labs/windmill/commit/c34633989e41e215d6183e5c887db68d4cc228d3))
* dynamic template for script inputs in flow ([3c16621](https://github.com/windmill-labs/windmill/commit/3c16621f6b9c2bee1f2630411bd70d075d247974))
* import and export flow from JSON ([7862ff4](https://github.com/windmill-labs/windmill/commit/7862ff41e25447d7b34aa261187bb98ed3f3105b))
* more visual cues about trigger scripts ([36606ab](https://github.com/windmill-labs/windmill/commit/36606ab8b675d01b0d38e2dd883b6e42b0987a6c))
* more visual cues about trigger scripts ([154c2a9](https://github.com/windmill-labs/windmill/commit/154c2a91ca6a4d60b02a44dda5fa23974594018b))
* rich rendering of flows ([38ffcfe](https://github.com/windmill-labs/windmill/commit/38ffcfeb292c6e9df0c89a4ef5364cdb8e23ccdd))


### Bug Fixes

* **deno-client:** make hack for patching openapi-generator more stable ([08ab4d1](https://github.com/windmill-labs/windmill/commit/08ab4d171a286d94e439a89d97115ad2db8e25d9))
* export json is converted to pull mode ([666e0f6](https://github.com/windmill-labs/windmill/commit/666e0f68d0dd84fce35e6fe1804c90a3c5125057))
* export json is converted to pull mode + rd fix ([c7528d4](https://github.com/windmill-labs/windmill/commit/c7528d417f276fbdb96751cda547feec7ac6fbc8))
* **frontend:** filter script by is_trigger and jobs by is_skipped + path fix ([97292d1](https://github.com/windmill-labs/windmill/commit/97292d18fb7158471f1be6ffbd45a612b09a689f))
* **frontend:** initFlow also reset schemaStore ([5941467](https://github.com/windmill-labs/windmill/commit/5941467ea19938b4d11b56c6f10f529c87cb52a3))
* **frontend:** remove unecessary step 1 of flows ([f429074](https://github.com/windmill-labs/windmill/commit/f429074528770f5eaebcf1ce687b6431321e169a))
* improve tooltip ([4be5d37](https://github.com/windmill-labs/windmill/commit/4be5d37a5441555c83eefbea17e86a5df4946749))
* improve tooltip ([c84b1c9](https://github.com/windmill-labs/windmill/commit/c84b1c9a8c6a03b9689e3405fa87f3c54016914a))
* placeholder undefined for arginput ([4d01598](https://github.com/windmill-labs/windmill/commit/4d01598e24fca673b0dc83860e151c21ab403b7a))

## [1.21.1](https://github.com/windmill-labs/windmill/compare/v1.21.0...v1.21.1) (2022-07-19)


### Bug Fixes

* **deno-client:** make hack for patching openapi-generator more stable ([2f4df43](https://github.com/windmill-labs/windmill/commit/2f4df43a1a798501449e82767d59f08e9cf95146))
* **python-client:** sed openapi to avoid generator circular dependency ([49f8050](https://github.com/windmill-labs/windmill/commit/49f8050aaf48c15fb79130a06ce754e285d17dd0))

## [1.21.0](https://github.com/windmill-labs/windmill/compare/v1.20.0...v1.21.0) (2022-07-19)


### Features

* add run_wait_result to mimic lambda ability ([6ef3754](https://github.com/windmill-labs/windmill/commit/6ef3754759346b8261934a35bd3bf3983872390f))


### Bug Fixes

* **backend:** clear env variables before running script ([98a5959](https://github.com/windmill-labs/windmill/commit/98a5959fcca19c54715e78055cf8881496209ac0))
* consistent exists/{resource} addition + usage in frontend ([ca66d33](https://github.com/windmill-labs/windmill/commit/ca66d33a4297d2f3a105829650a544f4a89c4615))
* **frontend:** validate username ([9828e54](https://github.com/windmill-labs/windmill/commit/9828e545e9649bc2ac6af598118ef85580fd80f3))
* list with is_skipped + deno-client fix ([6939f9d](https://github.com/windmill-labs/windmill/commit/6939f9d76b1579f2932e08df3f67dc293c642fd0))

## [1.20.0](https://github.com/windmill-labs/windmill/compare/v1.19.3...v1.20.0) (2022-07-17)


### Features

* trigger scripts and have flows being triggered by checking new external events regularly ([#200](https://github.com/windmill-labs/windmill/issues/200)) ([af23b30](https://github.com/windmill-labs/windmill/commit/af23b30c37b4225d6b927644f9612d4861e2d06c))


### Bug Fixes

* flow UI back and forth pull/push fix ([8918eb6](https://github.com/windmill-labs/windmill/commit/8918eb6fdb904e23b5dc340db669f6039ed7abb6))
* flow UI back and forth pull/push fix ([0973859](https://github.com/windmill-labs/windmill/commit/097385981323d5f88a51eb8df0e1114e8cf62727))
* **frontend:** chrome columns-2 fix for pull/push ([8272b11](https://github.com/windmill-labs/windmill/commit/8272b1110757ee0ed0cee4a7a6de537fcec83de3))
* **frontend:** createInlineScript only create trigger script if step = 0 ([bd004cf](https://github.com/windmill-labs/windmill/commit/bd004cff0f5150eb043f5446f5697bea43b1508b))
* HubPicker pick from trigger scripts when relevant ([7e846c3](https://github.com/windmill-labs/windmill/commit/7e846c32a63d9fe2f46f50f7642918cc34459829))

## [1.19.3](https://github.com/windmill-labs/windmill/compare/v1.19.2...v1.19.3) (2022-07-15)


### Bug Fixes

* **deno-client:** do not create resource for createInternalPath ([0967c1b](https://github.com/windmill-labs/windmill/commit/0967c1be65a9803e25f7701850be33121eb44d1b))

## [1.19.2](https://github.com/windmill-labs/windmill/compare/v1.19.1...v1.19.2) (2022-07-15)


### Bug Fixes

* **deno-client:** handle text/plain parse ([18e33bb](https://github.com/windmill-labs/windmill/commit/18e33bb40739fd699323f2da87de8c9696c0ef6c))

## [1.19.1](https://github.com/windmill-labs/windmill/compare/v1.19.0...v1.19.1) (2022-07-14)


### Bug Fixes

* **backend:** create resource would fail if is_oauth was not set ([cd621a6](https://github.com/windmill-labs/windmill/commit/cd621a6285d2aa0e554434998e931e96110464bd))
* **deno-client:** handle text/plain serialize ([98968ab](https://github.com/windmill-labs/windmill/commit/98968ab039fea89b7525fe7b852ba3d15dee831e))

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
