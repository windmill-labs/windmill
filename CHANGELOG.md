# Changelog


## [1.270.0](https://github.com/windmill-labs/windmill/compare/v1.269.0...v1.270.0) (2024-02-15)


### Features

* add raw option for urlencoded webhook call ([#3215](https://github.com/windmill-labs/windmill/issues/3215)) ([b81b095](https://github.com/windmill-labs/windmill/commit/b81b095b61f009fdf479b05cae113804c3975e1a))
* Git sync exclude certain type per repository ([#3210](https://github.com/windmill-labs/windmill/issues/3210)) ([86326c1](https://github.com/windmill-labs/windmill/commit/86326c16524586f2737f1c5e3e98d0d5a2f1df96))
* set branch summary on predicate gen ([#3212](https://github.com/windmill-labs/windmill/issues/3212)) ([156d10d](https://github.com/windmill-labs/windmill/commit/156d10d4162031bca50d413bc4e8a2bf24711420))
* Workspace encryption key can be manually updated ([#3223](https://github.com/windmill-labs/windmill/issues/3223)) ([e8ed478](https://github.com/windmill-labs/windmill/commit/e8ed4783b2ff692951ec16be35d6b5553728b86f))


### Bug Fixes

* add back resource type btn for bun/fetch ([#3217](https://github.com/windmill-labs/windmill/issues/3217)) ([d3a74e8](https://github.com/windmill-labs/windmill/commit/d3a74e881fdb7ac999508229f1f6677609d7e831))
* add timestamp array support in pg ([#3229](https://github.com/windmill-labs/windmill/issues/3229)) ([7f98a96](https://github.com/windmill-labs/windmill/commit/7f98a96e24103d92011f6896b0b332034b4e78f1))
* **frontend:** add header when downloading a CSV ([#3228](https://github.com/windmill-labs/windmill/issues/3228)) ([f399f49](https://github.com/windmill-labs/windmill/commit/f399f4921edb7735369e9d8abba95eefb53cfb4b))
* **frontend:** fix supabase connect ([#3218](https://github.com/windmill-labs/windmill/issues/3218)) ([69da45d](https://github.com/windmill-labs/windmill/commit/69da45ddf2c8969577eede4188d988ea88d16cbc))
* **frontend:** Fix table action recompute ([#3221](https://github.com/windmill-labs/windmill/issues/3221)) ([455aaed](https://github.com/windmill-labs/windmill/commit/455aaedd929d2bd4b637889fadc0411d7eeba5de))
* **frontend:** update a few svelte packages ([#3222](https://github.com/windmill-labs/windmill/issues/3222)) ([28192ec](https://github.com/windmill-labs/windmill/commit/28192ec014fe1c1bb93ea563a4976fae1bff9fec))
* improve transformer script gen ([#3211](https://github.com/windmill-labs/windmill/issues/3211)) ([d352b68](https://github.com/windmill-labs/windmill/commit/d352b68a26f3c58caccb67cc15ac5885496f80b4))
* oauth settings for github and gitlab ([#3219](https://github.com/windmill-labs/windmill/issues/3219)) ([d1f929b](https://github.com/windmill-labs/windmill/commit/d1f929b2bb73e4727089df2936dde723486ba458))
* prevent AI fill inputs freezing ([#3226](https://github.com/windmill-labs/windmill/issues/3226)) ([4492279](https://github.com/windmill-labs/windmill/commit/4492279c507d7ab8b705defba400dc53c853cd88))
* run recompute on success for toggles on click and set default value ([b75f79d](https://github.com/windmill-labs/windmill/commit/b75f79d3274de0c7620800afacedf9b5f980fda0))
* **sso:** improve handling of filters for get groups for sso ([44bfbad](https://github.com/windmill-labs/windmill/commit/44bfbadf689e11e9285de9503f738971fc9c34a6))

## [1.269.0](https://github.com/windmill-labs/windmill/compare/v1.268.0...v1.269.0) (2024-02-13)


### Features

* add ai for predicates and iterator expressions ([#3203](https://github.com/windmill-labs/windmill/issues/3203)) ([43e0ceb](https://github.com/windmill-labs/windmill/commit/43e0ceb342cd27ee94e7c1ac830ee642294e2f2f))
* **frontend:** add confirmation modal to the app button ([#3199](https://github.com/windmill-labs/windmill/issues/3199)) ([6200932](https://github.com/windmill-labs/windmill/commit/620093271a44e90ea9b081583b4f1d93b04794dd))
* Git sync can handle resource types, resources, variables and schedules ([#3202](https://github.com/windmill-labs/windmill/issues/3202)) ([0e7de63](https://github.com/windmill-labs/windmill/commit/0e7de63c4b698a449464a177b8da37ab9d0315ff))
* improve runs page + add all workspaces to admins runs page ([90c7c0e](https://github.com/windmill-labs/windmill/commit/90c7c0ed8a0b268c4fae7254e25ef29ee7e2aef6))


### Bug Fixes

* **cli:** update hub sync version ([8b46b95](https://github.com/windmill-labs/windmill/commit/8b46b953a2d0ac48bab51cadfc11a2d299242c4a))
* **frontend:** Truncate path on the run page ([#3208](https://github.com/windmill-labs/windmill/issues/3208)) ([3d0e5c8](https://github.com/windmill-labs/windmill/commit/3d0e5c8e576ab907a3533a30e3371641ddaea5f2))
* init scripts are tagged with 'init_script' ([3c52ef1](https://github.com/windmill-labs/windmill/commit/3c52ef14691b0db3774ed7dd376ee23687e9462f))
* **scim:** switch right join to left join for groups ([9655b8f](https://github.com/windmill-labs/windmill/commit/9655b8fb3275bc332a3bd2a10be27a30b3d653a5))
* **scim:** when deleting instance groups manually, delete also members mapping ([ce4d077](https://github.com/windmill-labs/windmill/commit/ce4d0777f6d9297af62b92d29855d7108aac23d2))
* search on table set page to 0 ([5c5c9c5](https://github.com/windmill-labs/windmill/commit/5c5c9c5ac4aa166054666b64d00cc82a2afac8f5))

## [1.268.0](https://github.com/windmill-labs/windmill/compare/v1.267.0...v1.268.0) (2024-02-11)


### Features

* flow inputs ai gen ([#3191](https://github.com/windmill-labs/windmill/issues/3191)) ([13e6706](https://github.com/windmill-labs/windmill/commit/13e6706a095639c92691a4e714704aedd09155c2))


### Bug Fixes

* allow direct git-sync setting loading ([f171d08](https://github.com/windmill-labs/windmill/commit/f171d0827858fbbf1b2a478164ae67bce404e7d1))
* **cli:** improve tty handling ([87ee3e4](https://github.com/windmill-labs/windmill/commit/87ee3e4a93f955f8ee5dab04e8bcec5415fb3fb5))
* **cli:** improve tty handling ([f1ae3f2](https://github.com/windmill-labs/windmill/commit/f1ae3f21ef97b5144cc670b1c5f12508d3ac6c9d))

## [1.267.0](https://github.com/windmill-labs/windmill/compare/v1.266.1...v1.267.0) (2024-02-10)


### Features

* default tag can be made workspace specific ([#3194](https://github.com/windmill-labs/windmill/issues/3194)) ([8a3a9bd](https://github.com/windmill-labs/windmill/commit/8a3a9bda1c03661c4072c9ce53c07ee632e3c6ca))


### Bug Fixes

* **cli:** improve restart_unless_cancelled handling ([16507ad](https://github.com/windmill-labs/windmill/commit/16507ad45eba1cb2212f22a3cce05b60b1d3a39c))

## [1.266.1](https://github.com/windmill-labs/windmill/compare/v1.266.0...v1.266.1) (2024-02-10)


### Bug Fixes

* improve load schedule args ([31469cb](https://github.com/windmill-labs/windmill/commit/31469cb77c5a768f2720fb711a203c57089a5a3b))

## [1.266.0](https://github.com/windmill-labs/windmill/compare/v1.265.3...v1.266.0) (2024-02-09)


### Features

* git sync now accepts path filters and type filters ([#3189](https://github.com/windmill-labs/windmill/issues/3189)) ([e9a6c81](https://github.com/windmill-labs/windmill/commit/e9a6c8154c4fc1bf52e26251a9d75e54c511837d))


### Bug Fixes

* add --unstable-http arg to deno ([#3186](https://github.com/windmill-labs/windmill/issues/3186)) ([8d5c7c2](https://github.com/windmill-labs/windmill/commit/8d5c7c2b461ee71b8376b12f574dffff1c9f1387))
* **frontend:** fix html component initial data ([#3188](https://github.com/windmill-labs/windmill/issues/3188)) ([6a13b97](https://github.com/windmill-labs/windmill/commit/6a13b97bc3561fe4bdeb8a02e88f80f749d906f6))
* improve no flow overlap + schedule args loading ([6614817](https://github.com/windmill-labs/windmill/commit/6614817cd4c208eebc52955f408041b14b9dd34f))
* improve no flow overlap + schedule args loading ([48b8520](https://github.com/windmill-labs/windmill/commit/48b8520239a591756e4573190e3fa45b57e52535))
* improve on-boarding flow app ([4d64c94](https://github.com/windmill-labs/windmill/commit/4d64c942fced44b4ca4af2801009849d5dd1f55b))
* Properly handle pip index urls in pip-compile ([#3192](https://github.com/windmill-labs/windmill/issues/3192)) ([b230378](https://github.com/windmill-labs/windmill/commit/b230378320336331de1c50d6fdec60da576751e6))

## [1.265.3](https://github.com/windmill-labs/windmill/compare/v1.265.2...v1.265.3) (2024-02-08)


### Bug Fixes

* ai builder colors ([b9c0eda](https://github.com/windmill-labs/windmill/commit/b9c0eda16874ee7a85a63872ea20636f64a48c90))

## [1.265.2](https://github.com/windmill-labs/windmill/compare/v1.265.1...v1.265.2) (2024-02-08)


### Bug Fixes

* Cuda image building ([#3179](https://github.com/windmill-labs/windmill/issues/3179)) ([29be502](https://github.com/windmill-labs/windmill/commit/29be5021ff9603d6cc7e302c46fb44c573967b80))
* **frontend:** Fix tutorials + Move into itself + Disable app history… ([#3181](https://github.com/windmill-labs/windmill/issues/3181)) ([9f98caa](https://github.com/windmill-labs/windmill/commit/9f98caa072b3ac83b60e9c9b0f31d39ae072b08a))
* handle better \u0000 in python result ([e4dc972](https://github.com/windmill-labs/windmill/commit/e4dc972d40770e5d0ffb2c2726b0053cdff7e407))
* Improve python writeS3File perf ([#3182](https://github.com/windmill-labs/windmill/issues/3182)) ([e00e3f9](https://github.com/windmill-labs/windmill/commit/e00e3f9d2d65b81cdf8abf7500deb5b4f5ecf607))

## [1.265.1](https://github.com/windmill-labs/windmill/compare/v1.265.0...v1.265.1) (2024-02-07)


### Bug Fixes

* graphql web worker ([#3177](https://github.com/windmill-labs/windmill/issues/3177)) ([361ea76](https://github.com/windmill-labs/windmill/commit/361ea7627982f36797138f58fe8ad9c503729e34))

## [1.265.0](https://github.com/windmill-labs/windmill/compare/v1.264.0...v1.265.0) (2024-02-07)


### Features

* Worker env variables are hidden to developers in the config panel ([#3175](https://github.com/windmill-labs/windmill/issues/3175)) ([126aa60](https://github.com/windmill-labs/windmill/commit/126aa60a9da2595f7f64a6e14b2467ac933acd22))


### Bug Fixes

* **frontend:** handle not found folder in FolderEditor ([#3170](https://github.com/windmill-labs/windmill/issues/3170)) ([284e43c](https://github.com/windmill-labs/windmill/commit/284e43c0644b05784497a2494de3a0e59f897dd4))
* improve agent policies ([73ff48b](https://github.com/windmill-labs/windmill/commit/73ff48bc8356c3c205f6a74eba7ad1c99c68e097))

## [1.264.0](https://github.com/windmill-labs/windmill/compare/v1.263.1...v1.264.0) (2024-02-07)


### Features

* violet ai branding + flow summary ([#3171](https://github.com/windmill-labs/windmill/issues/3171)) ([91743c3](https://github.com/windmill-labs/windmill/commit/91743c3cfbce22a72ca33feef4cd2fa4714a4282))


### Bug Fixes

* add audit logs to worker configs ([cd78c67](https://github.com/windmill-labs/windmill/commit/cd78c6766d2c92ba7759faf768ff93788ec6b7d8))
* export base64 from typescript client ([0af0aae](https://github.com/windmill-labs/windmill/commit/0af0aae0b537ec2b24205745d4d4d1dcfda1901c))
* **frontend:** Fix delete script ([#3166](https://github.com/windmill-labs/windmill/issues/3166)) ([83b8d62](https://github.com/windmill-labs/windmill/commit/83b8d628f006533eb23bf223e4a2bcd17155dcf2))
* Main compile breaks ([#3169](https://github.com/windmill-labs/windmill/issues/3169)) ([6edc4c4](https://github.com/windmill-labs/windmill/commit/6edc4c4fc822763992e33ab2dbeafbfeee0de1c8))
* only create shared dir symlink if not exists ([75e210b](https://github.com/windmill-labs/windmill/commit/75e210bfc7f7b6e32acdad74e222be3b28062cd1))
* SAML redirect uses SAMLRequest in URL ([#3168](https://github.com/windmill-labs/windmill/issues/3168)) ([812516b](https://github.com/windmill-labs/windmill/commit/812516bb06539cc408d2c46366f9adaa7925261d))

## [1.263.1](https://github.com/windmill-labs/windmill/compare/v1.263.0...v1.263.1) (2024-02-06)


### Bug Fixes

* **frontend:** fix toggles margins ([#3165](https://github.com/windmill-labs/windmill/issues/3165)) ([a352d85](https://github.com/windmill-labs/windmill/commit/a352d85b0ba4756226b98032e0879a9f91bb20e4))
* improve ts wrappers ([#3163](https://github.com/windmill-labs/windmill/issues/3163)) ([0fc2221](https://github.com/windmill-labs/windmill/commit/0fc22213e45691e5d42dacdbf903ae0416a9d599))

## [1.263.0](https://github.com/windmill-labs/windmill/compare/v1.262.1...v1.263.0) (2024-02-06)


### Features

* **frontend:** add status when a flow setting is enabled ([#3161](https://github.com/windmill-labs/windmill/issues/3161)) ([8a8c1d3](https://github.com/windmill-labs/windmill/commit/8a8c1d3c3f6ad6053fbd107a0f0e85d70f4215ac))
* new ai design ([#3152](https://github.com/windmill-labs/windmill/issues/3152)) ([58d3484](https://github.com/windmill-labs/windmill/commit/58d34845a05cd215f682bc9081b0cb883d174676))


### Bug Fixes

* add ping since to list workers ([557d0bc](https://github.com/windmill-labs/windmill/commit/557d0bcbef185aaef90d4058d9f01fa567bef349))
* **frontend:** fix dateslider doclink ([#3159](https://github.com/windmill-labs/windmill/issues/3159)) ([bc75a5a](https://github.com/windmill-labs/windmill/commit/bc75a5acbce89d3ff4332ecbe7b98c63d5e55e1b))
* **frontend:** Fix operators actions + small UI fixes ([#3157](https://github.com/windmill-labs/windmill/issues/3157)) ([4faedfe](https://github.com/windmill-labs/windmill/commit/4faedfe58926cfa770857a514d08749f3e40d17e))
* go client sets resource properly ([#3160](https://github.com/windmill-labs/windmill/issues/3160)) ([057b415](https://github.com/windmill-labs/windmill/commit/057b415e9ab5a2e5789be7181c86e885edd86c7f))
* increase default max conn of a worker to 4 ([887bf68](https://github.com/windmill-labs/windmill/commit/887bf6872b5b5438ca5f33ec5dda5162c1c4b132))
* scim added users are now auto-added/invited ([1352add](https://github.com/windmill-labs/windmill/commit/1352add8c65de5266d40baf3d9058a6f4ea3d9b0))

## [1.262.1](https://github.com/windmill-labs/windmill/compare/v1.262.0...v1.262.1) (2024-02-05)


### Bug Fixes

* add get_root_job_id typescript-client ([9877c5f](https://github.com/windmill-labs/windmill/commit/9877c5fd9da0b912a90efc0bfdc3a0ba08b04bfc))

## [1.262.0](https://github.com/windmill-labs/windmill/compare/v1.261.0...v1.262.0) (2024-02-05)


### Features

* **frontend:** add support for toasts in frontend scripts ([#3147](https://github.com/windmill-labs/windmill/issues/3147)) ([81174ab](https://github.com/windmill-labs/windmill/commit/81174abf5a9353535df3c7a8ce92a4e354689af5))
* **frontend:** alert component ([#3140](https://github.com/windmill-labs/windmill/issues/3140)) ([2637fa2](https://github.com/windmill-labs/windmill/commit/2637fa23bbae00b17bc08c31dc8a751b8c1581a7))
* **frontend:** App date slider component ([#3146](https://github.com/windmill-labs/windmill/issues/3146)) ([4c37479](https://github.com/windmill-labs/windmill/commit/4c37479b67b7515b905c7f043c4646fa0b3c80ec))


### Bug Fixes

* add get_root_job_id ([60f3a9f](https://github.com/windmill-labs/windmill/commit/60f3a9fa6b54ca5f62d10a82d974e78f2faaa198))
* auto-add user add user to the group all ([c067a87](https://github.com/windmill-labs/windmill/commit/c067a875710e2fa217d7c973e5d4d109d5bf0aa5))
* go preload wmill dependencies ([#3148](https://github.com/windmill-labs/windmill/issues/3148)) ([364284c](https://github.com/windmill-labs/windmill/commit/364284cf4121eca0fdb24d130cdf73b156c16886))
* go preload wmill dependencies ([#3149](https://github.com/windmill-labs/windmill/issues/3149)) ([9c9e543](https://github.com/windmill-labs/windmill/commit/9c9e5439968b0f141b8c6b90ced558f8c93f48ca))
* go preload wmill dependencies ([#3150](https://github.com/windmill-labs/windmill/issues/3150)) ([40c3b91](https://github.com/windmill-labs/windmill/commit/40c3b916d86a810ff7de57bc77f40c78131a784c))
* remove duplicated on deployment management UI ([6c184eb](https://github.com/windmill-labs/windmill/commit/6c184eb4e4da3af83792b56fb1cd987a53037356))
* scheduling of flows is done immediately ([a89f681](https://github.com/windmill-labs/windmill/commit/a89f6817f0af971c0ba85f5ce2bedda51126f0e0))

## [1.261.0](https://github.com/windmill-labs/windmill/compare/v1.260.1...v1.261.0) (2024-02-05)


### Features

* add flow debug info endpoint + button ([608c759](https://github.com/windmill-labs/windmill/commit/608c7597aedfb896376b2272e45a30a9e7545b58))
* add nobypassrls migration ([97d1349](https://github.com/windmill-labs/windmill/commit/97d134994edca26156aa3de85c5e9651e605dc5a))


### Bug Fixes

* add support for ephemeral tokens ([95952da](https://github.com/windmill-labs/windmill/commit/95952da387fa9954292932588bd1c106e811e59a))
* decrease database connections of workers to 3 ([2c4eb46](https://github.com/windmill-labs/windmill/commit/2c4eb46e65dfed981c0fce1acf35ea36225c6063))

## [1.260.1](https://github.com/windmill-labs/windmill/compare/v1.260.0...v1.260.1) (2024-02-02)


### Bug Fixes

* retrigger release ([160f91e](https://github.com/windmill-labs/windmill/commit/160f91e0be87a6ccf59eb99589ac7f73fa43d10c))

## [1.260.0](https://github.com/windmill-labs/windmill/compare/v1.259.2...v1.260.0) (2024-02-02)


### Features

* disable self approval toggle ([#3137](https://github.com/windmill-labs/windmill/issues/3137)) ([017d9b8](https://github.com/windmill-labs/windmill/commit/017d9b86707c8c11d41dec7532c440cc79801167))

## [1.259.2](https://github.com/windmill-labs/windmill/compare/v1.259.1...v1.259.2) (2024-02-02)


### Bug Fixes

* **frontend:** Fix dt branch ([#3124](https://github.com/windmill-labs/windmill/issues/3124)) ([fc8ef58](https://github.com/windmill-labs/windmill/commit/fc8ef5867acc7e4a62c8d8e378d179a240c02a66))
* Python buffered reader ([#3136](https://github.com/windmill-labs/windmill/issues/3136)) ([86aa6d0](https://github.com/windmill-labs/windmill/commit/86aa6d0f0dee237c10ace4a278b9a0dc03460f03))

## [1.259.1](https://github.com/windmill-labs/windmill/compare/v1.259.0...v1.259.1) (2024-02-02)


### Bug Fixes

* **python:** fix python reader ([583e942](https://github.com/windmill-labs/windmill/commit/583e942174673ec40f263a932912fac7ec00c383))

## [1.259.0](https://github.com/windmill-labs/windmill/compare/v1.258.4...v1.259.0) (2024-02-01)


### Features

* ai cron ([#3128](https://github.com/windmill-labs/windmill/issues/3128)) ([c4308de](https://github.com/windmill-labs/windmill/commit/c4308de7206a200a588f98f947fc532189c92ef0))
* auto-add users ([#3114](https://github.com/windmill-labs/windmill/issues/3114)) ([6b772dd](https://github.com/windmill-labs/windmill/commit/6b772dd2a7b7e5c9b58777a3e3e99f8b46775e93))
* **frontend:** add support for dynamic default values + enums ([#3109](https://github.com/windmill-labs/windmill/issues/3109)) ([ba10432](https://github.com/windmill-labs/windmill/commit/ba104324805edbced9c2adfbeccf8d83b91ea62d))
* generate script summary ([#3110](https://github.com/windmill-labs/windmill/issues/3110)) ([1446cb4](https://github.com/windmill-labs/windmill/commit/1446cb45adbbb4fa59ca2ca98e9d7dd37af26a76))
* migrate s3 client to object_store ([#3116](https://github.com/windmill-labs/windmill/issues/3116)) ([5dabe22](https://github.com/windmill-labs/windmill/commit/5dabe22935134984d017c20f4ce44389a67b5e4a))


### Bug Fixes

* **frontend:** fix DB studio when columns have space in their names ([#3126](https://github.com/windmill-labs/windmill/issues/3126)) ([8a8a30c](https://github.com/windmill-labs/windmill/commit/8a8a30c5ecf894945f812561ad0f79db7a75d541))
* **frontend:** fix schema form toolips ([#3123](https://github.com/windmill-labs/windmill/issues/3123)) ([5a6fc48](https://github.com/windmill-labs/windmill/commit/5a6fc4891f36f200f60e97ae168516fedd6d4411))
* handle array of null in display result ([e185eb3](https://github.com/windmill-labs/windmill/commit/e185eb37caf603d10a4e6084e6151d74735bd17b))
* metadata gen typo ([#3125](https://github.com/windmill-labs/windmill/issues/3125)) ([c926e71](https://github.com/windmill-labs/windmill/commit/c926e714dc5d683208171c5c7308ba6da2f04204))
* pg coerce nb to string ([#3127](https://github.com/windmill-labs/windmill/issues/3127)) ([b9d5506](https://github.com/windmill-labs/windmill/commit/b9d550679339f6fafc173fe68ab40730dc84376e))

## [1.258.4](https://github.com/windmill-labs/windmill/compare/v1.258.3...v1.258.4) (2024-01-31)


### Bug Fixes

* improve git sync ([23f06d1](https://github.com/windmill-labs/windmill/commit/23f06d1a0424862afd6f733ed103f7a66f59dd12))
* improve git sync ([cc0aec8](https://github.com/windmill-labs/windmill/commit/cc0aec87438ebe4aef6dae1464b9dc267da3ad97))

## [1.258.3](https://github.com/windmill-labs/windmill/compare/v1.258.2...v1.258.3) (2024-01-31)


### Bug Fixes

* **cli:** restore other files support ([7494e7e](https://github.com/windmill-labs/windmill/commit/7494e7ee2d7f56a95b6f976b9646c2bce3bdc22b))
* **frontend:** Fix adding nodes to decision tree ([#3107](https://github.com/windmill-labs/windmill/issues/3107)) ([740801f](https://github.com/windmill-labs/windmill/commit/740801f4a73849919419ee96f4fc090eef855a17))
* improve git sync ([b063164](https://github.com/windmill-labs/windmill/commit/b0631648c3899134e668c30b890efc40aa2c0c49))
* improve git sync ([1c4129c](https://github.com/windmill-labs/windmill/commit/1c4129c4f68f690eac6bffe9016bc8506bb9cf37))

## [1.258.2](https://github.com/windmill-labs/windmill/compare/v1.258.1...v1.258.2) (2024-01-31)


### Bug Fixes

* **cli:** push folders first + on_behalf_of stripped from metadata ([d2cbc7a](https://github.com/windmill-labs/windmill/commit/d2cbc7a41671e91e07fc1e81e966c51133abcc72))

## [1.258.1](https://github.com/windmill-labs/windmill/compare/v1.258.0...v1.258.1) (2024-01-31)


### Bug Fixes

* **cli:** add support for restart_unless_cancelled in sync ([b4d0a3c](https://github.com/windmill-labs/windmill/commit/b4d0a3c4239973fa32e198b117c25e0d13a53b4c))
* fix RETENTION period setting UI ([5cec2ed](https://github.com/windmill-labs/windmill/commit/5cec2edaefd908a5bf3c9853035e6e2d2466656d))

## [1.258.0](https://github.com/windmill-labs/windmill/compare/v1.257.0...v1.258.0) (2024-01-30)


### Features

* Support sending SIGINT to jobs ([#3094](https://github.com/windmill-labs/windmill/issues/3094)) ([a719170](https://github.com/windmill-labs/windmill/commit/a719170a6ae125f03312fc1c12a73fd2f01a09c6))


### Bug Fixes

* improve array static editor ([ef17fd0](https://github.com/windmill-labs/windmill/commit/ef17fd0f5ea02df218200709f742de2c5ae4cc76))

## [1.257.0](https://github.com/windmill-labs/windmill/compare/v1.256.0...v1.257.0) (2024-01-30)


### Features

* **frontend:** Correctly set the licence key on the approval page ([#3112](https://github.com/windmill-labs/windmill/issues/3112)) ([8ebc90a](https://github.com/windmill-labs/windmill/commit/8ebc90abb2e9d99c4f20cb2e14c0a5487d5a11a0))


### Bug Fixes

* **cli:** avoid flows in script generate-metadata ([565e166](https://github.com/windmill-labs/windmill/commit/565e1668b372e7ac4482fc13962b32b1dbb084b3))
* **frontend:** expose Filters + Displayed row count in the outputs ([#3101](https://github.com/windmill-labs/windmill/issues/3101)) ([b55c0bd](https://github.com/windmill-labs/windmill/commit/b55c0bd2c5a1646709328d8ee9b191a34a2c976c))
* **frontend:** fix logpanel ([#3111](https://github.com/windmill-labs/windmill/issues/3111)) ([64441b3](https://github.com/windmill-labs/windmill/commit/64441b34522ba99ce44e74a2c81dcc4999acdfa3))
* simplify folder creation by non admins ([34253fd](https://github.com/windmill-labs/windmill/commit/34253fd43b01fbe15dafe233d008fc9b98c29828))

## [1.256.0](https://github.com/windmill-labs/windmill/compare/v1.255.0...v1.256.0) (2024-01-30)


### Features

* list jobs metrics ([#3104](https://github.com/windmill-labs/windmill/issues/3104)) ([26d5c6c](https://github.com/windmill-labs/windmill/commit/26d5c6c1ba2dfa7edb0217bf7dc89f45279781a7))


### Bug Fixes

* add support for NPM_CONFIG_REGISTRY ([47fcfbb](https://github.com/windmill-labs/windmill/commit/47fcfbbbdf179d241f1deb3d5527f8faa0a8132e))
* add support for NPM_CONFIG_REGISTRY ([27f4624](https://github.com/windmill-labs/windmill/commit/27f4624b30fcb04c98d093e49c614baa5a938528))
* **frontend:** fix chartjs when resolvedDatasets is not defined ([#3106](https://github.com/windmill-labs/windmill/issues/3106)) ([0102dce](https://github.com/windmill-labs/windmill/commit/0102dcef8ecbc1d8a9528bac8fdfd95b93689155))

## [1.255.0](https://github.com/windmill-labs/windmill/compare/v1.254.1...v1.255.0) (2024-01-29)


### Features

* **cli:** allow all sync options to be passable from wmill.yaml directly ([2a80df4](https://github.com/windmill-labs/windmill/commit/2a80df4a80cbd4ccecf2066e55efe092b070fefa))
* **cli:** global generate-metadata + inherit deps from closest package.json/requirements.txt + bun settable as default ([#3102](https://github.com/windmill-labs/windmill/issues/3102)) ([49c1bc5](https://github.com/windmill-labs/windmill/commit/49c1bc50f3ae6aba42317c2a0ef181927e915d26))
* **cli:** make --raw the default for cli sync ([28a1966](https://github.com/windmill-labs/windmill/commit/28a196657fa60d4db7c6528367e578b1202c5353))
* **cli:** make default typescript configurable ([1f46bcb](https://github.com/windmill-labs/windmill/commit/1f46bcba72592e39820074d160381fe0ff42ac70))
* Download s3 file as stream in Python and TS ([#3099](https://github.com/windmill-labs/windmill/issues/3099)) ([6160889](https://github.com/windmill-labs/windmill/commit/616088979378712eab5b3abbb646d3e688d2cece))
* **frontend:** handle file default value ([#3095](https://github.com/windmill-labs/windmill/issues/3095)) ([94ddf80](https://github.com/windmill-labs/windmill/commit/94ddf803566e94c443f989fc0c7bde578b704786))
* Passing HOME env var through to python workers ([#3092](https://github.com/windmill-labs/windmill/issues/3092)) ([ec911f6](https://github.com/windmill-labs/windmill/commit/ec911f6a5a68b9d4b2703bef8ebc3342c755a35a))
* update openai models + increase length + improve code completion ([#3097](https://github.com/windmill-labs/windmill/issues/3097)) ([6d77578](https://github.com/windmill-labs/windmill/commit/6d77578590cc666272bb364d21b7f2a3fe4494ac))


### Bug Fixes

* **frontend:** various UI fix ([#3098](https://github.com/windmill-labs/windmill/issues/3098)) ([cbfa5ff](https://github.com/windmill-labs/windmill/commit/cbfa5ff8871097c4f292e32b850a6c10a0809575))
* improve display result ([ff559ec](https://github.com/windmill-labs/windmill/commit/ff559ecdbdfedc09e17a1ccfc94d17d5b587c453))
* improve schedule args clearing on script change ([59e0be7](https://github.com/windmill-labs/windmill/commit/59e0be77ad215e13dbd1e071766152c3fa676fde))
* update deno to 1.38 -&gt; 1.40.2 ([a5d2536](https://github.com/windmill-labs/windmill/commit/a5d25362dbbe09c0c544ddddf0749936cdb8e317))
* use extra headers when urlencoded ([#3103](https://github.com/windmill-labs/windmill/issues/3103)) ([8fcf119](https://github.com/windmill-labs/windmill/commit/8fcf119798c576a57e6dee3ed5119b7b5d1130f6))

## [1.254.1](https://github.com/windmill-labs/windmill/compare/v1.254.0...v1.254.1) (2024-01-27)


### Bug Fixes

* render all responsiveness fix ([18d832c](https://github.com/windmill-labs/windmill/commit/18d832c6347c31fb2c2ddc1268cfd066d01352ae))
* render all responsiveness fix ([dfabb37](https://github.com/windmill-labs/windmill/commit/dfabb371003e955a05e4d8672fb95a637f236a72))

## [1.254.0](https://github.com/windmill-labs/windmill/compare/v1.253.8...v1.254.0) (2024-01-26)


### Features

* **frontend:** add support for render all ([#3084](https://github.com/windmill-labs/windmill/issues/3084)) ([4607939](https://github.com/windmill-labs/windmill/commit/460793954944f695d6c8fed88424d48571348135))
* **frontend:** S3 resource schema ([#3083](https://github.com/windmill-labs/windmill/issues/3083)) ([fa8a6e8](https://github.com/windmill-labs/windmill/commit/fa8a6e8c3dcfcca6f17c2fdd4910ab7980192e11))


### Bug Fixes

* **frontend:** add missing InitializeComponent ([#3088](https://github.com/windmill-labs/windmill/issues/3088)) ([6a73ccf](https://github.com/windmill-labs/windmill/commit/6a73ccf6261d58f019bbb01e10c4f81be62f66e6))
* **frontend:** Fix currency input dark mode ([#3085](https://github.com/windmill-labs/windmill/issues/3085)) ([bcc341c](https://github.com/windmill-labs/windmill/commit/bcc341c255ee04dfbf28ed678b025310f57e501c))
* load input history correctly on past versions ([80eeba5](https://github.com/windmill-labs/windmill/commit/80eeba5ee34204876a4db4e3fda7539247cd0322))

## [1.253.8](https://github.com/windmill-labs/windmill/compare/v1.253.7...v1.253.8) (2024-01-26)


### Bug Fixes

* add support for instance name for mssql ([91289a0](https://github.com/windmill-labs/windmill/commit/91289a0d5a96d6f7b11e60d512d0d3a9ceb341e7))
* git sync include changing permissions on apps, scripts, flows ([ee965a1](https://github.com/windmill-labs/windmill/commit/ee965a1a4c1e122c5b0adb93c548b41d587bd0a2))
* git sync include changing permissions on folders ([0f6c127](https://github.com/windmill-labs/windmill/commit/0f6c127002cee35479231140ec96eef2176bba42))
* improve git sync on rename/deletion ([a025146](https://github.com/windmill-labs/windmill/commit/a0251463f803db7e35572ba68862345d2b53a399))

## [1.253.7](https://github.com/windmill-labs/windmill/compare/v1.253.6...v1.253.7) (2024-01-25)


### Bug Fixes

* Improvement for Python write_s3_file ([#3079](https://github.com/windmill-labs/windmill/issues/3079)) ([082aa6a](https://github.com/windmill-labs/windmill/commit/082aa6a61d860354929efcccfcff695c5ccc8c1d))

## [1.253.6](https://github.com/windmill-labs/windmill/compare/v1.253.5...v1.253.6) (2024-01-25)


### Bug Fixes

* merge conflicts ([7d6039f](https://github.com/windmill-labs/windmill/commit/7d6039f9694bfef672c508dbc2699d33bedeae23))

## [1.253.5](https://github.com/windmill-labs/windmill/compare/v1.253.4...v1.253.5) (2024-01-25)


### Bug Fixes

* openapi definition ([09e08c3](https://github.com/windmill-labs/windmill/commit/09e08c3c94dfff12ac8df9750bea3fadfa455f40))

## [1.253.4](https://github.com/windmill-labs/windmill/compare/v1.253.3...v1.253.4) (2024-01-25)


### Bug Fixes

* add s3 parquet file renderer ([4a6710e](https://github.com/windmill-labs/windmill/commit/4a6710ea913da64ce2caf93499c9ec69d89b3cf3))
* add s3 parquet file renderer ([2f8243b](https://github.com/windmill-labs/windmill/commit/2f8243b39085222a3bde8c31fce6fc36a7b8f453))
* improve parquet renderer error ([4b348a0](https://github.com/windmill-labs/windmill/commit/4b348a032e3cb7ea3c945635ed8d3cb1bd91b9b4))

## [1.253.3](https://github.com/windmill-labs/windmill/compare/v1.253.2...v1.253.3) (2024-01-25)


### Bug Fixes

* Better UI for S3 download and S3 TS SDK endpoints ([#3065](https://github.com/windmill-labs/windmill/issues/3065)) ([da6edee](https://github.com/windmill-labs/windmill/commit/da6edee4505318deef4557a82eb61356a5f1bfb4))

## [1.253.2](https://github.com/windmill-labs/windmill/compare/v1.253.1...v1.253.2) (2024-01-24)


### Bug Fixes

* **app:** improve app reactivity ([737c4fb](https://github.com/windmill-labs/windmill/commit/737c4fb497ce4d04c8404b745dd00868724cfc51))

## [1.253.1](https://github.com/windmill-labs/windmill/compare/v1.253.0...v1.253.1) (2024-01-24)


### Bug Fixes

* **scim:** improve get_user for scim ([a66208f](https://github.com/windmill-labs/windmill/commit/a66208f326cb1be9ede6408beb4b6357d7a8bd57))

## [1.253.0](https://github.com/windmill-labs/windmill/compare/v1.252.0...v1.253.0) (2024-01-24)


### Features

* **frontend:** app editor right click menu ([#3050](https://github.com/windmill-labs/windmill/issues/3050)) ([2b8c0bb](https://github.com/windmill-labs/windmill/commit/2b8c0bbaeae22e2231edb7b8560f58003312353f))


### Bug Fixes

* bun default registry and performance improvements ([801106e](https://github.com/windmill-labs/windmill/commit/801106e9b8878e5fd1acd805497ed81f0bf5c99c))

## [1.252.0](https://github.com/windmill-labs/windmill/compare/v1.251.1...v1.252.0) (2024-01-24)


### Features

* **cli:** add support for excludes in yaml.conf ([21bf011](https://github.com/windmill-labs/windmill/commit/21bf0115fc3e9af96262594eb76d664a3c531498))
* Custom concurrency key for scripts ([#3046](https://github.com/windmill-labs/windmill/issues/3046)) ([f189224](https://github.com/windmill-labs/windmill/commit/f189224b8d0b17fe7a5d78795ac4129f31919538))
* exporting tarball/sync doesn't require admin perms anymore ([c2fb24d](https://github.com/windmill-labs/windmill/commit/c2fb24d4803b9654c248f84861e86b99b559b6eb))
* **frontend:** add support for range area ([#3068](https://github.com/windmill-labs/windmill/issues/3068)) ([0dd54f9](https://github.com/windmill-labs/windmill/commit/0dd54f93bb652af17173697682a6080a513b7f5f))
* **frontend:** Ag Grid compactness ([#3052](https://github.com/windmill-labs/windmill/issues/3052)) ([1ffb4c5](https://github.com/windmill-labs/windmill/commit/1ffb4c5abebc418f41a969a983a03a2ebc521ea6))
* **frontend:** Changelog updates ([#3067](https://github.com/windmill-labs/windmill/issues/3067)) ([c14b880](https://github.com/windmill-labs/windmill/commit/c14b880a7c93424ff13cf0fe8d6e9499af6e56ac))
* **frontend:** display a warning with a documentation link on how to… ([#3012](https://github.com/windmill-labs/windmill/issues/3012)) ([c73bdad](https://github.com/windmill-labs/windmill/commit/c73bdad08c556b656c7fdcd509f097de675126c4))
* Set a default app for each workspace ([#3014](https://github.com/windmill-labs/windmill/issues/3014)) ([3225420](https://github.com/windmill-labs/windmill/commit/32254203d81d4353fbfb7f6cd7b72f1e237dc45c))


### Bug Fixes

* fix add resource with keys as objects ([0c88abc](https://github.com/windmill-labs/windmill/commit/0c88abccf0ebbeccd3bfb0b6a066feb78cc8ecf3))
* improve decision tree ([e6b8f73](https://github.com/windmill-labs/windmill/commit/e6b8f73c9cc8d94b58388726c5a31832871c398e))
* no-emit-index-url on lockfiles ([450267a](https://github.com/windmill-labs/windmill/commit/450267a84e428a7b5b107aa50ddb51a0f0a7f187))
* remove first part of account_identifier for snowflake ([e365693](https://github.com/windmill-labs/windmill/commit/e365693497e7055ee891748621ee3472efc6047a))

## [1.251.1](https://github.com/windmill-labs/windmill/compare/v1.251.0...v1.251.1) (2024-01-23)


### Bug Fixes

* improve npm typescript client ([5b98b00](https://github.com/windmill-labs/windmill/commit/5b98b005cda2615b332d904dc3d4a6a7898ae46e))

## [1.251.0](https://github.com/windmill-labs/windmill/compare/v1.250.0...v1.251.0) (2024-01-22)


### Features

* Download button for s3 files ([#3059](https://github.com/windmill-labs/windmill/issues/3059)) ([376038d](https://github.com/windmill-labs/windmill/commit/376038d70b04fe012566872bee6a4cc4b46db0c7))

## [1.250.0](https://github.com/windmill-labs/windmill/compare/v1.249.0...v1.250.0) (2024-01-22)


### Features

* deprecate .wmillignore in favor of wmill.yaml/includes ([b8defbc](https://github.com/windmill-labs/windmill/commit/b8defbcfc68181397ae50daba6616df26e4383fd))


### Bug Fixes

* fix initialization callback of AppDbexplorer ([1fcdad7](https://github.com/windmill-labs/windmill/commit/1fcdad7f7b284d0a66db57df0582c89a146ef653))

## [1.249.0](https://github.com/windmill-labs/windmill/compare/v1.248.0...v1.249.0) (2024-01-21)


### Features

* File path is option when uploading a file to S3 ([#3029](https://github.com/windmill-labs/windmill/issues/3029)) ([bbf897a](https://github.com/windmill-labs/windmill/commit/bbf897a718f403de0e9809914acd18b4b79fd605))
* improve cli to generate proper metadata and schema by default ([d1eed4e](https://github.com/windmill-labs/windmill/commit/d1eed4e09d09b4ec2a3aa27608dc2a70fc0a4d0a))
* improve handling of pinned versions for bun ([ab010ce](https://github.com/windmill-labs/windmill/commit/ab010ce4f3a0628d4699f558bd463e657a8f1a97))
* non owner can resume flows if resume url is in message ([ac87e2f](https://github.com/windmill-labs/windmill/commit/ac87e2f85b55742fd1472b3ba18cc1e523b56b98))


### Bug Fixes

* **frontend:** Fix display for array of objects ([#3051](https://github.com/windmill-labs/windmill/issues/3051)) ([773e2d3](https://github.com/windmill-labs/windmill/commit/773e2d3103d23b5eb31a47565d577461adf239df))
* improve approval/prompt helpers ([0d7d2ef](https://github.com/windmill-labs/windmill/commit/0d7d2efde8b6bb00ba8d14ce341fe230d79fe9f2))
* more explicit CLI error ([#3049](https://github.com/windmill-labs/windmill/issues/3049)) ([c4f0b67](https://github.com/windmill-labs/windmill/commit/c4f0b67fb65b705f7fecae93991880925f7b3c80))
* use hash on objects instead of shallow equal comparison to improve trigger reliability of apps ([d1cfe7c](https://github.com/windmill-labs/windmill/commit/d1cfe7c202cfa34bdc46997a50c278c9f8ff4fb0))

## [1.248.0](https://github.com/windmill-labs/windmill/compare/v1.247.0...v1.248.0) (2024-01-19)


### Features

* improve SCIM support for groups ([77f7fb2](https://github.com/windmill-labs/windmill/commit/77f7fb2dd35c337a3a79fcf9038c73cf3939f85f))


### Bug Fixes

* fix rename apps from home menu ([91ea031](https://github.com/windmill-labs/windmill/commit/91ea031f281af8776e94890e80101197f9c71e46))

## [1.247.0](https://github.com/windmill-labs/windmill/compare/v1.246.15...v1.247.0) (2024-01-19)


### Features

* **frontend:** Rich table display ([#3028](https://github.com/windmill-labs/windmill/issues/3028)) ([54cad28](https://github.com/windmill-labs/windmill/commit/54cad2886b7e5ca26c6da9547376b43daac9e881))


### Bug Fixes

* add ability to rename group from scim ([9fefdcc](https://github.com/windmill-labs/windmill/commit/9fefdccc132c4a61ab0a72a86c4dc9bbb23a811e))
* **frontend:** fix hidden wizards ([#3045](https://github.com/windmill-labs/windmill/issues/3045)) ([b64eb3d](https://github.com/windmill-labs/windmill/commit/b64eb3d8b10ba6ca453715dbf8d18b8ad7e3db17))
* improve onDemandOnly runnables ([a0d7ea2](https://github.com/windmill-labs/windmill/commit/a0d7ea22b4cd29f47ec308c9c6e2c2d8ab0ed38f))

## [1.246.15](https://github.com/windmill-labs/windmill/compare/v1.246.14...v1.246.15) (2024-01-18)


### Bug Fixes

* improve error message format for logs ([582339c](https://github.com/windmill-labs/windmill/commit/582339c83ef32a81c3b02d88a47fab3fd18ce2fe))

## [1.246.14](https://github.com/windmill-labs/windmill/compare/v1.246.13...v1.246.14) (2024-01-18)


### Bug Fixes

* fix OIDC issuer ([8b302b4](https://github.com/windmill-labs/windmill/commit/8b302b4dc564c6336deca26d36c270c70a0efd98))

## [1.246.13](https://github.com/windmill-labs/windmill/compare/v1.246.12...v1.246.13) (2024-01-18)


### Bug Fixes

* cli sync improvement ([58cad70](https://github.com/windmill-labs/windmill/commit/58cad70363310eb107a3e4c8c7b9428630e5deb3))

## [1.246.12](https://github.com/windmill-labs/windmill/compare/v1.246.11...v1.246.12) (2024-01-18)


### Bug Fixes

* cli sync improvement ([c8f269b](https://github.com/windmill-labs/windmill/commit/c8f269b870adac9628b197f4ab3516ccdbd590d3))

## [1.246.11](https://github.com/windmill-labs/windmill/compare/v1.246.10...v1.246.11) (2024-01-18)


### Bug Fixes

* improve cli script deletion ([20c422c](https://github.com/windmill-labs/windmill/commit/20c422c5467b4188a7618bc4c7c303c70346a5a9))

## [1.246.10](https://github.com/windmill-labs/windmill/compare/v1.246.9...v1.246.10) (2024-01-18)


### Bug Fixes

* make cli backcompatible with respect to lockfile ([eb000f1](https://github.com/windmill-labs/windmill/commit/eb000f1fbc697869aab46b3be430c8d56b7a7e8c))
* make some eval onDemandOnly ([36905da](https://github.com/windmill-labs/windmill/commit/36905daef60d78725d5b78d70e314281ed297565))

## [1.246.9](https://github.com/windmill-labs/windmill/compare/v1.246.8...v1.246.9) (2024-01-17)


### Bug Fixes

* avoid too long diffs in cli ([828cdd4](https://github.com/windmill-labs/windmill/commit/828cdd45f08a12071a028052b19503a441f047b1))

## [1.246.8](https://github.com/windmill-labs/windmill/compare/v1.246.7...v1.246.8) (2024-01-17)


### Bug Fixes

* improve lockfile handling for cli ([d00de26](https://github.com/windmill-labs/windmill/commit/d00de2640abfe6265f12ee4f237c9ee5ba3e00db))

## [1.246.7](https://github.com/windmill-labs/windmill/compare/v1.246.6...v1.246.7) (2024-01-17)


### Bug Fixes

* improve lockfile handling for cli ([8a6ea49](https://github.com/windmill-labs/windmill/commit/8a6ea496022ad70c658121e9694b634177dcc578))

## [1.246.6](https://github.com/windmill-labs/windmill/compare/v1.246.5...v1.246.6) (2024-01-17)


### Bug Fixes

* improve lockfile handling for cli ([0e9b649](https://github.com/windmill-labs/windmill/commit/0e9b649d03f79cdf6d5bece5dc0ae8072e22f5b0))

## [1.246.5](https://github.com/windmill-labs/windmill/compare/v1.246.4...v1.246.5) (2024-01-17)


### Bug Fixes

* improve app push for cli ([e6de809](https://github.com/windmill-labs/windmill/commit/e6de809ff29a9df54af5fef3f425072e24de49ec))
* lock file in metadata is now a string ([#3027](https://github.com/windmill-labs/windmill/issues/3027)) ([8752dcb](https://github.com/windmill-labs/windmill/commit/8752dcbb191279b44a2b86ee0ed45ab040465b96))

## [1.246.4](https://github.com/windmill-labs/windmill/compare/v1.246.3...v1.246.4) (2024-01-17)


### Bug Fixes

* **frontend:** Fix deno logo + add missing onDestroy ([#3025](https://github.com/windmill-labs/windmill/issues/3025)) ([b9de44c](https://github.com/windmill-labs/windmill/commit/b9de44c2b7daf964834af1d3598dbf40971e4a4f))
* make DisplayResult more resilient ([b42c84d](https://github.com/windmill-labs/windmill/commit/b42c84df40e234436e7fcf30a64317e654bed3ed))
* wmill app push &lt;path_to_app_file&gt; ([#3024](https://github.com/windmill-labs/windmill/issues/3024)) ([803962a](https://github.com/windmill-labs/windmill/commit/803962a943001ff5f2a58bf36f955cc16e92d2b1))

## [1.246.3](https://github.com/windmill-labs/windmill/compare/v1.246.2...v1.246.3) (2024-01-17)


### Bug Fixes

* oidc token generation endpoint GET -&gt; POST ([1f3e374](https://github.com/windmill-labs/windmill/commit/1f3e374b85581da463fda8727d9379d6711b7da8))

## [1.246.2](https://github.com/windmill-labs/windmill/compare/v1.246.1...v1.246.2) (2024-01-17)


### Bug Fixes

* oidc token generation endpoint GET -&gt; POST ([3119830](https://github.com/windmill-labs/windmill/commit/3119830062e9d4e30438950e208a2dde4eb12759))

## [1.246.1](https://github.com/windmill-labs/windmill/compare/v1.246.0...v1.246.1) (2024-01-17)


### Bug Fixes

* expose getIdToken in python-client ([4604ccd](https://github.com/windmill-labs/windmill/commit/4604ccde7dd656627605b8c55256cc3628235cab))
* expose getIdToken in typescript-client ([6568c9f](https://github.com/windmill-labs/windmill/commit/6568c9f93aa477c142bde6d989e4eec3a2440687))

## [1.246.0](https://github.com/windmill-labs/windmill/compare/v1.245.1...v1.246.0) (2024-01-17)


### Features

* OIDC support ([#3017](https://github.com/windmill-labs/windmill/issues/3017)) ([640ebcb](https://github.com/windmill-labs/windmill/commit/640ebcb146eae371abfa637a4f55fe1919aab013))

## [1.245.1](https://github.com/windmill-labs/windmill/compare/v1.245.0...v1.245.1) (2024-01-16)


### Bug Fixes

* CLI script generate-metadata creates a default file if none exist ([#3015](https://github.com/windmill-labs/windmill/issues/3015)) ([eb48e0a](https://github.com/windmill-labs/windmill/commit/eb48e0a1071d33e19425f2228e029453b3484458))

## [1.245.0](https://github.com/windmill-labs/windmill/compare/v1.244.4...v1.245.0) (2024-01-16)


### Features

* add script bootstrap and script generate-metadata CLI commands ([#3007](https://github.com/windmill-labs/windmill/issues/3007)) ([b9bee40](https://github.com/windmill-labs/windmill/commit/b9bee403f1ee922c776cf7a82aef0cdfc04c4c10))
* Browse s3 bucket content from workspace settings page ([#3013](https://github.com/windmill-labs/windmill/issues/3013)) ([1053979](https://github.com/windmill-labs/windmill/commit/10539790d20e01faf7aa992f44c89ab623a794c0))
* **frontend:** Add running runs on the script detail page ([#3005](https://github.com/windmill-labs/windmill/issues/3005)) ([c93932a](https://github.com/windmill-labs/windmill/commit/c93932a5b3b17cf3ddc7c86bf047343dc5da114d))
* **frontend:** add specific bun and deno icons ([#3006](https://github.com/windmill-labs/windmill/issues/3006)) ([e9ab3ce](https://github.com/windmill-labs/windmill/commit/e9ab3ceac901503c6d6e0af0954516af80e1f4a0))


### Bug Fixes

* s3 resource is accessed by backend with admin permissions ([#3011](https://github.com/windmill-labs/windmill/issues/3011)) ([9fffe4f](https://github.com/windmill-labs/windmill/commit/9fffe4f6f578665242612a596300e93b6cf2e6b6))

## [1.244.4](https://github.com/windmill-labs/windmill/compare/v1.244.2...v1.244.4) (2024-01-15)



### Features

* experimental nodejs support ([047ee10](https://github.com/windmill-labs/windmill/commit/047ee10246f8e1bb952d3b8cdf21612948ac9843))
* git sync branch name no contains the workspace ID and the type ([#3004](https://github.com/windmill-labs/windmill/issues/3004))


### Bug Fixes

* git sync branch name no contains the workspace ID and the type ([#3004](https://github.com/windmill-labs/windmill/issues/3004)) ([d845864](https://github.com/windmill-labs/windmill/commit/d845864872aff0057d6a3f5d9df2cc4a8c642be5))
* improve bun imports resolutions ([2b28854](https://github.com/windmill-labs/windmill/commit/2b288542bdfdba62b9182002db6c8c23cdd9869f))
* improve bun imports resolutions ([64e592b](https://github.com/windmill-labs/windmill/commit/64e592b874ef0414dcea2a3dd113d323cff0661e))

## [1.244.2](https://github.com/windmill-labs/windmill/compare/v1.244.1...v1.244.2) (2024-01-13)


### Bug Fixes

* improve favorite menu ([a1f93a4](https://github.com/windmill-labs/windmill/commit/a1f93a495e36e04b40c58bb8e33ab2336628ba8b))

## [1.244.1](https://github.com/windmill-labs/windmill/compare/v1.244.0...v1.244.1) (2024-01-13)


### Bug Fixes

* menu colors ([b84cd6d](https://github.com/windmill-labs/windmill/commit/b84cd6d52d723fea55a407d347afec1c669da03a))
* menu colors ([5201dcd](https://github.com/windmill-labs/windmill/commit/5201dcdd679690bf82b2606d85829ae9333e09ac))

## [1.244.0](https://github.com/windmill-labs/windmill/compare/v1.243.0...v1.244.0) (2024-01-13)


### Features

* **frontend:** Operator mode ([#2973](https://github.com/windmill-labs/windmill/issues/2973)) ([aaff17f](https://github.com/windmill-labs/windmill/commit/aaff17f813ae3f31cae58bb40b9c8118e772a2d8))


### Bug Fixes

* add ability to set secret variable from python ([0733dd1](https://github.com/windmill-labs/windmill/commit/0733dd118e463f73caa9155b97fc8d9b02759e06))
* improve oauth accounts permissions ([1621975](https://github.com/windmill-labs/windmill/commit/16219755a7fa8b9ff5a901c18842a1eceb68086a))
* improve table behavior when searching ([f0c4901](https://github.com/windmill-labs/windmill/commit/f0c4901c218cfc4564f9f2be0cacaf3d9997d822))

## [1.243.0](https://github.com/windmill-labs/windmill/compare/v1.242.0...v1.243.0) (2024-01-13)


### Features

* add support for bun install scopes ([d785def](https://github.com/windmill-labs/windmill/commit/d785deff3312b076e3ed9043924f514f8823e041))
* **frontend:** s3 file upload ([#2976](https://github.com/windmill-labs/windmill/issues/2976)) ([3c59fb8](https://github.com/windmill-labs/windmill/commit/3c59fb8b4d8a80077c2f352ccf1314fd32ff442c))
* GIt sync to multiple repo ([#2996](https://github.com/windmill-labs/windmill/issues/2996)) ([fda0e28](https://github.com/windmill-labs/windmill/commit/fda0e28db31fba9f71be6db7280696c3949fd7be))
* support dynamic args in tags ([aafd7d9](https://github.com/windmill-labs/windmill/commit/aafd7d90037bae20808e836d34c352ca6b357155))
* Tag override for scheduled scripts ([#2998](https://github.com/windmill-labs/windmill/issues/2998)) ([99484bd](https://github.com/windmill-labs/windmill/commit/99484bdc20bac966ca1d9d45ca4278133ea91b87))


### Bug Fixes

* allow for any extra bunfig config ([e200889](https://github.com/windmill-labs/windmill/commit/e200889cff6c5166ce6564d7f9d606c1613fe03b))

## [1.242.0](https://github.com/windmill-labs/windmill/compare/v1.241.0...v1.242.0) (2024-01-12)


### Features

* Instance group management page ([#2994](https://github.com/windmill-labs/windmill/issues/2994)) ([5f54f55](https://github.com/windmill-labs/windmill/commit/5f54f557903792ea6307b17f966c216cbd402709))


### Bug Fixes

* catch more agGrid errors ([01dbf54](https://github.com/windmill-labs/windmill/commit/01dbf548f712c2e669038e4eb704c470d2cb0426))

## [1.241.0](https://github.com/windmill-labs/windmill/compare/v1.240.0...v1.241.0) (2024-01-12)


### Features

* Instance group CRUD API ([#2992](https://github.com/windmill-labs/windmill/issues/2992)) ([5a157c4](https://github.com/windmill-labs/windmill/commit/5a157c415e7f075d3f57872a9adf7f5130632bb6))
* make dedicated workers for flows able to share runtime for the same scripts ([d59b89e](https://github.com/windmill-labs/windmill/commit/d59b89ec3cdf2285c0eac56d7678fc0b9a2c2a32))


### Bug Fixes

* add cache for flow as flow step ([28ac763](https://github.com/windmill-labs/windmill/commit/28ac7632cf767b1c0df30d93c841ad32bf891202))
* git sync now works for delete and rename ([#2988](https://github.com/windmill-labs/windmill/issues/2988)) ([cde574b](https://github.com/windmill-labs/windmill/commit/cde574b8910bbaf737e0d7e515c42fc796911c5e))

## [1.240.0](https://github.com/windmill-labs/windmill/compare/v1.239.0...v1.240.0) (2024-01-11)


### Features

* **cli:** introduce --stateful for CLI, in preparation for --raw to become the default ([39ecf80](https://github.com/windmill-labs/windmill/commit/39ecf8017ee88c4311cb2dd580b20cd59066612e))

## [1.239.0](https://github.com/windmill-labs/windmill/compare/v1.238.0...v1.239.0) (2024-01-11)


### Features

* add configurable ordering for script's generated UI ([717ccc9](https://github.com/windmill-labs/windmill/commit/717ccc94a06ee65a5676c7d9091faf892396657e))
* **frontend:** AG chart ([#2972](https://github.com/windmill-labs/windmill/issues/2972)) ([e3c1661](https://github.com/windmill-labs/windmill/commit/e3c166154da4251e21a5f6a6fcf5b2e101b558e1))


### Bug Fixes

* **cli:** improve .wmillignore handling of folders ([6996c90](https://github.com/windmill-labs/windmill/commit/6996c9083d9ae93922caf68e5658f18b49678630))
* handle Etc/Unknown timezone better ([8c97ef0](https://github.com/windmill-labs/windmill/commit/8c97ef0394925b49a04c253259783be301e8f4ee))
* only cache flow if it's a success ([589e683](https://github.com/windmill-labs/windmill/commit/589e683c27f15fcb48993f2f6e91523abef1794a))
* spelling error dtails -&gt; details ([#2986](https://github.com/windmill-labs/windmill/issues/2986)) ([308c4ce](https://github.com/windmill-labs/windmill/commit/308c4ceb475fef033ea87af5f772e1f35b4fb16d))

## [1.238.0](https://github.com/windmill-labs/windmill/compare/v1.237.0...v1.238.0) (2024-01-10)


### Features

* add ability to use secrets in pip requirements ([3517c7f](https://github.com/windmill-labs/windmill/commit/3517c7f28389a4ddf46a6bef4f2044aa94174050))
* add support for multiselect in python ([c8a793d](https://github.com/windmill-labs/windmill/commit/c8a793d35ea843fb75428182edc06cfe7105af3b))
* **frontend:** add shortcuts to audit logs ([#2975](https://github.com/windmill-labs/windmill/issues/2975)) ([4147d16](https://github.com/windmill-labs/windmill/commit/4147d1604a4c50ad0c667b413d64b44b357fb7e4))
* **frontend:** Display the index by which a node can be selected with the setTab function + add tooltip ([#2974](https://github.com/windmill-labs/windmill/issues/2974)) ([cffae36](https://github.com/windmill-labs/windmill/commit/cffae3633107d9359e04354ba01876ccaf98193a))
* S3 multipart upload accepts a custom S3 resource ([#2982](https://github.com/windmill-labs/windmill/issues/2982)) ([eea0d92](https://github.com/windmill-labs/windmill/commit/eea0d92dd11a3607fbb156e2bf3a3a001ab5e6a0))


### Bug Fixes

* main compile ([#2983](https://github.com/windmill-labs/windmill/issues/2983)) ([6d5c3f0](https://github.com/windmill-labs/windmill/commit/6d5c3f0f2252be15bf0169a42f5baa06d592911c))

## [1.237.0](https://github.com/windmill-labs/windmill/compare/v1.236.2...v1.237.0) (2024-01-08)


### Features

* make okta SSO configurable using custom domain ([4f01ee8](https://github.com/windmill-labs/windmill/commit/4f01ee89de743eab6ae323dea1ec4190ee23e0bf))


### Bug Fixes

* delete is captured in more fields in app ([d340fbc](https://github.com/windmill-labs/windmill/commit/d340fbc02fd9500249c1d23799c1bdbfe9602cb8))

## [1.236.2](https://github.com/windmill-labs/windmill/compare/v1.236.1...v1.236.2) (2024-01-08)


### Bug Fixes

* handle better non object result for app result loader ([269afe4](https://github.com/windmill-labs/windmill/commit/269afe41f7e02da8d3a364e649e5fef00847d24a))

## [1.236.1](https://github.com/windmill-labs/windmill/compare/v1.236.0...v1.236.1) (2024-01-08)


### Bug Fixes

* fix app initialization ([6b075e9](https://github.com/windmill-labs/windmill/commit/6b075e92ef4e312e6a4ce582dc7c4d88456b2f5d))

## [1.236.0](https://github.com/windmill-labs/windmill/compare/v1.235.1...v1.236.0) (2024-01-08)


### Features

* improve git sync and recompute dependents when common python script is redeployed ([#2967](https://github.com/windmill-labs/windmill/issues/2967)) ([9449381](https://github.com/windmill-labs/windmill/commit/94493812ac0030cc6bba468d8ee24a14661716a8))


### Bug Fixes

* reduce stack size needed for workflow transitions ([4384617](https://github.com/windmill-labs/windmill/commit/4384617f1ec4c27de88ef6c895d6ef060a79f6bd))

## [1.235.1](https://github.com/windmill-labs/windmill/compare/v1.235.0...v1.235.1) (2024-01-07)


### Bug Fixes

* fix resultjobloader ([2e02300](https://github.com/windmill-labs/windmill/commit/2e023003da8bea9efa86c0fbc52bf9723c73907d))

## [1.235.0](https://github.com/windmill-labs/windmill/compare/v1.234.1...v1.235.0) (2024-01-06)


### Features

* **frontend:** DB Explorer ([#2892](https://github.com/windmill-labs/windmill/issues/2892)) ([fffc533](https://github.com/windmill-labs/windmill/commit/fffc5338ce76fda9a68b19f03cefe199cc09a905))

## [1.234.1](https://github.com/windmill-labs/windmill/compare/v1.234.0...v1.234.1) (2024-01-05)


### Bug Fixes

* fix double view runs button ([9efcb53](https://github.com/windmill-labs/windmill/commit/9efcb53bcd903bf756c4beb92a5c7634e8e016ad))
* fix typo "Wourker" -&gt; "Worker" ([4385edc](https://github.com/windmill-labs/windmill/commit/4385edc6ffdb1b2e4544347928b4cf08faf1eaa1))

## [1.234.0](https://github.com/windmill-labs/windmill/compare/v1.233.0...v1.234.0) (2024-01-05)


### Features

* Detailed job memory footprint on job run page ([#2956](https://github.com/windmill-labs/windmill/issues/2956)) ([0710ce6](https://github.com/windmill-labs/windmill/commit/0710ce6ef25d8f2344ac1350e96aa906f16d1be1))
* git sync can now push commits to individual branches ([#2959](https://github.com/windmill-labs/windmill/issues/2959)) ([fb5cd6a](https://github.com/windmill-labs/windmill/commit/fb5cd6a4298c88e26481b76258bf5b09a11cf4b0))


### Bug Fixes

* Deleting workspace delete all workspace related values in DB ([#2961](https://github.com/windmill-labs/windmill/issues/2961)) ([c05d119](https://github.com/windmill-labs/windmill/commit/c05d119ae5dfb6f1bae88fc9d53b48a707cd2f15))

## [1.233.0](https://github.com/windmill-labs/windmill/compare/v1.232.0...v1.233.0) (2024-01-03)


### Features

* **frontend:** add the view runs buttons for operators ([#2932](https://github.com/windmill-labs/windmill/issues/2932)) ([821d2d7](https://github.com/windmill-labs/windmill/commit/821d2d7ea15e828dbb3683c85025dbc09c7e8dc7))
* Job caching now works with S3 files ([#2934](https://github.com/windmill-labs/windmill/issues/2934)) ([b44618f](https://github.com/windmill-labs/windmill/commit/b44618f35b6fe4558a5cd9fddec1e82216a2cf5e))
* PIP_LOCAL_DEPENDENCIES now accepts regex and is configurable from the UI ([#2940](https://github.com/windmill-labs/windmill/issues/2940)) ([e0140f1](https://github.com/windmill-labs/windmill/commit/e0140f175d371e239f315613a537820b9a6ad25f))


### Bug Fixes

* allow extraConfig for agGrid to set onGridReady ([74fc2d3](https://github.com/windmill-labs/windmill/commit/74fc2d3f4397d976fec326893f8196008cb8b332))
* decision tree fix wording ([#2941](https://github.com/windmill-labs/windmill/issues/2941)) ([0299c65](https://github.com/windmill-labs/windmill/commit/0299c656c368876581dc228c76513f1171ffd682))
* decision tree nits ([#2936](https://github.com/windmill-labs/windmill/issues/2936)) ([787017f](https://github.com/windmill-labs/windmill/commit/787017f53c8227e1246b09e412e92316977937ae))
* **frontend:** handle case where large json has a file and a filename ([#2951](https://github.com/windmill-labs/windmill/issues/2951)) ([2494171](https://github.com/windmill-labs/windmill/commit/2494171b9cb3d1857235b6558d276451d5550560))
* persist state for agGrid ([c085f5f](https://github.com/windmill-labs/windmill/commit/c085f5fa7ec598d0abf6aad98de0e5c7b1c92985))
* Tag all docker images with latest and main tags ([#2953](https://github.com/windmill-labs/windmill/issues/2953)) ([9655718](https://github.com/windmill-labs/windmill/commit/965571853ca253b130bcc4473190b6c9c98f44e1))

## [1.232.0](https://github.com/windmill-labs/windmill/compare/v1.231.0...v1.232.0) (2023-12-27)


### Features

* add summary to schedules ([606b03d](https://github.com/windmill-labs/windmill/commit/606b03d7613cf15ab9a60ef34a82657ff49ccdbd))
* no overlap for flows togglable on schedules ([4935528](https://github.com/windmill-labs/windmill/commit/49355280f6442c342a46f90b2fe2cb089b3f4eff))


### Bug Fixes

* fix file input ([263f41c](https://github.com/windmill-labs/windmill/commit/263f41cda3dba24bcaa2bbabfc3f5bc54c70f9cb))
* improve ata for bun ([b66fcbf](https://github.com/windmill-labs/windmill/commit/b66fcbf6261f69a57e6711fa8b9d1d499c1b47ed))
* use bun install in every case ([948b0da](https://github.com/windmill-labs/windmill/commit/948b0da59e6bcd3b6312373a1bbe52d541e54d56))

## [1.231.0](https://github.com/windmill-labs/windmill/compare/v1.230.0...v1.231.0) (2023-12-26)


### Features

* bun can read private npm package ([#2915](https://github.com/windmill-labs/windmill/issues/2915)) ([5fa5ab1](https://github.com/windmill-labs/windmill/commit/5fa5ab1514a2d20083d30809c30e50bfcb29e82f))
* **cli:** support skip args for push --raw ([fc07d58](https://github.com/windmill-labs/windmill/commit/fc07d5886893c8d493c3ca507e0c9d4a066622fc))


### Bug Fixes

* improve conditional wrapper ([4b65594](https://github.com/windmill-labs/windmill/commit/4b65594cbd138452c471a339aaa548c27fc0a4ce))

## [1.230.0](https://github.com/windmill-labs/windmill/compare/v1.229.0...v1.230.0) (2023-12-22)


### Features

* add button for auto-formatting ([642c037](https://github.com/windmill-labs/windmill/commit/642c037c1a87efd8d63558cc0ca5e98fb7b51ff7))
* move S3 file within bucket ([#2913](https://github.com/windmill-labs/windmill/issues/2913)) ([e40787c](https://github.com/windmill-labs/windmill/commit/e40787c616ec288ffc83450fac6f6f7f127ac008))
* quick search in flow support workspace scripts ([5698a0e](https://github.com/windmill-labs/windmill/commit/5698a0ebf2052fd51b687951595a543f0b6c0308))
* Retry for scheduled scripts ([#2893](https://github.com/windmill-labs/windmill/issues/2893)) ([5f60d46](https://github.com/windmill-labs/windmill/commit/5f60d468365c65dc48e247107487f64adf77b123))
* S3 delete file and upload new ([#2909](https://github.com/windmill-labs/windmill/issues/2909)) ([e27e887](https://github.com/windmill-labs/windmill/commit/e27e8874918af84d8afffb26abd5828b4c43ed54))


### Bug Fixes

* add relative path handling for bun ATA ([6b570a7](https://github.com/windmill-labs/windmill/commit/6b570a779fa7592c527cee84b0ca238872d86347))
* edit schedule from trigger page ([9667eb7](https://github.com/windmill-labs/windmill/commit/9667eb74f99b74621e0f00e3f380bbd34dd9654f))
* Failing jobs in dedicated worker mode are now marked as failing ([#2894](https://github.com/windmill-labs/windmill/issues/2894)) ([5f85b67](https://github.com/windmill-labs/windmill/commit/5f85b67dfcf063fd8a3c3f69f0e7605fc40e473d))
* fix multiplayer ([b5f1f99](https://github.com/windmill-labs/windmill/commit/b5f1f99daa30627f224f262653e459799fc0d6fe))
* **frontend:** Fix debug condition + decision tree + add missing date to job preview + prevent header node from being selected in the flow editor ([#2912](https://github.com/windmill-labs/windmill/issues/2912)) ([d7b777a](https://github.com/windmill-labs/windmill/commit/d7b777a8e9b4a2a4f0772f9b29851476f9405dae))
* improve javascript editors with semantic validation ([b3527f5](https://github.com/windmill-labs/windmill/commit/b3527f5164f9e487840773df57846394f118ebbb))
* pin deno windmill-client version to relase ([ea322d9](https://github.com/windmill-labs/windmill/commit/ea322d9c14cc92b54cfc39791e162f8f248e74ab))
* Python wrapper catches BaseException instead of Exception ([#2902](https://github.com/windmill-labs/windmill/issues/2902)) ([4c3c988](https://github.com/windmill-labs/windmill/commit/4c3c988f7bd35f290b9d051369029e790b1e5c5b))
* remove bad app type forcing ([#2906](https://github.com/windmill-labs/windmill/issues/2906)) ([187c34a](https://github.com/windmill-labs/windmill/commit/187c34a6835e3e748bc609fb4096f42bf3e09a73))
* Update S3 TS code snippets and Python SDK for Polars 0.20.X ([#2911](https://github.com/windmill-labs/windmill/issues/2911)) ([3cbdd89](https://github.com/windmill-labs/windmill/commit/3cbdd890cd2aaef1d983da0270f8c230c9b9285e))

## [1.229.0](https://github.com/windmill-labs/windmill/compare/v1.228.1...v1.229.0) (2023-12-21)


### Features

* **frontend:** add a quick button to open decision tree graph ([#2897](https://github.com/windmill-labs/windmill/issues/2897)) ([3f7c19f](https://github.com/windmill-labs/windmill/commit/3f7c19f267a9965f21cf8a08040a45e088d24918))


### Bug Fixes

* **frontend:** fix save draft when the app has no versions ([#2904](https://github.com/windmill-labs/windmill/issues/2904)) ([0170fcc](https://github.com/windmill-labs/windmill/commit/0170fcc338e0b59f8af1894d868f3819cad8aca3))
* Re-add TS SDK changes for S3 ([#2899](https://github.com/windmill-labs/windmill/issues/2899)) ([5fafd71](https://github.com/windmill-labs/windmill/commit/5fafd71135c5d4d1764e9f96984bc5f5e5810465))

## [1.228.1](https://github.com/windmill-labs/windmill/compare/v1.228.0...v1.228.1) (2023-12-20)


### Bug Fixes

* TS SDK not found upon install error ([#2895](https://github.com/windmill-labs/windmill/issues/2895)) ([46e9818](https://github.com/windmill-labs/windmill/commit/46e9818c001b040c462ffe0b6f8681a97113ff04))

## [1.228.0](https://github.com/windmill-labs/windmill/compare/v1.227.1...v1.228.0) (2023-12-20)


### Features

* Add Zitadel IDP ([#2888](https://github.com/windmill-labs/windmill/issues/2888)) ([afbea19](https://github.com/windmill-labs/windmill/commit/afbea19421572f2986bf8ab1e5817f1902c0a22c))
* bun automatic type acquisition in frontend directly ([#2884](https://github.com/windmill-labs/windmill/issues/2884)) ([d507ce4](https://github.com/windmill-labs/windmill/commit/d507ce449b04bf0be5ea29ec2431ad1eea405f13))
* not on latest app version warning ([#2889](https://github.com/windmill-labs/windmill/issues/2889)) ([e8b2cc8](https://github.com/windmill-labs/windmill/commit/e8b2cc8835881a7b2c0a273cb8e505644e9be001))
* S3 objects are now typed in Python and TS SDK ([#2878](https://github.com/windmill-labs/windmill/issues/2878)) ([2155a6c](https://github.com/windmill-labs/windmill/commit/2155a6c1334b79272742ba4626abd9572c9130d2))


### Bug Fixes

* Persistent script spinner ([#2879](https://github.com/windmill-labs/windmill/issues/2879)) ([5dd5809](https://github.com/windmill-labs/windmill/commit/5dd58094fde38f13476e79b539e670ee150306d7))
* Python imports from git repository ([#2886](https://github.com/windmill-labs/windmill/issues/2886)) ([292b230](https://github.com/windmill-labs/windmill/commit/292b2309a7b7093c85614ac9764620c9dce0e316))
* remove 0x00 from logs automatically from bashoutput ([59397e7](https://github.com/windmill-labs/windmill/commit/59397e7445f78cca2e602d8b45591a0bb123a734))

## [1.227.1](https://github.com/windmill-labs/windmill/compare/v1.227.0...v1.227.1) (2023-12-17)


### Bug Fixes

* improve on click behavior of tree view ([fa7261f](https://github.com/windmill-labs/windmill/commit/fa7261f273b517acd85a8c3f465cd8095cdf9f3c))

## [1.227.0](https://github.com/windmill-labs/windmill/compare/v1.226.1...v1.227.0) (2023-12-17)


### Features

* Add download button to S3 file picker ([#2857](https://github.com/windmill-labs/windmill/issues/2857)) ([f8c7a8e](https://github.com/windmill-labs/windmill/commit/f8c7a8edf87226e020cec5e602e2dcb31f87d95e))
* add filter for home screen to filter u/* ([7fac60c](https://github.com/windmill-labs/windmill/commit/7fac60c37f638c0a717d2062f7e79db655506c4c))
* Add Kanidm SSO support ([#2871](https://github.com/windmill-labs/windmill/issues/2871)) ([40013cd](https://github.com/windmill-labs/windmill/commit/40013cd6cb00ebaf83eb75ec3f6907a727ba8f63))
* add overridable tag directly from run page ([9b25faa](https://github.com/windmill-labs/windmill/commit/9b25faaa95a4f659aa2c72b0b8ed46acee31a691))
* Cancel persistent script runs from drawer ([#2847](https://github.com/windmill-labs/windmill/issues/2847)) ([9b0b919](https://github.com/windmill-labs/windmill/commit/9b0b9197d717c6dad7b1f5e934158e7526455a47))
* **frontend:** Add submitted text prop ([#2864](https://github.com/windmill-labs/windmill/issues/2864)) ([864e6a5](https://github.com/windmill-labs/windmill/commit/864e6a5b9432e32ed1463972f7f980ae0b52745b))
* **frontend:** Decision tree ([#2812](https://github.com/windmill-labs/windmill/issues/2812)) ([c80544e](https://github.com/windmill-labs/windmill/commit/c80544e8e4f7b635847603e6e9ef9e661ce20c89))


### Bug Fixes

* **cli:** improve flow sync for windows ([65e18ab](https://github.com/windmill-labs/windmill/commit/65e18abe7d17334391b9326cd9968e64fa9f6586))
* **cli:** improve flow sync for windows ([0118136](https://github.com/windmill-labs/windmill/commit/011813654da596a8c7235408f1cd77e9abc63255))
* **cli:** remove is_expired from variables ([aa44a88](https://github.com/windmill-labs/windmill/commit/aa44a88960123204405e03efea87f0ba6bbe15ec))
* **frontend:** fix tutorial typos ([#2852](https://github.com/windmill-labs/windmill/issues/2852)) ([28b8c4b](https://github.com/windmill-labs/windmill/commit/28b8c4be833efd0b4c56584945b823c4e4fafa7a))
* handle empty result ([#2862](https://github.com/windmill-labs/windmill/issues/2862)) ([e04d8b0](https://github.com/windmill-labs/windmill/commit/e04d8b0d51ed57bd56b28c178dc6668be65fafbd))
* postgres ssl mode ([#2861](https://github.com/windmill-labs/windmill/issues/2861)) ([6c809b8](https://github.com/windmill-labs/windmill/commit/6c809b86300cd4cb461556eb570620f70ca0e26d))
* powershell nsjail ([#2856](https://github.com/windmill-labs/windmill/issues/2856)) ([fdd9374](https://github.com/windmill-labs/windmill/commit/fdd9374aa1cc4d21e9543771e9d1ad256e083461))
* Stopping perpetual scripts ([#2860](https://github.com/windmill-labs/windmill/issues/2860)) ([a84b432](https://github.com/windmill-labs/windmill/commit/a84b4324d2b39eea42fc8c0ed7c9df2a6c2207aa))

## [1.226.1](https://github.com/windmill-labs/windmill/compare/v1.226.0...v1.226.1) (2023-12-14)


### Bug Fixes

* cli uses await for every push call ([996bf64](https://github.com/windmill-labs/windmill/commit/996bf64393e221f3749e17c3c3755f45a73b616f))
* **cli:** check for existing resource even in raw mode ([f039008](https://github.com/windmill-labs/windmill/commit/f039008dc0d5e8f309148c1cab2998e03b4298c1))
* getVariable can now return the encrypted value ([087c516](https://github.com/windmill-labs/windmill/commit/087c5167afc90dacb4264bde73834d60a6ae2063))

## [1.226.0](https://github.com/windmill-labs/windmill/compare/v1.225.1...v1.226.0) (2023-12-14)


### Features

* ee-only stats ([#2849](https://github.com/windmill-labs/windmill/issues/2849)) ([3c4e41c](https://github.com/windmill-labs/windmill/commit/3c4e41c9975956339682ef35c298d294c55304f0))


### Bug Fixes

* fix s3 test connection ([17bb7d3](https://github.com/windmill-labs/windmill/commit/17bb7d3991d48abe08ab42b9381285112f2440f1))
* Handle s3object in Deno and Bun ([#2851](https://github.com/windmill-labs/windmill/issues/2851)) ([0f913d0](https://github.com/windmill-labs/windmill/commit/0f913d0df915a8189a036f06bd6684d9ed5ecadc))

## [1.225.1](https://github.com/windmill-labs/windmill/compare/v1.225.0...v1.225.1) (2023-12-13)


### Bug Fixes

* fix setting base_url from empty initial value ([63740ad](https://github.com/windmill-labs/windmill/commit/63740adec8b075069cbee942ae03c1c2cbec5de0))
* pwsh modules nsjail ([#2845](https://github.com/windmill-labs/windmill/issues/2845)) ([b939785](https://github.com/windmill-labs/windmill/commit/b9397856a8c2219d5315eb441b2b279a1dd8dab5))

## [1.225.0](https://github.com/windmill-labs/windmill/compare/v1.224.1...v1.225.0) (2023-12-13)


### Features

* Add default timeout for instance and custom timeout for scripts ([#2811](https://github.com/windmill-labs/windmill/issues/2811)) ([26670f6](https://github.com/windmill-labs/windmill/commit/26670f62e5e9f7bd50895fae9dd98bc75b61bb44))
* Add jitter to exponential backoff ([#2842](https://github.com/windmill-labs/windmill/issues/2842)) ([dd1032f](https://github.com/windmill-labs/windmill/commit/dd1032fbc3b589f00095c2f9523e1e3f7fca80d5))
* EnvVar allow list is configurable from the UI ([#2829](https://github.com/windmill-labs/windmill/issues/2829)) ([350c8e8](https://github.com/windmill-labs/windmill/commit/350c8e843f8facb154bb6b8223744e1c5ba65a28))
* oauth flow openai key ([#2833](https://github.com/windmill-labs/windmill/issues/2833)) ([4c7d64b](https://github.com/windmill-labs/windmill/commit/4c7d64bc5fdfeb33c63524ddd61173c655ab28e3))
* powershell deps ([#2820](https://github.com/windmill-labs/windmill/issues/2820)) ([505b417](https://github.com/windmill-labs/windmill/commit/505b417f3b07fd5b27256a6a01001aadba7c1f84))
* Resource type description is now displayed when creating a new resource ([#2834](https://github.com/windmill-labs/windmill/issues/2834)) ([c06a56a](https://github.com/windmill-labs/windmill/commit/c06a56ab785f072ff5525e685188d97c71f71da5))


### Bug Fixes

* improve nested flow status viewer ([18e07a7](https://github.com/windmill-labs/windmill/commit/18e07a781d453b867f325ae6086abe406767fd11))
* openai cache lock ([#2841](https://github.com/windmill-labs/windmill/issues/2841)) ([107bfa7](https://github.com/windmill-labs/windmill/commit/107bfa72520bca2df2b92adbbbb7e7f652b85a68))
* Simplify worker env var input ([#2838](https://github.com/windmill-labs/windmill/issues/2838)) ([b401984](https://github.com/windmill-labs/windmill/commit/b401984514f095a60bce913d245160dd8746a9a5))

## [1.224.1](https://github.com/windmill-labs/windmill/compare/v1.224.0...v1.224.1) (2023-12-11)


### Bug Fixes

* **frontend:** add truncate to DrawerContent ([#2830](https://github.com/windmill-labs/windmill/issues/2830)) ([69abe27](https://github.com/windmill-labs/windmill/commit/69abe27662c801d9ecbe2d31a51142e84d7dcfa2))
* sql types completions when monaco initialized ([#2831](https://github.com/windmill-labs/windmill/issues/2831)) ([64a4528](https://github.com/windmill-labs/windmill/commit/64a452826f3e00aaea6b5205c908f117327398c4))

## [1.224.0](https://github.com/windmill-labs/windmill/compare/v1.223.1...v1.224.0) (2023-12-10)


### Features

* add authelia sso support ([#2824](https://github.com/windmill-labs/windmill/issues/2824)) ([6fefdb5](https://github.com/windmill-labs/windmill/commit/6fefdb5438d0fd7546759d93b5f4465731a4cac5))
* embedded server only get killed after last job of worker ([4f4ca26](https://github.com/windmill-labs/windmill/commit/4f4ca264aef3e1d3ee4748593f0ebcb425d333ef))


### Bug Fixes

* make cleanup_module backcompatible ([58c4c0b](https://github.com/windmill-labs/windmill/commit/58c4c0b5c2b009ff56e3eb6bd19e766dbff4cf54))

## [1.223.1](https://github.com/windmill-labs/windmill/compare/v1.223.0...v1.223.1) (2023-12-08)


### Bug Fixes

* improve aggrid selectedRow output + make first selectedRow configurable ([651d998](https://github.com/windmill-labs/windmill/commit/651d99824d1932a48fdb9ffd5dff944fa2c2c8aa))

## [1.223.0](https://github.com/windmill-labs/windmill/compare/v1.222.0...v1.223.0) (2023-12-08)


### Features

* Add a git sync test button ([#2817](https://github.com/windmill-labs/windmill/issues/2817)) ([535202c](https://github.com/windmill-labs/windmill/commit/535202c1fe1e3a90b1aff2dad6632f615ca8aef8))
* add mysql datetime ([#2808](https://github.com/windmill-labs/windmill/issues/2808)) ([8896a8c](https://github.com/windmill-labs/windmill/commit/8896a8cacffb5da5575203a5fc9aa6863b303fde))
* Add possibility to delete flow step results when the flow is complete ([#2806](https://github.com/windmill-labs/windmill/issues/2806)) ([b65657d](https://github.com/windmill-labs/windmill/commit/b65657d0f8db21f87a90bdcd6ce1122a8bb209ce))
* no worker with tag warning ([#2803](https://github.com/windmill-labs/windmill/issues/2803)) ([e7141c1](https://github.com/windmill-labs/windmill/commit/e7141c147d9ff4728399ae0703ca636c776cb264))
* sql types autocomplete ([#2810](https://github.com/windmill-labs/windmill/issues/2810)) ([a9b5d6b](https://github.com/windmill-labs/windmill/commit/a9b5d6bebb00c96cc329b4178816c73db5cbe3f0))


### Bug Fixes

* conditional component also for toggles ([d26730a](https://github.com/windmill-labs/windmill/commit/d26730ad3419df5c5d75328e7f441c701b2c0c68))
* db schema autocomplete ([#2809](https://github.com/windmill-labs/windmill/issues/2809)) ([e250897](https://github.com/windmill-labs/windmill/commit/e2508972378f455b7af7984f2c33894dadd4f7bc))
* deno chromium support when nsjail enabled ([#2815](https://github.com/windmill-labs/windmill/issues/2815)) ([2964569](https://github.com/windmill-labs/windmill/commit/296456996293d861c7cab2d10e01e3b6443e421c))
* Fix test_complex_flow_restart ([#2813](https://github.com/windmill-labs/windmill/issues/2813)) ([b089449](https://github.com/windmill-labs/windmill/commit/b0894492f1e3c06880c0e1875c8c8886e31a165f))
* improve sql default arg parsing + auto invite ([033000f](https://github.com/windmill-labs/windmill/commit/033000fed36d6172cf3b9a83ff449f5ba051fa08))
* support interval in pg ([009a83d](https://github.com/windmill-labs/windmill/commit/009a83dd9bbc115b94978d2cb3113cbe10acd05d))

## [1.222.0](https://github.com/windmill-labs/windmill/compare/v1.221.0...v1.222.0) (2023-12-07)


### Features

* conditional fields on forms ([b10df30](https://github.com/windmill-labs/windmill/commit/b10df302455202940348cd20c2e733734fb7d027))
* mysql named params ([#2805](https://github.com/windmill-labs/windmill/issues/2805)) ([c4a1054](https://github.com/windmill-labs/windmill/commit/c4a1054a5a4ab3fd700de842a1f46d5886632b11))


### Bug Fixes

* add saml feature flag ([#2802](https://github.com/windmill-labs/windmill/issues/2802)) ([da15a56](https://github.com/windmill-labs/windmill/commit/da15a56a07288d4164c88289dd559389735db719))
* app reports screenshot refresh ([#2800](https://github.com/windmill-labs/windmill/issues/2800)) ([f7f7130](https://github.com/windmill-labs/windmill/commit/f7f71302f7d0fcac7468d506ebaceb55332ec1ed))
* fix extraOptions for agGrid ([f7d7746](https://github.com/windmill-labs/windmill/commit/f7d7746327411da06cede4e0ff06222d318d01bc))
* fix extraOptions for agGrid ([2f7fb9b](https://github.com/windmill-labs/windmill/commit/2f7fb9bfd490932a2f0bc01f02df678668487f8c))
* **frontend:** Fix chartjs options merge ([#2804](https://github.com/windmill-labs/windmill/issues/2804)) ([e11b257](https://github.com/windmill-labs/windmill/commit/e11b257b2d761b2b26160d61f8e113dbe47e9efe))

## [1.221.0](https://github.com/windmill-labs/windmill/compare/v1.220.0...v1.221.0) (2023-12-06)


### Features

* limit pro plans ([#2794](https://github.com/windmill-labs/windmill/issues/2794)) ([c58190e](https://github.com/windmill-labs/windmill/commit/c58190ef788feaae9bac9848e8cfa59d10f51cab))


### Bug Fixes

* Allow run git for Deno ([#2789](https://github.com/windmill-labs/windmill/issues/2789)) ([8ad8d20](https://github.com/windmill-labs/windmill/commit/8ad8d20136f3151ca28784ba3513af562fcb5c9b))
* Allow run git for Deno ([#2790](https://github.com/windmill-labs/windmill/issues/2790)) ([d822beb](https://github.com/windmill-labs/windmill/commit/d822beba63f08e4e1e1dac4aff56353276558f02))
* app reports nits ([#2788](https://github.com/windmill-labs/windmill/issues/2788)) ([0b13f2d](https://github.com/windmill-labs/windmill/commit/0b13f2d782d5626d64b5efe39232ec8da268cd27))
* app reports ts ([#2798](https://github.com/windmill-labs/windmill/issues/2798)) ([4f6a116](https://github.com/windmill-labs/windmill/commit/4f6a116e462d198cb81260749fdd8d57bc0ef03a))
* Deno allow run git ([#2791](https://github.com/windmill-labs/windmill/issues/2791)) ([256ee64](https://github.com/windmill-labs/windmill/commit/256ee646b49bca091ee6e7a0fea37f7d98c9e378))
* **frontend:** fix storage key typo ([#2796](https://github.com/windmill-labs/windmill/issues/2796)) ([e4da819](https://github.com/windmill-labs/windmill/commit/e4da819cda12e0e1f27cabc594fe5af5e2d9966c))
* **frontend:** use correct id for selected for loop iteration ([#2787](https://github.com/windmill-labs/windmill/issues/2787)) ([90668fb](https://github.com/windmill-labs/windmill/commit/90668fb0e5b4ac9f09a0b1e57fcb10b4b4208d2e))
* improve conditionalwrapper selection ([b5f5355](https://github.com/windmill-labs/windmill/commit/b5f53556a9afb3becbaaab034eeb8fdf4b1178a1))
* linked variable already being a variable ([9c4f1e0](https://github.com/windmill-labs/windmill/commit/9c4f1e0333743b35c237b4d8fe49757c8ae122e7))
* smtp doesn't require username/password ([abb50fa](https://github.com/windmill-labs/windmill/commit/abb50fac93a615d0ee18e9776c1fb84829fa2464))

## [1.220.0](https://github.com/windmill-labs/windmill/compare/v1.219.1...v1.220.0) (2023-12-05)


### Features

* Add workspace settings to sync scripts/flows/apps to git repo on deployment ([#2740](https://github.com/windmill-labs/windmill/issues/2740)) ([194ee83](https://github.com/windmill-labs/windmill/commit/194ee830cc8d8cc0ea69c877b428966f3f20bdb5))
* app reports v2 ([#2778](https://github.com/windmill-labs/windmill/issues/2778)) ([f36a510](https://github.com/windmill-labs/windmill/commit/f36a510025d87fc20e15f66e041390cc701c65c8))
* CLI now accepts message arg for sync push ([#2784](https://github.com/windmill-labs/windmill/issues/2784)) ([d9458e9](https://github.com/windmill-labs/windmill/commit/d9458e96ebc26160e9d5f909574bd1cf1d288d73))
* Custom content type for script and flow results ([#2767](https://github.com/windmill-labs/windmill/issues/2767)) ([6c0f921](https://github.com/windmill-labs/windmill/commit/6c0f921b3ab3acac8890ceb1fa0565bf082de0d4))
* **frontend:** add before and after icons for text input components ([#2746](https://github.com/windmill-labs/windmill/issues/2746)) ([a3f1111](https://github.com/windmill-labs/windmill/commit/a3f1111ca7715ffaa3a615786adee20e00870b30))
* History commit message can be updated from the UI ([#2777](https://github.com/windmill-labs/windmill/issues/2777)) ([874b7a5](https://github.com/windmill-labs/windmill/commit/874b7a50f72a6505490a02c82786ea5f5c8261e0))
* perpetual scripts ([bea0da3](https://github.com/windmill-labs/windmill/commit/bea0da3d6ff81ace956d6a1e7fb643e449927b05))
* result preview in app editor ([#2761](https://github.com/windmill-labs/windmill/issues/2761)) ([51d648d](https://github.com/windmill-labs/windmill/commit/51d648d450c578580f6d9a9630ff91920060cc27))
* save inline scripts of apps to workspace + search by path ([f50cef0](https://github.com/windmill-labs/windmill/commit/f50cef03670e50b8d7a6bb4719ae636f2714f22f))
* Various minor improvements for S3 DX ([#2760](https://github.com/windmill-labs/windmill/issues/2760)) ([5cb3c34](https://github.com/windmill-labs/windmill/commit/5cb3c34e440be878687217fdd95d735f053c7839))


### Bug Fixes

* add access-control-allow-origin to static assets ([60cd14f](https://github.com/windmill-labs/windmill/commit/60cd14ff59a80f671838a5f9bfa21effc309cb54))
* add run_flow_sync to python client + few nits ([042504f](https://github.com/windmill-labs/windmill/commit/042504f9052963c882ac346d2026c518c6d94f0f))
* Array of S3 files now shows links to the S3 explorer ([#2783](https://github.com/windmill-labs/windmill/issues/2783)) ([20b0500](https://github.com/windmill-labs/windmill/commit/20b0500a2824be1e715d4301e05c6081a5575dbe))
* deployment_metadata table app_version is a nullable BIGINT ([#2769](https://github.com/windmill-labs/windmill/issues/2769)) ([df86538](https://github.com/windmill-labs/windmill/commit/df865380bceb8663defb322a88f58f7c3f1a263c))
* do not prorate for team plan checkouts mornings of firsts ([#2751](https://github.com/windmill-labs/windmill/issues/2751)) ([8fc0afc](https://github.com/windmill-labs/windmill/commit/8fc0afce714774647f86e1aca8eeec4b7d4c13fc))
* fix early return sync on existing flows ([60c4860](https://github.com/windmill-labs/windmill/commit/60c486023310ef85d1cb171ebca792be8d7917e8))
* **frontend:** Fix dark mode observer, removed code duplication ([#2770](https://github.com/windmill-labs/windmill/issues/2770)) ([a6d1e7c](https://github.com/windmill-labs/windmill/commit/a6d1e7c79bb16d2c02d53f9ede56451e064956e0))
* **frontend:** fix fileinput drag and drop check ([#2781](https://github.com/windmill-labs/windmill/issues/2781)) ([0c3d4a6](https://github.com/windmill-labs/windmill/commit/0c3d4a64c4a8ad823646f9ce27de7e58ccc002e8))
* **frontend:** fix resource search ([#2748](https://github.com/windmill-labs/windmill/issues/2748)) ([6eaec47](https://github.com/windmill-labs/windmill/commit/6eaec471629d6c60e6c3123372fa3569bc5471c7))
* **frontend:** fix sign in with Google button ([#2756](https://github.com/windmill-labs/windmill/issues/2756)) ([8ca04e4](https://github.com/windmill-labs/windmill/commit/8ca04e49b061675f820e4aeaad6196403eabe135))
* **frontend:** fix tutorials contols ([#2745](https://github.com/windmill-labs/windmill/issues/2745)) ([85805bd](https://github.com/windmill-labs/windmill/commit/85805bdf8265f4fe14a583b43b786257ffccc4dc))
* **frontend:** infer args if the schema is not an object ([#2782](https://github.com/windmill-labs/windmill/issues/2782)) ([b67d005](https://github.com/windmill-labs/windmill/commit/b67d00540b7adfac3550299d50fc9a620b5a6e2e))
* improve azure openai readme ([#2754](https://github.com/windmill-labs/windmill/issues/2754)) ([d47d4cc](https://github.com/windmill-labs/windmill/commit/d47d4ccf85747a14e0498be871ead08371392ff5))
* show bg runnables currently running ([a122b6c](https://github.com/windmill-labs/windmill/commit/a122b6cab664e0b8b3d7223b9fd70569a2fe0991))
* string or enum ([#2752](https://github.com/windmill-labs/windmill/issues/2752)) ([583dae6](https://github.com/windmill-labs/windmill/commit/583dae6a72ee6be70dee5a356e1b4172db9a7e2d))
* string results with custom content types are not quoted ([#2768](https://github.com/windmill-labs/windmill/issues/2768)) ([1cad51b](https://github.com/windmill-labs/windmill/commit/1cad51b5941a3bd1892c935581373bb7375da45a))
* strip prefix of s3 resource ([#2780](https://github.com/windmill-labs/windmill/issues/2780)) ([f3a0c81](https://github.com/windmill-labs/windmill/commit/f3a0c81639662a7da65af49e4032dfc573dd93eb))

## [1.219.1](https://github.com/windmill-labs/windmill/compare/v1.219.0...v1.219.1) (2023-12-01)


### Bug Fixes

* fix editorContext in preview ([cb42f10](https://github.com/windmill-labs/windmill/commit/cb42f10dc605f00ba70fadc61069aba98af8ec88))
* maps mapRegion update on move ([62de305](https://github.com/windmill-labs/windmill/commit/62de305908ba7e6ac75aa5e8ffba822522b08345))

## [1.219.0](https://github.com/windmill-labs/windmill/compare/v1.218.0...v1.219.0) (2023-12-01)


### Features

* eval preview for apps ([d4374a0](https://github.com/windmill-labs/windmill/commit/d4374a0103d2244b31b5cec7649dbfb96b2af1b4))


### Bug Fixes

* **frontend:** rename menu component ([#2738](https://github.com/windmill-labs/windmill/issues/2738)) ([f816ad0](https://github.com/windmill-labs/windmill/commit/f816ad01d3e39917714abadcf3833a4d8619e4f3))
* improve quickstyle to be in static ccomponents only ([4774e03](https://github.com/windmill-labs/windmill/commit/4774e03be5663017bccbe95f026cc39d4c43f536))
* improve stat card ([fc5f054](https://github.com/windmill-labs/windmill/commit/fc5f054b94fa58ee889d10d06e486b6e8c4f885e))
* limit log pull from queued jobs ([592d783](https://github.com/windmill-labs/windmill/commit/592d7839d183843b425521a319b802fb8cac3f21))
* support INET in pg ([007d5be](https://github.com/windmill-labs/windmill/commit/007d5be23e038fbf82907c3b335b3d747791295d))

## [1.218.0](https://github.com/windmill-labs/windmill/compare/v1.217.0...v1.218.0) (2023-11-30)


### Features

* **frontend:** add menu component ([#2721](https://github.com/windmill-labs/windmill/issues/2721)) ([0665692](https://github.com/windmill-labs/windmill/commit/06656924ae8173e906a243aab2af658a1689af01))
* **frontend:** resource rework ([#2725](https://github.com/windmill-labs/windmill/issues/2725)) ([ec25856](https://github.com/windmill-labs/windmill/commit/ec25856b7c8a7d33a50d1beb5f3e99c7b912e1ca))


### Bug Fixes

* make REST language support URLSearchParams, headers, FormData ([830dec0](https://github.com/windmill-labs/windmill/commit/830dec0f90e0189b4dea4de8c44a03e437acf440))
* Pull patched version of gosyn ([#2734](https://github.com/windmill-labs/windmill/issues/2734)) ([0bf3685](https://github.com/windmill-labs/windmill/commit/0bf3685cbdecae0a8e7a24a5198ae2fed98fe340))
* s3 resource openapi spec ([#2730](https://github.com/windmill-labs/windmill/issues/2730)) ([16d10ae](https://github.com/windmill-labs/windmill/commit/16d10aedf79cddf7cdc15d55c42c72f4948b3ee2))
* update deployed even when draft only ([#2694](https://github.com/windmill-labs/windmill/issues/2694)) ([1f99fcd](https://github.com/windmill-labs/windmill/commit/1f99fcd3af21208676aa90015559359740b0534a))
* Using latest gosyn ([#2737](https://github.com/windmill-labs/windmill/issues/2737)) ([920cc9a](https://github.com/windmill-labs/windmill/commit/920cc9a576db0112ffd9572480c3d0a8aa08055b))
* Workspace error handler creation was not adding the slack resource ([#2733](https://github.com/windmill-labs/windmill/issues/2733)) ([70c504e](https://github.com/windmill-labs/windmill/commit/70c504edfaf7c114c91af2b3ca39dfee073f04f2))

## [1.217.0](https://github.com/windmill-labs/windmill/compare/v1.216.0...v1.217.0) (2023-11-29)


### Features

* add support for raw query args ([8275602](https://github.com/windmill-labs/windmill/commit/82756023728fe392fee93a7bba0567ce582aad8c))
* **frontend:** add "hide schedules" filter ([#2710](https://github.com/windmill-labs/windmill/issues/2710)) ([46e0f91](https://github.com/windmill-labs/windmill/commit/46e0f913878c9f688594cfe1c9184438e4facf49))
* **frontend:** Added tailwind classes auto-complete ([#2712](https://github.com/windmill-labs/windmill/issues/2712)) ([2d3ce8a](https://github.com/windmill-labs/windmill/commit/2d3ce8a49c952accb4f1ceb0e6ea608df539f179))
* **frontend:** Stat card improvement ([#2709](https://github.com/windmill-labs/windmill/issues/2709)) ([89abb68](https://github.com/windmill-labs/windmill/commit/89abb68f635d062778485c15d88fd6272ebd40a2))
* scheduled app reports ([#2714](https://github.com/windmill-labs/windmill/issues/2714)) ([3789b34](https://github.com/windmill-labs/windmill/commit/3789b34dae72ff57bb9e0bb7e93439446c78095d))


### Bug Fixes

* ai fix popup placement + update edit/fix prompt to return complete code ([#2715](https://github.com/windmill-labs/windmill/issues/2715)) ([76a387f](https://github.com/windmill-labs/windmill/commit/76a387f4a181d21afad0138c07f947aa292978cc))
* better error for moved openai resource ([#2724](https://github.com/windmill-labs/windmill/issues/2724)) ([15b2c9f](https://github.com/windmill-labs/windmill/commit/15b2c9f171122ab074f8430a785f8ed577921fcc))
* Deno can talk to private NPM registries behind HTTPS ([#2713](https://github.com/windmill-labs/windmill/issues/2713)) ([726866b](https://github.com/windmill-labs/windmill/commit/726866b410863ca9583a8145ebd9f4c4557cef08))
* Error handler now supports flows ([#2707](https://github.com/windmill-labs/windmill/issues/2707)) ([36e46e2](https://github.com/windmill-labs/windmill/commit/36e46e2e47e1e949e1e235fa4ec0365b46d80de1))
* **frontend:** fix separator z-index ([#2720](https://github.com/windmill-labs/windmill/issues/2720)) ([9525ab7](https://github.com/windmill-labs/windmill/commit/9525ab7bbab9f820d1f6fee44f9a2e9ad19d1b54))
* **frontend:** Fix table initial ordering ([#2727](https://github.com/windmill-labs/windmill/issues/2727)) ([1a1d1db](https://github.com/windmill-labs/windmill/commit/1a1d1db96fd069434291f4b5f37ae1d0e21c5e44))
* **frontend:** Improve error message + fix overflow when file name is too long ([#2691](https://github.com/windmill-labs/windmill/issues/2691)) ([c990f85](https://github.com/windmill-labs/windmill/commit/c990f856aaf821899cc54db20a653af396ceae7f))
* generate cargo lock file ([#2722](https://github.com/windmill-labs/windmill/issues/2722)) ([bd31979](https://github.com/windmill-labs/windmill/commit/bd31979a62a30071e68385b5eb5dc28f37ea8fb5))
* improve autocomplete reactivity ([c3eaf0b](https://github.com/windmill-labs/windmill/commit/c3eaf0bf4aab531b52a1b3ac276c6840b0925786))
* make dedicated workers able to redeploy automatically ([e6d67f4](https://github.com/windmill-labs/windmill/commit/e6d67f4e5994360e7ae83c1303b31514b6062d1d))
* minor fixes to private NPM and python registries to get everything working ([#2728](https://github.com/windmill-labs/windmill/issues/2728)) ([3d6fb15](https://github.com/windmill-labs/windmill/commit/3d6fb15a90f4d5aa18a6d6158760a380295e3d9e))
* only list session and permanent token in user settings ([c8046af](https://github.com/windmill-labs/windmill/commit/c8046af9d01ade6891ea97e56974bb742bbf3e6e))
* trim .bun.ts for local imports ([d2b3026](https://github.com/windmill-labs/windmill/commit/d2b3026032d288e1e5de1f37979d92d02094b3e4))

## [1.216.0](https://github.com/windmill-labs/windmill/compare/v1.215.0...v1.216.0) (2023-11-26)


### Features

* add early return for flows ([dbaef0a](https://github.com/windmill-labs/windmill/commit/dbaef0aa5f22c55b691f6205ae053c155a0e025c))
* fill pg resource from string ([#2699](https://github.com/windmill-labs/windmill/issues/2699)) ([f1cabb4](https://github.com/windmill-labs/windmill/commit/f1cabb40f6cbec05b105563dad58da048d9187ec))
* **frontend:** add currency format + add enum autocomplete + fix run… ([#2670](https://github.com/windmill-labs/windmill/issues/2670)) ([0c0f43d](https://github.com/windmill-labs/windmill/commit/0c0f43dd3ac0541d889046c0c6df8dbf78650b9c))
* **frontend:** add stat card ([#2687](https://github.com/windmill-labs/windmill/issues/2687)) ([81ef24b](https://github.com/windmill-labs/windmill/commit/81ef24b3cec18f86e407b343b4d72ac566d268e1))
* **python:** Update return type for 'get_resource' function ([#2695](https://github.com/windmill-labs/windmill/issues/2695)) ([603e7ff](https://github.com/windmill-labs/windmill/commit/603e7ff67f5f68d291db37173b89325d5de911a1))


### Bug Fixes

* expand enum type narrowing to forms ([94e9b80](https://github.com/windmill-labs/windmill/commit/94e9b80e1993a217415118bf48b8575cd8363746))
* **frontend:** correctly handle when result is undefined ([#2693](https://github.com/windmill-labs/windmill/issues/2693)) ([f4aa768](https://github.com/windmill-labs/windmill/commit/f4aa76897ea0f261c1e47976f517058601c479f9))
* improve multiselect from form ([5fa653d](https://github.com/windmill-labs/windmill/commit/5fa653d154c6e8697119796a025bdc380f68de9a))
* improve resource pages ([16be230](https://github.com/windmill-labs/windmill/commit/16be2300ff66cad8b740856a6435d1c1e53bce99))
* lighten monaco editor workers ([#2690](https://github.com/windmill-labs/windmill/issues/2690)) ([4c42836](https://github.com/windmill-labs/windmill/commit/4c42836cfcad63da023a658213f03b45c925efb2))
* Priority tags FE buggy when missing from config ([#2702](https://github.com/windmill-labs/windmill/issues/2702)) ([e8b1f22](https://github.com/windmill-labs/windmill/commit/e8b1f220dd299922a32b04451756f2b7fa735d5d))
* S3 bucket browser small improvements and fixes ([#2700](https://github.com/windmill-labs/windmill/issues/2700)) ([55e34d8](https://github.com/windmill-labs/windmill/commit/55e34d8cdd64362615f2c1bc2698c0538c287784))

## [1.215.0](https://github.com/windmill-labs/windmill/compare/v1.214.1...v1.215.0) (2023-11-23)


### Features

* query embeddings from s3 ([#2683](https://github.com/windmill-labs/windmill/issues/2683)) ([e3f2b43](https://github.com/windmill-labs/windmill/commit/e3f2b43748e8e0853dd0dbae87cba37b950dc76e))


### Bug Fixes

* bigquery schema and date inputs ([#2688](https://github.com/windmill-labs/windmill/issues/2688)) ([f5e098d](https://github.com/windmill-labs/windmill/commit/f5e098d03b467002a8deab4d2c519945c01458d4))
* improve wmill go client ([cfd3da4](https://github.com/windmill-labs/windmill/commit/cfd3da41efe4e89e5d8672c08433442b05d11f37))

## [1.214.1](https://github.com/windmill-labs/windmill/compare/v1.214.0...v1.214.1) (2023-11-23)


### Features

* **python:** Quality of Life improvements to Python client ([#2686](https://github.com/windmill-labs/windmill/issues/2686)) ([1c2abcd](https://github.com/windmill-labs/windmill/commit/1c2abcda231b2d10e7a358a4e4b7973785cb6199))


### Bug Fixes

* add image base64 source kinds ([7ae84fc](https://github.com/windmill-labs/windmill/commit/7ae84fce5014d97bf17803e2ccefafee009b28bb))
* fix backend build ([45ee1d7](https://github.com/windmill-labs/windmill/commit/45ee1d770345f03c03cc48a3ddcc130ca3655a4c))
* **frontend:** disable active interaction to avoid broken state ([#2675](https://github.com/windmill-labs/windmill/issues/2675)) ([bfdb559](https://github.com/windmill-labs/windmill/commit/bfdb559b47786e37f135ca345c55828cc70e49e9))
* **frontend:** improve tutorial ux ([#2677](https://github.com/windmill-labs/windmill/issues/2677)) ([7fe3bca](https://github.com/windmill-labs/windmill/commit/7fe3bca624fa3b434e3fbe9b33b213449044accc))
* **frontend:** use popover for schedule for later on the runs page ([#2678](https://github.com/windmill-labs/windmill/issues/2678)) ([31fbc58](https://github.com/windmill-labs/windmill/commit/31fbc5867cc72befea5733811148126289ce8bb9))
* graphql variables ([#2682](https://github.com/windmill-labs/windmill/issues/2682)) ([217e694](https://github.com/windmill-labs/windmill/commit/217e69498fa5958e83c595003b2a086528e51388))
* relax tags constraints ([8f8ea22](https://github.com/windmill-labs/windmill/commit/8f8ea227c850f9d01f913660547b6a6394c6e142))
* Various fixes and improvements for Windmill S3 capabilities ([#2674](https://github.com/windmill-labs/windmill/issues/2674)) ([c42c54e](https://github.com/windmill-labs/windmill/commit/c42c54e69b5acdcbe44a0c26ae7fbaa2189e6b5f))

## [1.214.0](https://github.com/windmill-labs/windmill/compare/v1.213.0...v1.214.0) (2023-11-22)


### Features

* hub path scripts + nested inputs glue ([#2668](https://github.com/windmill-labs/windmill/issues/2668)) ([ad199af](https://github.com/windmill-labs/windmill/commit/ad199afd06814540bb7d36669709902be56eeb8a))
* **python:** Refactor Windmill Python client (remove `windmill-api`) ([#2665](https://github.com/windmill-labs/windmill/issues/2665)) ([37ffdae](https://github.com/windmill-labs/windmill/commit/37ffdaed60fb750f1466b440353e1d8409eaea90))


### Bug Fixes

* fix flow primary schedule clearing ([3ebe6d7](https://github.com/windmill-labs/windmill/commit/3ebe6d7a620e37fd6c81bcf4c8713a48eb081f81))

## [1.213.0](https://github.com/windmill-labs/windmill/compare/v1.212.0...v1.213.0) (2023-11-21)


### Features

* code completion UI + other nits ([#2657](https://github.com/windmill-labs/windmill/issues/2657)) ([6d426b4](https://github.com/windmill-labs/windmill/commit/6d426b4ec49d4749a89c86dc5eb1f11bf705ca26))
* Expanding an s3object result now opens the S3 file browser ([#2656](https://github.com/windmill-labs/windmill/issues/2656)) ([baac93f](https://github.com/windmill-labs/windmill/commit/baac93f40140ee37548a273885c028a8e6500b6d))


### Bug Fixes

* ask to return value ([#2659](https://github.com/windmill-labs/windmill/issues/2659)) ([de1e1f5](https://github.com/windmill-labs/windmill/commit/de1e1f545d4cd42f46d9af9a0349af86acf1901c))
* fix embedded approval step timeouts ([51ce2f8](https://github.com/windmill-labs/windmill/commit/51ce2f8cb308da285dab0dc433bf596caa5eeed0))
* fix error handling for list of errors ([c1bb97d](https://github.com/windmill-labs/windmill/commit/c1bb97d990810c9b3d909c9a045f0ce84be6c25e))
* set session code completion to enabled by default ([#2664](https://github.com/windmill-labs/windmill/issues/2664)) ([ded0bb8](https://github.com/windmill-labs/windmill/commit/ded0bb890bb54ef80c857395474e692865ee4717))

## [1.212.0](https://github.com/windmill-labs/windmill/compare/v1.211.0...v1.212.0) (2023-11-20)


### Features

* S3 file picker as a drawer ([#2640](https://github.com/windmill-labs/windmill/issues/2640)) ([624b4d0](https://github.com/windmill-labs/windmill/commit/624b4d0e9898dddcce3cb2ce989ce1f9e4736061))
* test an iteration ([c2598b3](https://github.com/windmill-labs/windmill/commit/c2598b330450f9885f7d10e2b5baa54d6ef88cc5))
* upgrade to gpt-4-turbo ([#2655](https://github.com/windmill-labs/windmill/issues/2655)) ([8ea98c2](https://github.com/windmill-labs/windmill/commit/8ea98c2c8d636209954a267116eb03ab13217ef8))

## [1.211.0](https://github.com/windmill-labs/windmill/compare/v1.210.1...v1.211.0) (2023-11-20)


### Features

* agent mode v0 ([cd260e7](https://github.com/windmill-labs/windmill/commit/cd260e7062802ab39115025577a2456c66435899))
* agent mode v0 ([47ad8d6](https://github.com/windmill-labs/windmill/commit/47ad8d6013e7144a5d018f43c827b171228b5f42))
* ai regex ([19907e4](https://github.com/windmill-labs/windmill/commit/19907e4012e88a8fd28a5f0564a7ea48ec12020c))
* custom error messages for forms ([2f15ebc](https://github.com/windmill-labs/windmill/commit/2f15ebc5f9628b0d26ec08b3527ddc96c6d42ba0))
* **python:** Add functionality and resiliency to wmill python client ([#2650](https://github.com/windmill-labs/windmill/issues/2650)) ([c7a30f7](https://github.com/windmill-labs/windmill/commit/c7a30f7c9db26252f0ee69e1276ddccc0d52acb3))


### Bug Fixes

* add refresh button to item picker ([4882d94](https://github.com/windmill-labs/windmill/commit/4882d94dfe18c156662fe483e1dd4f5d3e3be3af))
* fix base64 assignment (file input) ([70f1210](https://github.com/windmill-labs/windmill/commit/70f121035edd94b3940f530a5603b1ff4bf03839))
* main broken tests ([#2652](https://github.com/windmill-labs/windmill/issues/2652)) ([c90f7f1](https://github.com/windmill-labs/windmill/commit/c90f7f167e0b9c20008a62eeeef7819e24fc3da9))
* token expiry is equal to timeout ([b667317](https://github.com/windmill-labs/windmill/commit/b667317d44f37bb50f24e356a1a3d231ebe8b4b4))

## [1.210.1](https://github.com/windmill-labs/windmill/compare/v1.210.0...v1.210.1) (2023-11-18)


### Bug Fixes

* add toggle to invites ([36379d6](https://github.com/windmill-labs/windmill/commit/36379d6db05de170e0237b12e767b6d6f4a6f2ef))
* auto-invite all instead of by domain ([c07e905](https://github.com/windmill-labs/windmill/commit/c07e9056f1872cae70b8c3bebdbbf47daeee7ac5))
* flow copilot arg types ([#2648](https://github.com/windmill-labs/windmill/issues/2648)) ([08c14e5](https://github.com/windmill-labs/windmill/commit/08c14e51c792fe65d4f993379eff0e5c8a75215b))
* improve error message for unauthorized variables/resources ([5a72ca9](https://github.com/windmill-labs/windmill/commit/5a72ca9b24c5c4e9fe94c7865b9145283aceff53))
* leave workspace + instance api ([ee243de](https://github.com/windmill-labs/windmill/commit/ee243dedc6df28a64f15e0b274b7fa96f6428474))
* make wmill compatible with python 3.7 ([6ae1a69](https://github.com/windmill-labs/windmill/commit/6ae1a69b75fe039586956064880c274a21fc5970))
* migrate old state env variable to new env variable ([2692737](https://github.com/windmill-labs/windmill/commit/2692737418ed601c0a36c368f59ffb8d10d9ad38))

## [1.210.0](https://github.com/windmill-labs/windmill/compare/v1.209.0...v1.210.0) (2023-11-17)


### Features

* truncate logs when necessary ([05d3fca](https://github.com/windmill-labs/windmill/commit/05d3fcad269adebd2a701da8a49cdc692f26b18f))
* truncate logs when necessary ([4d92745](https://github.com/windmill-labs/windmill/commit/4d927457867a3c828b5d09f43fc905984769e29f))


### Bug Fixes

* remove stats payload logging ([#2644](https://github.com/windmill-labs/windmill/issues/2644)) ([cc978c2](https://github.com/windmill-labs/windmill/commit/cc978c230fe65d4f93b369abff96ee3354d3a899))
* restore flow local storage ([526b14c](https://github.com/windmill-labs/windmill/commit/526b14cca3ca5eb3fc2c60e404b057da05b84b4c))

## [1.209.0](https://github.com/windmill-labs/windmill/compare/v1.208.0...v1.209.0) (2023-11-17)


### Features

* add prompt history ([#2638](https://github.com/windmill-labs/windmill/issues/2638)) ([c2557a2](https://github.com/windmill-labs/windmill/commit/c2557a270f89d4bc055d03833bb90b20161e9de0))
* collect vcpus and accounts usage ([#2635](https://github.com/windmill-labs/windmill/issues/2635)) ([8183c38](https://github.com/windmill-labs/windmill/commit/8183c38d05da220e036ec36a8ac5cc6f6e004d56))
* dedicated workers for flows ([#2637](https://github.com/windmill-labs/windmill/issues/2637)) ([b13cc58](https://github.com/windmill-labs/windmill/commit/b13cc58315ea07d67d98bffb907e2cbd0c1d6b7d))
* **frontend:** add filter by resource on Audit Log page ([#2628](https://github.com/windmill-labs/windmill/issues/2628)) ([1d1258f](https://github.com/windmill-labs/windmill/commit/1d1258ff5873023e66e727a87d6ecb04d1a77e70))
* **frontend:** Add optional dropdown trigger ([#2642](https://github.com/windmill-labs/windmill/issues/2642)) ([998843b](https://github.com/windmill-labs/windmill/commit/998843ba62a9b154c7a4903bef5b6fc2dc3fafe5))


### Bug Fixes

* improve suspended flow handling when missing next step ([316d5f3](https://github.com/windmill-labs/windmill/commit/316d5f344f409ca564f92e32d6a04068a316c012))
* nested schemas can use required ([33dfcfb](https://github.com/windmill-labs/windmill/commit/33dfcfbdc74d611e457e54cc8ddb9152bf365560))
* support non root imports in bun ([9ff428f](https://github.com/windmill-labs/windmill/commit/9ff428f210dd1dea6cff664aaade1411317587c5))
* www-url-encoded body ([9c7bb5e](https://github.com/windmill-labs/windmill/commit/9c7bb5ecd9f169d627d98cc1a494d0e5010f0166))
* x overflow scroll + add clear history btn ([#2641](https://github.com/windmill-labs/windmill/issues/2641)) ([1aef2aa](https://github.com/windmill-labs/windmill/commit/1aef2aae9027010346ce352e6a6993fb175bc904))

## [1.208.0](https://github.com/windmill-labs/windmill/compare/v1.207.0...v1.208.0) (2023-11-15)


### Features

* Add Python SDK capabilities to generate Polars and DuckDB connection settings to S3 ([#2625](https://github.com/windmill-labs/windmill/issues/2625)) ([9009365](https://github.com/windmill-labs/windmill/commit/90093656b49f78d550012ff2e8627fc43b8e26ea))
* add yaml for import/export/diff ([#2631](https://github.com/windmill-labs/windmill/issues/2631)) ([34abe01](https://github.com/windmill-labs/windmill/commit/34abe01c2f0d9d7055ed17cb2dd34acf9d5e2e88))
* sync team plans ([#2619](https://github.com/windmill-labs/windmill/issues/2619)) ([f38498b](https://github.com/windmill-labs/windmill/commit/f38498b725cb475d8b4c04641d3fa9870ed23adc))


### Bug Fixes

* Resource secret heuristic is wrong ([#2632](https://github.com/windmill-labs/windmill/issues/2632)) ([0d6c1ec](https://github.com/windmill-labs/windmill/commit/0d6c1ec064d83d3394791cef776bcd33ef3f570f))
* stripe ops ee only ([#2634](https://github.com/windmill-labs/windmill/issues/2634)) ([e3cb2e6](https://github.com/windmill-labs/windmill/commit/e3cb2e66ba41b156e2833ce54b1b153eba20785e))

## [1.207.0](https://github.com/windmill-labs/windmill/compare/v1.206.0...v1.207.0) (2023-11-15)


### Features

* cache postgres connection ([#2621](https://github.com/windmill-labs/windmill/issues/2621)) ([ff8b9b0](https://github.com/windmill-labs/windmill/commit/ff8b9b03848bf44303bb3dc7d04572823fef28f8))


### Bug Fixes

* ai gen minor fixes ([#2626](https://github.com/windmill-labs/windmill/issues/2626)) ([eafde10](https://github.com/windmill-labs/windmill/commit/eafde1099532caacd23af19fd1405f71e6f7f723))
* copy component only if empty selection ([9fd8a31](https://github.com/windmill-labs/windmill/commit/9fd8a31e1c3b515022e7967d5195c9b38a1e4cc3))
* false not undefined in result ([#2620](https://github.com/windmill-labs/windmill/issues/2620)) ([f185eba](https://github.com/windmill-labs/windmill/commit/f185eba49cc84fc0bcfcfd00716999c11212db44))
* fix delete schedule clear jobs ([244b85e](https://github.com/windmill-labs/windmill/commit/244b85e859e74e8e509c48c22dc9f543f11dd1f2))
* fix diff editor error ([cbeefa8](https://github.com/windmill-labs/windmill/commit/cbeefa807a25e06dc8052e5c0984fe9c883da460))
* **frontend:** Fix Step viewer ([#2627](https://github.com/windmill-labs/windmill/issues/2627)) ([1daccdb](https://github.com/windmill-labs/windmill/commit/1daccdbdb104f4d6e619ed0bdf28ef9ecc15bebb))

## [1.206.0](https://github.com/windmill-labs/windmill/compare/v1.205.0...v1.206.0) (2023-11-12)


### Features

* Add Authentik SSO support ([#2614](https://github.com/windmill-labs/windmill/issues/2614)) ([ce38a43](https://github.com/windmill-labs/windmill/commit/ce38a4322e13b35660f16d4a3ca66224487dd27b))
* add dedicated worker support for deno ([528f6fa](https://github.com/windmill-labs/windmill/commit/528f6fa9107d3b93c36fd5418ca95b7f2b701445))
* add support for requiring users to pre-exist ([eb5067a](https://github.com/windmill-labs/windmill/commit/eb5067aee5fd0c64614c967ef4b2af2398e8807c))


### Bug Fixes

* add windmill loading screen ([8151b01](https://github.com/windmill-labs/windmill/commit/8151b01013383a83ce8aca8c2f918ead159b4273))
* fix selected component switch tab + copyComponent keep layout ([6a819cc](https://github.com/windmill-labs/windmill/commit/6a819cc51211cba7009d985caddb9ae6f0ec8f48))
* handle console logs in dedicated workers ([fa3efd3](https://github.com/windmill-labs/windmill/commit/fa3efd3f608a754f0c271da557a244ff6b359cfd))
* keep dark theme on login redirect ([7bbe3cf](https://github.com/windmill-labs/windmill/commit/7bbe3cf288ec480e56cc33a5d3248c00567e137f))
* update monaco ([#2616](https://github.com/windmill-labs/windmill/issues/2616)) ([f809172](https://github.com/windmill-labs/windmill/commit/f809172e3da88594d7e5a3ed8a12116e804ca6ae))

## [1.205.0](https://github.com/windmill-labs/windmill/compare/v1.204.1...v1.205.0) (2023-11-11)


### Features

* exit dedicated worker upon new deployment ([2038fe6](https://github.com/windmill-labs/windmill/commit/2038fe68ce2ffeeed34c4cfcb44c46ad08f8e732))
* exit dedicated worker upon new deployment ([4661dd2](https://github.com/windmill-labs/windmill/commit/4661dd2cea343572099ca66459d3db5f3c7879c8))


### Bug Fixes

* improve display of optimized one-loops + dependency jobs of dedicated workers happen on dedicated worker ([f998c72](https://github.com/windmill-labs/windmill/commit/f998c7229cd0b27d906a79f99ee7c6e0da1d1810))
* Invalid config for workers does not panic ([#2612](https://github.com/windmill-labs/windmill/issues/2612)) ([aee35d6](https://github.com/windmill-labs/windmill/commit/aee35d6d511d16130fb64ae4dd2e28757e99f79a))
* support digit prefixed script in python ([b9e41d0](https://github.com/windmill-labs/windmill/commit/b9e41d066da801ac43276c48af3f100aa70c5a5d))

## [1.204.1](https://github.com/windmill-labs/windmill/compare/v1.204.0...v1.204.1) (2023-11-10)


### Bug Fixes

* fix custom components ([4136442](https://github.com/windmill-labs/windmill/commit/41364421ea2ed5980bada139261760bbb6ee8e31))
* **frontend:** fix login icons + add Okta ([#2609](https://github.com/windmill-labs/windmill/issues/2609)) ([e22f373](https://github.com/windmill-labs/windmill/commit/e22f3738d512b4d7657acc8d4ddf280039acbe56))
* optimize single step iterative forloops ([#2596](https://github.com/windmill-labs/windmill/issues/2596)) ([88e3648](https://github.com/windmill-labs/windmill/commit/88e3648ee413286769b72acc02a4af6173fa6bac))

## [1.204.0](https://github.com/windmill-labs/windmill/compare/v1.203.0...v1.204.0) (2023-11-10)


### Features

* add sql server ([#2604](https://github.com/windmill-labs/windmill/issues/2604)) ([577e130](https://github.com/windmill-labs/windmill/commit/577e1300b93773ab038b067574928d92cae69275))
* add support for custom sso logins ([0ccf706](https://github.com/windmill-labs/windmill/commit/0ccf706fa28ba615b887ae9c930089be45f14b13))
* **frontend:** add confirmation modal when deleting a user in instance settings ([#2608](https://github.com/windmill-labs/windmill/issues/2608)) ([a99edf7](https://github.com/windmill-labs/windmill/commit/a99edf7764f1a46809387f851fd9acdb1057810a))
* **frontend:** Buttons refactor ([#2545](https://github.com/windmill-labs/windmill/issues/2545)) ([fe35c7a](https://github.com/windmill-labs/windmill/commit/fe35c7ad3cf5cb9d8ebcd2a0723533200034fa74))
* **frontend:** draft script/flow can only access the edit page until… ([#2607](https://github.com/windmill-labs/windmill/issues/2607)) ([adad712](https://github.com/windmill-labs/windmill/commit/adad71266269c17d27ca1bbe8ebe5505b89eb855))


### Bug Fixes

* `iter` args conflicts with external variables named iter ([#2605](https://github.com/windmill-labs/windmill/issues/2605)) ([cb01703](https://github.com/windmill-labs/windmill/commit/cb01703a08f4f63362af98594eec4d08e3f25c04))

## [1.203.0](https://github.com/windmill-labs/windmill/compare/v1.202.1...v1.203.0) (2023-11-09)


### Features

* add support for custom components in react or vanilla JS ([#2603](https://github.com/windmill-labs/windmill/issues/2603)) ([28c9fdc](https://github.com/windmill-labs/windmill/commit/28c9fdc4f209bcc099f741a448cd3af0797acd53))
* **frontend:** add a way to customise the link's label ([#2591](https://github.com/windmill-labs/windmill/issues/2591)) ([72854b5](https://github.com/windmill-labs/windmill/commit/72854b55b9db5c7c2ec3cbf65b0ed851ca7eb29a))
* **frontend:** Migrate flow advanced settings to new layout ([#2589](https://github.com/windmill-labs/windmill/issues/2589)) ([55e3a95](https://github.com/windmill-labs/windmill/commit/55e3a9561899127ba647ff87d32cf010f2aefb90))


### Bug Fixes

* Fix error handler token injection ([#2598](https://github.com/windmill-labs/windmill/issues/2598)) ([aefa43d](https://github.com/windmill-labs/windmill/commit/aefa43dcafe929d8939dd2ee5ba94633759204a7))
* frontend build ([#2593](https://github.com/windmill-labs/windmill/issues/2593)) ([a87b15f](https://github.com/windmill-labs/windmill/commit/a87b15f2c56c19e6f901da69240b3c127ece3b50))
* Frontend workspace error handler args ([#2597](https://github.com/windmill-labs/windmill/issues/2597)) ([fc87413](https://github.com/windmill-labs/windmill/commit/fc874132c029a7fb2571fe5c296c836b451b351a))
* **frontend:** correctly clear result when removing all options in a multi select ([#2600](https://github.com/windmill-labs/windmill/issues/2600)) ([ed24838](https://github.com/windmill-labs/windmill/commit/ed24838b46916f8415afcfab3e9700d2ffad9a63))
* Pythong script in dedicated worker fails with more than 1 arg ([#2588](https://github.com/windmill-labs/windmill/issues/2588)) ([0d846b3](https://github.com/windmill-labs/windmill/commit/0d846b310d8f1ade8a01607d49c6e50ba417f54e))
* s3 snippets arg inputs update ([#2592](https://github.com/windmill-labs/windmill/issues/2592)) ([16a5fb9](https://github.com/windmill-labs/windmill/commit/16a5fb9e8662afdf84c7e87dbe7a8db7d7f09563))

## [1.202.1](https://github.com/windmill-labs/windmill/compare/v1.202.0...v1.202.1) (2023-11-07)


### Bug Fixes

* remove the FOO ([f89a01f](https://github.com/windmill-labs/windmill/commit/f89a01ff2fb26224884e59599f72da0b83fa4a0f))

## [1.202.0](https://github.com/windmill-labs/windmill/compare/v1.201.0...v1.202.0) (2023-11-07)


### Features

* add diffs when editing workspace script inside a flow ([#2581](https://github.com/windmill-labs/windmill/issues/2581)) ([e84e38d](https://github.com/windmill-labs/windmill/commit/e84e38d3bd4e3cb95fc71f91b3bd133740d82b05))
* Add override all schedule handlers button ([#2579](https://github.com/windmill-labs/windmill/issues/2579)) ([f2bff84](https://github.com/windmill-labs/windmill/commit/f2bff8450223d29d3de8edd2e60d483f6ced5caa))
* add support for flows in vscode extension ([#2585](https://github.com/windmill-labs/windmill/issues/2585)) ([8a7fe93](https://github.com/windmill-labs/windmill/commit/8a7fe93559209e7aa5427f5b6a8b9e03df9da406))
* **frontend:** Rework variable table ([#2576](https://github.com/windmill-labs/windmill/issues/2576)) ([b040a89](https://github.com/windmill-labs/windmill/commit/b040a89b27f6dca41049e4bceeae4e3665c005ae))


### Bug Fixes

* add tag support for CLI ([0ede0f4](https://github.com/windmill-labs/windmill/commit/0ede0f4c972eb1b65dcb542ea6facf5ee2c74cfd))
* add tag sync for cli ([6c12c6e](https://github.com/windmill-labs/windmill/commit/6c12c6e7846c8be3f843ca7a98b8bac6fac1d7e8))
* **frontend:** add missing classes when tabs are in sidebar mode ([#2577](https://github.com/windmill-labs/windmill/issues/2577)) ([dd3594c](https://github.com/windmill-labs/windmill/commit/dd3594c5e5624b712564126f046fde4fc06c42ca))
* **frontend:** escape wasn't removing the hash when closing a drawer ([#2583](https://github.com/windmill-labs/windmill/issues/2583)) ([0951431](https://github.com/windmill-labs/windmill/commit/0951431e419c127cc562158447012960feb6d3af))
* handle graphql invalid response ([#2582](https://github.com/windmill-labs/windmill/issues/2582)) ([12e731b](https://github.com/windmill-labs/windmill/commit/12e731b5c03fa788c9f0d00a955b7b01b4c570a0))

## [1.201.0](https://github.com/windmill-labs/windmill/compare/v1.200.0...v1.201.0) (2023-11-06)


### Features

* add new stats ([#2568](https://github.com/windmill-labs/windmill/issues/2568)) ([1ed52ab](https://github.com/windmill-labs/windmill/commit/1ed52ab4c96988847c74c0672497dce1dd24ff6d))
* Add toggle to optionally mute error handler for cancelled jobs ([#2567](https://github.com/windmill-labs/windmill/issues/2567)) ([83f9ef3](https://github.com/windmill-labs/windmill/commit/83f9ef34e6f7bb207f3410cbe2cceca7b52a4dea))
* **frontend:** Ag grid styling ([#2565](https://github.com/windmill-labs/windmill/issues/2565)) ([97c5fe8](https://github.com/windmill-labs/windmill/commit/97c5fe8985166c9dae063de7e9d122914b190a4e))
* telemetry disclosure ([#2562](https://github.com/windmill-labs/windmill/issues/2562)) ([1bb8b60](https://github.com/windmill-labs/windmill/commit/1bb8b606ed2e8ade12d5278072cc2b57c8d3ca27))


### Bug Fixes

* add no changes popup when saving draft ([#2571](https://github.com/windmill-labs/windmill/issues/2571)) ([d3dbb18](https://github.com/windmill-labs/windmill/commit/d3dbb188156cfd98e6cb1348d40e8854c008559e))
* apps diffs ([#2570](https://github.com/windmill-labs/windmill/issues/2570)) ([3ed7ae7](https://github.com/windmill-labs/windmill/commit/3ed7ae7ffa28cb8b8c4799034ae8f0c8822fd519))
* flow diffs ([#2561](https://github.com/windmill-labs/windmill/issues/2561)) ([aa5b71c](https://github.com/windmill-labs/windmill/commit/aa5b71ca05429788078547b3249eb1c3cd375ccc))
* **frontend:** fix label event issues ([#2574](https://github.com/windmill-labs/windmill/issues/2574)) ([8935d22](https://github.com/windmill-labs/windmill/commit/8935d2272fcd630ccb1ab70ba0fa334934640fcb))
* improve dedicated workers ([141f45b](https://github.com/windmill-labs/windmill/commit/141f45bf95388c0547e84980096a99607f3dba2f))
* minor bug fixes ([#2566](https://github.com/windmill-labs/windmill/issues/2566)) ([e195202](https://github.com/windmill-labs/windmill/commit/e19520295f41784aae66df0d686b22fec810d57b))

## [1.200.0](https://github.com/windmill-labs/windmill/compare/v1.199.0...v1.200.0) (2023-11-04)


### Features

* improve drafts and diffs ([#2534](https://github.com/windmill-labs/windmill/issues/2534)) ([3bfc2c8](https://github.com/windmill-labs/windmill/commit/3bfc2c81d2d405c8ea12d68622bbf7175b3947db))


### Bug Fixes

* **frontend:** fix treeview ([#2552](https://github.com/windmill-labs/windmill/issues/2552)) ([02764b1](https://github.com/windmill-labs/windmill/commit/02764b1fad7e2f44f46b49cbe7500266e9cc2f8e))
* return non-integer sleep error directly ([#2558](https://github.com/windmill-labs/windmill/issues/2558)) ([543fae7](https://github.com/windmill-labs/windmill/commit/543fae77a75b88a1199f8d3cbb0460257ed5db95))

## [1.199.0](https://github.com/windmill-labs/windmill/compare/v1.198.0...v1.199.0) (2023-11-03)


### Features

* Schedule error handler improvements ([#2555](https://github.com/windmill-labs/windmill/issues/2555)) ([668c9da](https://github.com/windmill-labs/windmill/commit/668c9da6461997c1b7907111bbfd0eff5e0ec159))


### Bug Fixes

* fail on non-integer sleep value ([#2556](https://github.com/windmill-labs/windmill/issues/2556)) ([6f47b96](https://github.com/windmill-labs/windmill/commit/6f47b9600629ec24a4e265a0ccc9eee75458229f))

## [1.198.0](https://github.com/windmill-labs/windmill/compare/v1.197.1...v1.198.0) (2023-11-03)


### Features

* **frontend:** fix table when seaching with hidden columns ([#2549](https://github.com/windmill-labs/windmill/issues/2549)) ([0aaffad](https://github.com/windmill-labs/windmill/commit/0aaffadf0ca6964539011733b5d4882fdd26588a))
* **frontend:** update displayed path for treeview ([#2551](https://github.com/windmill-labs/windmill/issues/2551)) ([0349ba5](https://github.com/windmill-labs/windmill/commit/0349ba5d567f2b52ff8058f347e501598ce4c981))


### Bug Fixes

* **frontend:** fix mobile sidebar opacity ([#2554](https://github.com/windmill-labs/windmill/issues/2554)) ([e1e48cf](https://github.com/windmill-labs/windmill/commit/e1e48cfc5f3cb68aa4bbd18a1a2ad6f0a300c374))
* make graph rendering uniform across all rem ([#2553](https://github.com/windmill-labs/windmill/issues/2553)) ([0d4fc6a](https://github.com/windmill-labs/windmill/commit/0d4fc6a0bbc9b8e20c20bf2646eab202f795b8bd))
* make python imports work at any nesting level ([75a5766](https://github.com/windmill-labs/windmill/commit/75a5766f8bf14c3749aff56fb94a8c04b32de4b6))
* make timeline fit for high number of iterations for flows ([37eac60](https://github.com/windmill-labs/windmill/commit/37eac608666d903504872ce815839a7493fe876a))
* subflow with cache can not be considered simple ([54f0812](https://github.com/windmill-labs/windmill/commit/54f08122d2837b5ee3d283733e0da403b21fadf0))
* support results[&lt;x&gt;] + export more metatada for scripts ([0f37439](https://github.com/windmill-labs/windmill/commit/0f37439877ab498a615746357d771703db47a6d2))

## [1.197.1](https://github.com/windmill-labs/windmill/compare/v1.197.0...v1.197.1) (2023-11-02)


### Bug Fixes

* fix cli ([77e0e2e](https://github.com/windmill-labs/windmill/commit/77e0e2ebc1fbe00eec431bd5d20619b89e8b7511))
* Slack error handler missing "slack" arg ([#2546](https://github.com/windmill-labs/windmill/issues/2546)) ([7ba2a6c](https://github.com/windmill-labs/windmill/commit/7ba2a6c4f111b980181034ef5181193996c19fc4))

## [1.197.0](https://github.com/windmill-labs/windmill/compare/v1.196.0...v1.197.0) (2023-11-02)


### Features

* **frontend:** add treeview ([#2542](https://github.com/windmill-labs/windmill/issues/2542)) ([86a2ced](https://github.com/windmill-labs/windmill/commit/86a2ced605fbab27bd01984c846e467a2612102b))
* **frontend:** fix sidebar ([#2544](https://github.com/windmill-labs/windmill/issues/2544)) ([b4f043d](https://github.com/windmill-labs/windmill/commit/b4f043d32dd4fefc104d0fca429f4b39a23e1166))
* **frontend:** fix sidebar color ([#2541](https://github.com/windmill-labs/windmill/issues/2541)) ([30a9460](https://github.com/windmill-labs/windmill/commit/30a9460cca676ac8f8e585024a0927ca90252f17))


### Bug Fixes

* enable default tokenizer truncation ([#2537](https://github.com/windmill-labs/windmill/issues/2537)) ([29aabd3](https://github.com/windmill-labs/windmill/commit/29aabd3472f59a4b5a657e7b046d66183d5fa0ba))
* fix powershell args passing ([b4d5c5a](https://github.com/windmill-labs/windmill/commit/b4d5c5add8b92db1094e46c347efded52aa0f389))
* improve rendering of list with undefined heights ([9eec2e2](https://github.com/windmill-labs/windmill/commit/9eec2e2c3e0183520cc50c716342bf329145edbd))

## [1.196.0](https://github.com/windmill-labs/windmill/compare/v1.195.0...v1.196.0) (2023-11-01)


### Features

* improve inputs handling for large list on apps ([270d871](https://github.com/windmill-labs/windmill/commit/270d871039c708b7cfa218e22650fc25b1ec841c))

## [1.195.0](https://github.com/windmill-labs/windmill/compare/v1.194.0...v1.195.0) (2023-10-31)


### Features

* Ability to restart flow on loop/branchall iteration ([#2526](https://github.com/windmill-labs/windmill/issues/2526)) ([c31299b](https://github.com/windmill-labs/windmill/commit/c31299bed8110f53e31f69983f144aaa82d5560d))
* **frontend:** chartjs wizard ([#2532](https://github.com/windmill-labs/windmill/issues/2532)) ([03dfe71](https://github.com/windmill-labs/windmill/commit/03dfe711c6292d8d9af78ed610fd2885ad62b8d7))
* invalidate result cache on flow or script change ([cf9669c](https://github.com/windmill-labs/windmill/commit/cf9669c18de6091dbf5dafad0f6ffd6e17675ca4))


### Bug Fixes

* add on success events to triggers list ([1974012](https://github.com/windmill-labs/windmill/commit/1974012621f3a2112eace60ba5d68854d567c9c2))
* fix quick search scripts ([b3d2213](https://github.com/windmill-labs/windmill/commit/b3d2213ccec73bf4a7f27242ac31eb0941b791ac))
* Load schedule statistics in background ([#2530](https://github.com/windmill-labs/windmill/issues/2530)) ([c98ebf9](https://github.com/windmill-labs/windmill/commit/c98ebf92e5f99f63cc3754555b5867f71c09e1a9))
* only load embeddings if in server mode ([c803631](https://github.com/windmill-labs/windmill/commit/c8036317d23bf81b474742223e903255dd8825e0))

## [1.194.0](https://github.com/windmill-labs/windmill/compare/v1.193.0...v1.194.0) (2023-10-30)


### Features

* **frontend:** plotly wizard ([#2517](https://github.com/windmill-labs/windmill/issues/2517)) ([40adfdb](https://github.com/windmill-labs/windmill/commit/40adfdb7fd6e21d053e79c6db876a4a8c90509c3))
* Restartable flows ([#2514](https://github.com/windmill-labs/windmill/issues/2514)) ([76a736a](https://github.com/windmill-labs/windmill/commit/76a736aee67517807d86e1c8c3961af907fc919c))


### Bug Fixes

* assign column length of table actions based on number of actions ([0c672e7](https://github.com/windmill-labs/windmill/commit/0c672e7e18b52d4615dcf473bf1424dfe685cc9d))
* fix table reactivity ([01560db](https://github.com/windmill-labs/windmill/commit/01560dbdaf2dc30c16e9182b5a3353d13c927827))
* **frontend:** fix ai gen ([#2518](https://github.com/windmill-labs/windmill/issues/2518)) ([6f4fb76](https://github.com/windmill-labs/windmill/commit/6f4fb7668cd09d2e5fdca8977718fd6b87883a27))
* make modal and drawer button hiddable as config ([2e55af5](https://github.com/windmill-labs/windmill/commit/2e55af50c70b9d1118e9a63f119ac31e5a574e51))
* workspaced embeddings for resource types ([#2525](https://github.com/windmill-labs/windmill/issues/2525)) ([302649f](https://github.com/windmill-labs/windmill/commit/302649faa858233ea62073a2460e1586db67249f))

## [1.193.0](https://github.com/windmill-labs/windmill/compare/v1.192.0...v1.193.0) (2023-10-28)


### Features

* refactor metrics and add performance debug metrics ([#2520](https://github.com/windmill-labs/windmill/issues/2520)) ([b888348](https://github.com/windmill-labs/windmill/commit/b8883481f46f1317748bb1884e0f0d287e8ae7fa))


### Bug Fixes

* **frontend:** add a disabled prop to text inputs ([#2512](https://github.com/windmill-labs/windmill/issues/2512)) ([7164de8](https://github.com/windmill-labs/windmill/commit/7164de81b02376f61eac965f08d75ab8b790e0ee))
* **frontend:** fix insert new item ([#2519](https://github.com/windmill-labs/windmill/issues/2519)) ([c4383cf](https://github.com/windmill-labs/windmill/commit/c4383cfe740220f4674c93beeae9eb74397f7aff))
* **frontend:** increased size limit for images ([#2510](https://github.com/windmill-labs/windmill/issues/2510)) ([aaa2657](https://github.com/windmill-labs/windmill/commit/aaa26579dc5e24f46e554a42c0121fb6a04d58f3))
* move keep job directories and expose debug metrics to instance settings UI ([55ceca1](https://github.com/windmill-labs/windmill/commit/55ceca19131ac6dfb190f8818b18b46ca329babc))
* prometheus metrics are an instance settings ([ea28163](https://github.com/windmill-labs/windmill/commit/ea28163865a22174dc1b92242a24989a1a47af21))

## [1.192.0](https://github.com/windmill-labs/windmill/compare/v1.191.0...v1.192.0) (2023-10-25)


### Features

* **frontend:** add display borders configuration to list component ([#2508](https://github.com/windmill-labs/windmill/issues/2508)) ([dc54829](https://github.com/windmill-labs/windmill/commit/dc548292ac05f92116d2d65863da3a52e0cfe027))


### Bug Fixes

* do not share http_client in js_eval runtime ([402193c](https://github.com/windmill-labs/windmill/commit/402193cef9eff0ca03f5bd854d29f95774c4b73e))
* fix global instance dynamic css ([8efe0ca](https://github.com/windmill-labs/windmill/commit/8efe0cadacaae894cf93a3a569e3d0b8e79c7d14))

## [1.191.0](https://github.com/windmill-labs/windmill/compare/v1.190.3...v1.191.0) (2023-10-24)


### Features

* Priority worker tags ([#2504](https://github.com/windmill-labs/windmill/issues/2504)) ([51f2198](https://github.com/windmill-labs/windmill/commit/51f2198c3403a424787b8dee51bc7eddc13c31b8))


### Bug Fixes

* concurrency limit EE feature warning ([#2505](https://github.com/windmill-labs/windmill/issues/2505)) ([927cbbe](https://github.com/windmill-labs/windmill/commit/927cbbe23090b212b13c106b65ad65668baf2f04))
* improve concurrency limit lock ([d44b078](https://github.com/windmill-labs/windmill/commit/d44b078e70a5782f1a1c88a4546d369a547e966a))
* improve runs to display flow informations ([9eaffa5](https://github.com/windmill-labs/windmill/commit/9eaffa5b5fe59ed9e0e7e2cea0721eea75b3d1b3))

## [1.190.3](https://github.com/windmill-labs/windmill/compare/v1.190.2...v1.190.3) (2023-10-24)


### Bug Fixes

* sort arg infos on the client-side ([8025a27](https://github.com/windmill-labs/windmill/commit/8025a27b8ce36b9c9b8d1d17d72075819f58c607))

## [1.190.2](https://github.com/windmill-labs/windmill/compare/v1.190.1...v1.190.2) (2023-10-24)


### Bug Fixes

* ListableQueuedJob were missing the priority field ([#2500](https://github.com/windmill-labs/windmill/issues/2500)) ([96f3854](https://github.com/windmill-labs/windmill/commit/96f38541459195ed70bd24b62b7d1f081be4cb95))

## [1.190.1](https://github.com/windmill-labs/windmill/compare/v1.190.0...v1.190.1) (2023-10-24)


### Bug Fixes

* add shapefile to python remapping ([2bd629e](https://github.com/windmill-labs/windmill/commit/2bd629ecabaded79df9e504fea4136bc8c365e03))
* improve flow performance at high-throughput ([1ec56c0](https://github.com/windmill-labs/windmill/commit/1ec56c0e237eedd6f13b86e077c3b90f6862a414))
* Track job UUIDs in concurrency table instead of a simple counter ([#2498](https://github.com/windmill-labs/windmill/issues/2498)) ([f428581](https://github.com/windmill-labs/windmill/commit/f4285812696a80e3a92a0e5d79e5c19ca78d58fb))

## [1.190.0](https://github.com/windmill-labs/windmill/compare/v1.189.0...v1.190.0) (2023-10-23)


### Features

* dedicated worker for python ([#2492](https://github.com/windmill-labs/windmill/issues/2492)) ([39f3078](https://github.com/windmill-labs/windmill/commit/39f30785a04a54c651e532d7ede3b8c17cdec7ea))


### Bug Fixes

* stats nits ([#2490](https://github.com/windmill-labs/windmill/issues/2490)) ([ec65fa8](https://github.com/windmill-labs/windmill/commit/ec65fa84cc233789b0335a558edfe7e2db6b729d))

## [1.189.0](https://github.com/windmill-labs/windmill/compare/v1.188.1...v1.189.0) (2023-10-23)


### Features

* add form validation for apps ([8ac0562](https://github.com/windmill-labs/windmill/commit/8ac0562a3625546ec9e16db12d310e65fb9e867f))
* add unique id ([#2483](https://github.com/windmill-labs/windmill/issues/2483)) ([7037d70](https://github.com/windmill-labs/windmill/commit/7037d70ca235aa85c0f550e2c6c03cf457fe6eeb))
* dragndrop component on creation ([2b70184](https://github.com/windmill-labs/windmill/commit/2b7018413a90274164a4c5743ddf1631b1b62f9f))
* improve dragndrop experience on editor ([6951331](https://github.com/windmill-labs/windmill/commit/69513319e783b800e857270e03f180b078156afd))
* Priority queue ([#2476](https://github.com/windmill-labs/windmill/issues/2476)) ([3f4af48](https://github.com/windmill-labs/windmill/commit/3f4af48b0b584096f5753a618ac3de11d89063b6))


### Bug Fixes

* fix drawer escape order ([8a8feb3](https://github.com/windmill-labs/windmill/commit/8a8feb378ee086357e71d6f0eb4c4a37d35db066))
* fix include_header ([44c3c96](https://github.com/windmill-labs/windmill/commit/44c3c96d3eb72d2c2fc75e83b3490f5edffeb88b))
* graphql local sync ([#2488](https://github.com/windmill-labs/windmill/issues/2488)) ([2e8dba9](https://github.com/windmill-labs/windmill/commit/2e8dba94425cf5b14ecfe58023f394a10256bdf0))
* powershell local sync ([#2489](https://github.com/windmill-labs/windmill/issues/2489)) ([3c6186d](https://github.com/windmill-labs/windmill/commit/3c6186dc50891a68aefc44424cb412075c00f880))
* Update openapi definition to be compatible with oapi-codegen ([#2487](https://github.com/windmill-labs/windmill/issues/2487)) ([af45ef0](https://github.com/windmill-labs/windmill/commit/af45ef06f2edfba671043274e69f6b53cc1e31f5))

## [1.188.1](https://github.com/windmill-labs/windmill/compare/v1.188.0...v1.188.1) (2023-10-21)


### Bug Fixes

* allow superadmin to run inline scripts in repo they are not part of ([cef2949](https://github.com/windmill-labs/windmill/commit/cef2949497e03bd31e4804e820ca7204962ebd23))

## [1.188.0](https://github.com/windmill-labs/windmill/compare/v1.187.0...v1.188.0) (2023-10-21)


### Features

* enable secret to be read from file ([#2477](https://github.com/windmill-labs/windmill/issues/2477)) ([7905e2c](https://github.com/windmill-labs/windmill/commit/7905e2c853fa519f2ad868c14679c3a3fad17205))


### Bug Fixes

* fix app reactivity ([656cb83](https://github.com/windmill-labs/windmill/commit/656cb83d6b86353598a53f9a9071c7e47185f18e))

## [1.187.0](https://github.com/windmill-labs/windmill/compare/v1.186.0...v1.187.0) (2023-10-21)


### Features

* timelines for apps ([d0161d2](https://github.com/windmill-labs/windmill/commit/d0161d2772e6bead7917bcfc12e721d0c79bca69))
* timelines for apps ([2385e18](https://github.com/windmill-labs/windmill/commit/2385e182867a4cc51a268545a9c62e7c9a90aa20))


### Bug Fixes

* cache embedding model in docker img ([#2474](https://github.com/windmill-labs/windmill/issues/2474)) ([8fe30ca](https://github.com/windmill-labs/windmill/commit/8fe30ca4caae488c8586d35ec2979ac045f86eb3))
* **frontend:** Remove monaco as a dependency of AppPreview ([#2475](https://github.com/windmill-labs/windmill/issues/2475)) ([dd1e03d](https://github.com/windmill-labs/windmill/commit/dd1e03de4a965f75a66fda027e993435684d0790))
* hub scripts search types ([#2471](https://github.com/windmill-labs/windmill/issues/2471)) ([e0edd37](https://github.com/windmill-labs/windmill/commit/e0edd3763704e0e52956043fb20e73fb8380cad1))
* windmill_status_code script now properly return + script bash default arg ([255dd53](https://github.com/windmill-labs/windmill/commit/255dd53ed38deb116eb09d202f2c615e63239c6c))

## [1.186.0](https://github.com/windmill-labs/windmill/compare/v1.185.0...v1.186.0) (2023-10-19)


### Features

* Approval step optionally require logged-in user ([#2462](https://github.com/windmill-labs/windmill/issues/2462)) ([9442068](https://github.com/windmill-labs/windmill/commit/9442068374f57263bf3be5ecae03c95bd6ac5702))
* Flow approvers user groups can be JS InputTransforms ([#2468](https://github.com/windmill-labs/windmill/issues/2468)) ([1200add](https://github.com/windmill-labs/windmill/commit/1200add2a7a3cac4f2519db33f9285e38591b19d))
* local hub embeddings search ([#2463](https://github.com/windmill-labs/windmill/issues/2463)) ([ef3e4b2](https://github.com/windmill-labs/windmill/commit/ef3e4b2623d07e605d0507983f7976ec6656b4f6))
* test openai key + improve AI UI ([#2465](https://github.com/windmill-labs/windmill/issues/2465)) ([94a52f1](https://github.com/windmill-labs/windmill/commit/94a52f1d2de14e78485a5429e56afaa7b9628199))
* timeline for apps ([#2470](https://github.com/windmill-labs/windmill/issues/2470)) ([5469321](https://github.com/windmill-labs/windmill/commit/54693210229c01ed27d4ad2c4ca362b78292ad88))


### Bug Fixes

* embeddings duplicate id ([#2467](https://github.com/windmill-labs/windmill/issues/2467)) ([666ac56](https://github.com/windmill-labs/windmill/commit/666ac56824dead5dd1e44d7960de3c492096b445))
* hub scripts search ([#2469](https://github.com/windmill-labs/windmill/issues/2469)) ([3a03eb3](https://github.com/windmill-labs/windmill/commit/3a03eb37606ae569674f5e77a72f638f560c4c60))

## [1.185.0](https://github.com/windmill-labs/windmill/compare/v1.184.0...v1.185.0) (2023-10-19)


### Features

* add timeline progress bars to flows" ([#2464](https://github.com/windmill-labs/windmill/issues/2464)) ([d96f8d0](https://github.com/windmill-labs/windmill/commit/d96f8d0d41540053cb0e65b643c3ce3e1f43a095))


### Bug Fixes

* add select tabs as list inputs for apps ([39e612e](https://github.com/windmill-labs/windmill/commit/39e612e11601ab0aea26ccf30ad45b6452c127ce))
* fix handling of undefined values in input transforms by serde ([acbe129](https://github.com/windmill-labs/windmill/commit/acbe1298fc01dd3264e3533277a1c837e3b2961b))
* fix mocking for workflows ([f9045dc](https://github.com/windmill-labs/windmill/commit/f9045dc70f42df53222ccfc58599e32b8e2487aa))
* go-client build dependencies ([#2460](https://github.com/windmill-labs/windmill/issues/2460)) ([92c0ab2](https://github.com/windmill-labs/windmill/commit/92c0ab21b7f22626dbed02017fddc11e8c093186))

## [1.184.0](https://github.com/windmill-labs/windmill/compare/v1.183.0...v1.184.0) (2023-10-17)


### Features

* filter resource types passed to gpt-4 ([#2430](https://github.com/windmill-labs/windmill/issues/2430)) ([e20889b](https://github.com/windmill-labs/windmill/commit/e20889b910b0e5c72d9e04eedc59b650a2550dce))
* **frontend:** app editor tutorials ([#2443](https://github.com/windmill-labs/windmill/issues/2443)) ([aaf8385](https://github.com/windmill-labs/windmill/commit/aaf83859bd268e8bf8ecb382c2b39a92ddb40967))
* Improve Slack schedule error handler and default to it ([#2439](https://github.com/windmill-labs/windmill/issues/2439)) ([a1d6799](https://github.com/windmill-labs/windmill/commit/a1d6799625ae40c5f88615007d01f11b55a0add4))
* Mute workspace error handler for flows and scripts ([#2458](https://github.com/windmill-labs/windmill/issues/2458)) ([2dc75f0](https://github.com/windmill-labs/windmill/commit/2dc75f0f6528ecd884d93ad749dae28efa249f06))
* refactor entirely json processing in favor or rawjson to handle larger payloads ([#2446](https://github.com/windmill-labs/windmill/issues/2446)) ([9314d38](https://github.com/windmill-labs/windmill/commit/9314d38bf1da6247b367effe69394f25a27067ca))
* Workspace error handler now supports args and Slack for EE ([#2447](https://github.com/windmill-labs/windmill/issues/2447)) ([f7cc773](https://github.com/windmill-labs/windmill/commit/f7cc77382652a41e27fefc2b988e034447881bcb))


### Bug Fixes

* Error handler script pickers lists both "regular" script and "error handler" scripts ([#2449](https://github.com/windmill-labs/windmill/issues/2449)) ([8a3537b](https://github.com/windmill-labs/windmill/commit/8a3537b76124b67d0aa163f8dcc55f1db0f4f56d))
* fix previous ids for iterators and branches ([8d89605](https://github.com/windmill-labs/windmill/commit/8d89605bc6c1fcda0ae3d2d37353f7c76ed18ff6))
* **frontend:** fix forloop tutorial ([#2444](https://github.com/windmill-labs/windmill/issues/2444)) ([26371fd](https://github.com/windmill-labs/windmill/commit/26371fde0c35d508af82e9951bbe2fc74e4235ff))
* **frontend:** fix style panel overflow ([#2437](https://github.com/windmill-labs/windmill/issues/2437)) ([0ce4b34](https://github.com/windmill-labs/windmill/commit/0ce4b344818b4f25533224f3c6b5b6e99e823110))
* **frontend:** simplify flow tutorials ([#2448](https://github.com/windmill-labs/windmill/issues/2448)) ([0c2004f](https://github.com/windmill-labs/windmill/commit/0c2004f5adff2a2752255cbc1fa5f1a4a82b177d))
* Slack token is readable by g/error_handler ([#2454](https://github.com/windmill-labs/windmill/issues/2454)) ([f9e48dd](https://github.com/windmill-labs/windmill/commit/f9e48ddcba3d776cca263219a229b02c95ef9abb))
* update bun to 1.0.5 ([a84ce44](https://github.com/windmill-labs/windmill/commit/a84ce44cd9a7ecf2baf8388a43e362fed875c1a1))
* update bun to 1.0.6 ([e770f25](https://github.com/windmill-labs/windmill/commit/e770f25667229189acedc25fba685a43c827537b))
* Workspace error handler extra args are passed to job ([#2452](https://github.com/windmill-labs/windmill/issues/2452)) ([b7ce7f0](https://github.com/windmill-labs/windmill/commit/b7ce7f0b18537836f16fa3fefcdd80b622b51665))

## [1.183.0](https://github.com/windmill-labs/windmill/compare/v1.182.3...v1.183.0) (2023-10-11)


### Features

* **frontend:** Table wizard ([#2416](https://github.com/windmill-labs/windmill/issues/2416)) ([6f0cda0](https://github.com/windmill-labs/windmill/commit/6f0cda0e1ea84e2b5c5d297c841749dc5bae879d))


### Bug Fixes

* benchmark config syntax error ([#2432](https://github.com/windmill-labs/windmill/issues/2432)) ([109c2f1](https://github.com/windmill-labs/windmill/commit/109c2f17d68e0cac2f365297cc2fcdd54d9d105a))
* **frontend:** add a validation for base url ([#2434](https://github.com/windmill-labs/windmill/issues/2434)) ([c914ac6](https://github.com/windmill-labs/windmill/commit/c914ac64cfbaacaf5fe3c7486ea9901ce4828387))
* **frontend:** fix drawer title truncate ([#2429](https://github.com/windmill-labs/windmill/issues/2429)) ([46d2c13](https://github.com/windmill-labs/windmill/commit/46d2c13e0d2dde1e87c3bbe7cc2be29de84fa2cf))
* **frontend:** fix mobile multi select ([#2431](https://github.com/windmill-labs/windmill/issues/2431)) ([cb2b6df](https://github.com/windmill-labs/windmill/commit/cb2b6dfdba8953a3d1f432e4af2b2725f5e267ca))
* **frontend:** fix table wizards for old apps ([#2435](https://github.com/windmill-labs/windmill/issues/2435)) ([e088ec5](https://github.com/windmill-labs/windmill/commit/e088ec566958079e468b3c1f5df057f6e70dffc3))

## [1.182.3](https://github.com/windmill-labs/windmill/compare/v1.182.2...v1.182.3) (2023-10-10)


### Bug Fixes

* improve binary build ([094539f](https://github.com/windmill-labs/windmill/commit/094539ff3aa79531953f82941337bdd3d34db630))

## [1.182.2](https://github.com/windmill-labs/windmill/compare/v1.182.1...v1.182.2) (2023-10-10)


### Bug Fixes

* add binaries to release ([17b42e6](https://github.com/windmill-labs/windmill/commit/17b42e6a3555ae1f45d8f24934f290a72e3d60c5))

## [1.182.1](https://github.com/windmill-labs/windmill/compare/v1.182.0...v1.182.1) (2023-10-10)


### Bug Fixes

* Small fixes UI & Slack OAuth tuto ([#2398](https://github.com/windmill-labs/windmill/issues/2398)) ([e1eccc2](https://github.com/windmill-labs/windmill/commit/e1eccc2d9331ba4e33019a6109bc0368d718397c))

## [1.182.0](https://github.com/windmill-labs/windmill/compare/v1.181.0...v1.182.0) (2023-10-10)


### Features

* add support for aggrid ee ([c4a817a](https://github.com/windmill-labs/windmill/commit/c4a817aeb6590d8972342f815f3cf3b891ea1446))
* **frontend:** App polish ([#2397](https://github.com/windmill-labs/windmill/issues/2397)) ([11e0bc7](https://github.com/windmill-labs/windmill/commit/11e0bc76c4bc80339e43590f5becf6b2442a2227))
* **frontend:** column definition helper ([#2399](https://github.com/windmill-labs/windmill/issues/2399)) ([53447f1](https://github.com/windmill-labs/windmill/commit/53447f1b43e897bb8856106cabc502c822052441))
* **frontend:** error handler tutorial ([#2404](https://github.com/windmill-labs/windmill/issues/2404)) ([bc1ad3b](https://github.com/windmill-labs/windmill/commit/bc1ad3b8d09fb2b6547dbcb37ac074ffdf9b383c))
* **frontend:** fix css editor + fix dark mode ([#2409](https://github.com/windmill-labs/windmill/issues/2409)) ([2d7712c](https://github.com/windmill-labs/windmill/commit/2d7712c02115006fe84cb323b3b3af99ac14ffdb))
* manage cache and init scripts from worker group UI ([#2396](https://github.com/windmill-labs/windmill/issues/2396)) ([2c9ae41](https://github.com/windmill-labs/windmill/commit/2c9ae41706edc6570559d7d83864fb05c846c0c1))


### Bug Fixes

* add lsp absolute imports for deno in all cases ([27c45e3](https://github.com/windmill-labs/windmill/commit/27c45e38cc57350df193440aa0c09ddbca93902a))
* fix aggrid initialization ([9b75e33](https://github.com/windmill-labs/windmill/commit/9b75e33887c3a9c4cac84d648763a6e3b4490fae))
* **frontend:** Fix tutorial trigger ([#2392](https://github.com/windmill-labs/windmill/issues/2392)) ([cad37bc](https://github.com/windmill-labs/windmill/commit/cad37bc6defa7a42b96fec6ad0a9bcac55d88d51))
* improve flow status viewer for large values ([64c5590](https://github.com/windmill-labs/windmill/commit/64c5590aa32e4dbff6af43e711cb6899c02e4ee3))
* improve handling of large results by frontend ([21454a7](https://github.com/windmill-labs/windmill/commit/21454a7a052db3cc1d24fd36c4504098751c66d2))
* tarball for workspace export is generated in /tmp/windmill ([f4957d6](https://github.com/windmill-labs/windmill/commit/f4957d66b9bf6124ad3f73912f32cd1ea47b46e2))

## [1.181.0](https://github.com/windmill-labs/windmill/compare/v1.180.0...v1.181.0) (2023-10-05)


### Features

* add npm_config_registry support for bun, deno and being settable from UI ([#2373](https://github.com/windmill-labs/windmill/issues/2373)) ([c42b875](https://github.com/windmill-labs/windmill/commit/c42b8750f1d41c9b4de6c96f1ea82239c5325495))
* **frontend:** add driverjs ([#2327](https://github.com/windmill-labs/windmill/issues/2327)) ([bda6f1f](https://github.com/windmill-labs/windmill/commit/bda6f1fe5d44a3c1d925c1b8a8e872d9f5fba484))


### Bug Fixes

* add numeric, array and date types ([#2379](https://github.com/windmill-labs/windmill/issues/2379)) ([768f972](https://github.com/windmill-labs/windmill/commit/768f972cbf578b3394f89120d172b02bcaac5413))
* add reserved variables in args ([#2371](https://github.com/windmill-labs/windmill/issues/2371)) ([e7165f3](https://github.com/windmill-labs/windmill/commit/e7165f3357a2ba7a690accd78a03c2518aa61860))
* ai flow prompt fix + explanation in ui ([#2374](https://github.com/windmill-labs/windmill/issues/2374)) ([66d15f0](https://github.com/windmill-labs/windmill/commit/66d15f0c17698077c5bf299af8368e9cfdbf3ecb))
* flow trigger prompt + lower temp ([#2377](https://github.com/windmill-labs/windmill/issues/2377)) ([733bfe3](https://github.com/windmill-labs/windmill/commit/733bfe3f14e6eb0237c6a528ab64ae71082a4679))
* **frontend:** fix flow tutorials ([#2383](https://github.com/windmill-labs/windmill/issues/2383)) ([63ad53f](https://github.com/windmill-labs/windmill/commit/63ad53fa70c4f1769d873bff962bfb2d66081163))
* schema autocomplete/ai ([#2372](https://github.com/windmill-labs/windmill/issues/2372)) ([9ed748a](https://github.com/windmill-labs/windmill/commit/9ed748a0dac95f152f91de6e25b63d841af0dd50))
* trigger bun prompt ([#2368](https://github.com/windmill-labs/windmill/issues/2368)) ([fc9adbe](https://github.com/windmill-labs/windmill/commit/fc9adbe56081065fa3de662e664fcebe0f4c25ee))

## [1.180.0](https://github.com/windmill-labs/windmill/compare/v1.179.1...v1.180.0) (2023-10-01)


### Features

* code content search ([#2367](https://github.com/windmill-labs/windmill/issues/2367)) ([fb96059](https://github.com/windmill-labs/windmill/commit/fb960594fce265d5d4f4eb443e0c9cc19d14e025))


### Bug Fixes

* improve connection in apps ([a2fca17](https://github.com/windmill-labs/windmill/commit/a2fca17ae2ac8257154e2aec4a0ceabfe16fc46a))

## [1.179.1](https://github.com/windmill-labs/windmill/compare/v1.179.0...v1.179.1) (2023-09-30)


### Bug Fixes

* fix 0 len flow module processing ([f97289a](https://github.com/windmill-labs/windmill/commit/f97289a3d8bc6ce978d0be1fec35a424211e4a20))

## [1.179.0](https://github.com/windmill-labs/windmill/compare/v1.178.1...v1.179.0) (2023-09-30)


### Features

* add trustedDependencies escape hatch for bun ([#2364](https://github.com/windmill-labs/windmill/issues/2364)) ([52df265](https://github.com/windmill-labs/windmill/commit/52df2650ea5d5c03e94c96af0b8a79275856fc37))
* ai code completion ([#2361](https://github.com/windmill-labs/windmill/issues/2361)) ([0937706](https://github.com/windmill-labs/windmill/commit/093770692ac40b8ee0139f24d63bcccda9bf6ddb))
* **backend:** parse expires_in from string in TokenResponse ([#2353](https://github.com/windmill-labs/windmill/issues/2353)) ([4621915](https://github.com/windmill-labs/windmill/commit/46219154de07ef5a6e071f1c2859cea35c7f9943))
* **frontend:** copy schema from json and past runs in flow inputs ([#2352](https://github.com/windmill-labs/windmill/issues/2352)) ([3cb2977](https://github.com/windmill-labs/windmill/commit/3cb29778dd70199d9504aa7c1a12bfd7a02569d6))


### Bug Fixes

* error handler does not recover flow anymore and error handler is called only once up the flow ([445bf96](https://github.com/windmill-labs/windmill/commit/445bf965eddc6da39a125fce60b53e0903698664))
* **frontend:** Properly handle click ([#2351](https://github.com/windmill-labs/windmill/issues/2351)) ([55b7f98](https://github.com/windmill-labs/windmill/commit/55b7f982c2bbbb5d4daa9752ec8ffc0c79c374fc))
* **frontend:** timezone fix ([#2360](https://github.com/windmill-labs/windmill/issues/2360)) ([dcfa5fc](https://github.com/windmill-labs/windmill/commit/dcfa5fc0e40f5cd8dba5a26be31695ce765c7e23))
* improve superadmin settings page ([b029027](https://github.com/windmill-labs/windmill/commit/b029027c1c75c0b6489966371db7d2f9c99d15f8))
* non skipped failures stop even in presence of an error handler ([1c5cc0c](https://github.com/windmill-labs/windmill/commit/1c5cc0c237101caf6c5e6e34b11c967a27cd4112))
* remove shared http clients in rest runtime ([4931ed9](https://github.com/windmill-labs/windmill/commit/4931ed95c4b12f63effa1dd7d6a5cd526a612302))

## [1.178.1](https://github.com/windmill-labs/windmill/compare/v1.178.0...v1.178.1) (2023-09-28)


### Bug Fixes

* improve license key check ([035bad5](https://github.com/windmill-labs/windmill/commit/035bad5268d182af3f30915b5356defd7f6ccbc0))

## [1.178.0](https://github.com/windmill-labs/windmill/compare/v1.177.1...v1.178.0) (2023-09-28)


### Features

* **frontend:** add app groups management ([#2347](https://github.com/windmill-labs/windmill/issues/2347)) ([20e0427](https://github.com/windmill-labs/windmill/commit/20e0427a1303c1c32f41b198cd2d0f7f28b5bd32))
* **frontend:** add AppDrawer controls ([#2339](https://github.com/windmill-labs/windmill/issues/2339)) ([3de6d44](https://github.com/windmill-labs/windmill/commit/3de6d446f281dcaac288deee19342a08e0ccf9af))
* **frontend:** Switch to component list when deleting a component ([#2346](https://github.com/windmill-labs/windmill/issues/2346)) ([6fcd72c](https://github.com/windmill-labs/windmill/commit/6fcd72c79453dd9d60ca869cd9996cc0c25971fa))


### Bug Fixes

* add env tags to default worker group ([#2348](https://github.com/windmill-labs/windmill/issues/2348)) ([f5bed95](https://github.com/windmill-labs/windmill/commit/f5bed95ab15bc397f822b06816c43b4b13a84af3))

## [1.177.1](https://github.com/windmill-labs/windmill/compare/v1.177.0...v1.177.1) (2023-09-26)


### Bug Fixes

* **frontend:** fix modal closing issues ([#2340](https://github.com/windmill-labs/windmill/issues/2340)) ([18cf8fa](https://github.com/windmill-labs/windmill/commit/18cf8faec16d496e4b327505b682459ed518a5b4))
* **frontend:** fix overflow ([#2341](https://github.com/windmill-labs/windmill/issues/2341)) ([2e8f2ec](https://github.com/windmill-labs/windmill/commit/2e8f2ec724f6802170121f4f8aa73b697a39c9ee))
* improve list component handling of non array data ([dc44b08](https://github.com/windmill-labs/windmill/commit/dc44b0841af17227160b9d56ec446e6646a8ab0d))

## [1.177.0](https://github.com/windmill-labs/windmill/compare/v1.176.0...v1.177.0) (2023-09-26)


### Features

* add custom oauth support ([#2336](https://github.com/windmill-labs/windmill/issues/2336)) ([01277f4](https://github.com/windmill-labs/windmill/commit/01277f4d3b8bb04b955d5bbb2ed69c1c7c8f4f9e))
* support automatic reconnection to pg ([ccaa05d](https://github.com/windmill-labs/windmill/commit/ccaa05d4bf5954c3fb8678239d2962cac6550a5a))


### Bug Fixes

* fix resource type picker object reinitialization ([f0f15c4](https://github.com/windmill-labs/windmill/commit/f0f15c47cb35cc1e3cfa13549465803a1e970770))
* **frontend:** Fix build ([#2330](https://github.com/windmill-labs/windmill/issues/2330)) ([46592af](https://github.com/windmill-labs/windmill/commit/46592affd3d51b54632a2a7a281c11141edcb4a5))
* **frontend:** Fix markdown dark mode ([#2329](https://github.com/windmill-labs/windmill/issues/2329)) ([6c19740](https://github.com/windmill-labs/windmill/commit/6c197407185810f43c47d4107007bd69814a1d65))
* set min size of components to 1 ([d298093](https://github.com/windmill-labs/windmill/commit/d298093e29bd9983c7631a8f8c80e47b768bb93c))

## [1.176.0](https://github.com/windmill-labs/windmill/compare/v1.175.0...v1.176.0) (2023-09-24)


### Features

* add license key as superadmin setting ([#2321](https://github.com/windmill-labs/windmill/issues/2321)) ([304a259](https://github.com/windmill-labs/windmill/commit/304a2596fd29fbd9a79c5cf9fe4df7b44d5c5254))
* add running filter ([ea364ad](https://github.com/windmill-labs/windmill/commit/ea364ad9602647cbc9e8ee78fb5f17f0012105f6))
* ai flow trigger menu ([#2317](https://github.com/windmill-labs/windmill/issues/2317)) ([95194ab](https://github.com/windmill-labs/windmill/commit/95194abeacc42416174ee9dd79b75f2204a40d33))
* improved dedicated benchmarks + buffer fix ([#2313](https://github.com/windmill-labs/windmill/issues/2313)) ([fc93c2a](https://github.com/windmill-labs/windmill/commit/fc93c2a7cece95c00070a3a3391ae2bcb4513e85))
* set instance settings from UI ([#2314](https://github.com/windmill-labs/windmill/issues/2314)) ([2f0e43b](https://github.com/windmill-labs/windmill/commit/2f0e43bfdbd1e196131f126c83b1d7dd2eea98d8))


### Bug Fixes

* add ability to test this step for flow step ([3585929](https://github.com/windmill-labs/windmill/commit/3585929bb758b0cfc2cbe43f66597b184e7b8ee0))
* benchmark worker tags ([#2319](https://github.com/windmill-labs/windmill/issues/2319)) ([481bcd5](https://github.com/windmill-labs/windmill/commit/481bcd53cb07e4520d5fd81572cad74340c4eb64))
* change cache implementation to remove async-timer ([4911b4b](https://github.com/windmill-labs/windmill/commit/4911b4b3fd6e3a9f6bccc4c8712b736e18dcb6e1))
* fix upto preview issue with nested flows ([6492ff6](https://github.com/windmill-labs/windmill/commit/6492ff627a800832e12a31fd89a6070703988eb9))
* flow steps appears in all static inputs ([c043847](https://github.com/windmill-labs/windmill/commit/c0438479aa3b6dc6349df01abdd9dcc434fe8781))
* optimize performance for bun scripts without deps ([5b33f56](https://github.com/windmill-labs/windmill/commit/5b33f563e6e83605ae72338af351dcc97beb1a55))
* overflow on workspace script picker ([5e4db0e](https://github.com/windmill-labs/windmill/commit/5e4db0ebab616305928cfa455af6833335e0fcf9))
* tag id as flow ([#2318](https://github.com/windmill-labs/windmill/issues/2318)) ([f68cee4](https://github.com/windmill-labs/windmill/commit/f68cee4ebddbf6e774f80e91a8c89fb8dc213f91))

## [1.175.0](https://github.com/windmill-labs/windmill/compare/v1.174.0...v1.175.0) (2023-09-19)


### Features

* add batch jobs ([#2306](https://github.com/windmill-labs/windmill/issues/2306)) ([5867e5d](https://github.com/windmill-labs/windmill/commit/5867e5d0f80fd515fab165659831b5ee9a8c3f97))
* add dediacted worker env var ([#2296](https://github.com/windmill-labs/windmill/issues/2296)) ([e0c6eee](https://github.com/windmill-labs/windmill/commit/e0c6eee16e535b3a7d803a7978e463404f5fec30))
* dedicated benchmarks ([#2297](https://github.com/windmill-labs/windmill/issues/2297)) ([c549239](https://github.com/windmill-labs/windmill/commit/c5492396843ddd9143ffe890696d0317c970de36))
* **frontend:** Add component control doc ([#2295](https://github.com/windmill-labs/windmill/issues/2295)) ([26f8863](https://github.com/windmill-labs/windmill/commit/26f88636f0b972d4fe4931ed02135c38b27a56d2))
* suggest adding openai key on workspace creation ([a6b3b2f](https://github.com/windmill-labs/windmill/commit/a6b3b2f63b317825a3d80218cbb606b9f610c221))
* support pinned versions for bun in deployed scripts ([03806dc](https://github.com/windmill-labs/windmill/commit/03806dc3907cba724be14acb6aadf5be6e35cdb6))


### Bug Fixes

* add HOME to bun and deno ([0e3ecc7](https://github.com/windmill-labs/windmill/commit/0e3ecc7d6025c173135f20bacc33a0dc972ec222))
* add queue_count to metrics ([9ced883](https://github.com/windmill-labs/windmill/commit/9ced8834a45151c6900b1eb33eca2cff4886a065))
* ai improve prompts ([#2310](https://github.com/windmill-labs/windmill/issues/2310)) ([b647213](https://github.com/windmill-labs/windmill/commit/b647213b2c968b0cb1f90c97d94e8023c415dd55))
* **frontend:** add missing key ([#2299](https://github.com/windmill-labs/windmill/issues/2299)) ([39d2467](https://github.com/windmill-labs/windmill/commit/39d24672ddd696372e55e9b4566f322a322385a8))
* **frontend:** Always mount components ([#2309](https://github.com/windmill-labs/windmill/issues/2309)) ([34f94aa](https://github.com/windmill-labs/windmill/commit/34f94aa50e92254114c046fa8b7e900d93807937))
* **frontend:** fix alignment ([#2307](https://github.com/windmill-labs/windmill/issues/2307)) ([f9fc6f1](https://github.com/windmill-labs/windmill/commit/f9fc6f19482e68c9ccba0014879fd8761662c36a))
* **frontend:** Fix rich result styling + add title and hideDetails config ([#2294](https://github.com/windmill-labs/windmill/issues/2294)) ([732daef](https://github.com/windmill-labs/windmill/commit/732daef1c3515f7df3e09deac691bb585f9859cd))
* **frontend:** fix tab styling + component bg ([#2308](https://github.com/windmill-labs/windmill/issues/2308)) ([5e773d3](https://github.com/windmill-labs/windmill/commit/5e773d386343f003425173207c166e3c4eeef956))
* **frontend:** fix theme make default ([#2304](https://github.com/windmill-labs/windmill/issues/2304)) ([4629819](https://github.com/windmill-labs/windmill/commit/46298197c5333a81b9b8a004027ab9a856bdada4))
* **frontend:** fix theme UI ([#2305](https://github.com/windmill-labs/windmill/issues/2305)) ([576f76b](https://github.com/windmill-labs/windmill/commit/576f76b1ffe9c50c8ccaca8c5e34d0ec03aebf3f))
* validate more strongly usernames ([47094bb](https://github.com/windmill-labs/windmill/commit/47094bb8d1c6f4ba621d42515dede061fd04afdd))

## [1.174.0](https://github.com/windmill-labs/windmill/compare/v1.173.0...v1.174.0) (2023-09-15)


### Features

* ai gen support all langs ([#2276](https://github.com/windmill-labs/windmill/issues/2276)) ([39590b3](https://github.com/windmill-labs/windmill/commit/39590b3d2592b2d08117c0f70829c13f1efb4885))
* bun absolute/relative imports + tests ([#2286](https://github.com/windmill-labs/windmill/issues/2286)) ([e5ce85b](https://github.com/windmill-labs/windmill/commit/e5ce85b9affe665342f24b1d39ce3d03db09b941))
* **frontend:** Global CSS editor ([#2178](https://github.com/windmill-labs/windmill/issues/2178)) ([7e9ee39](https://github.com/windmill-labs/windmill/commit/7e9ee39aa69bc31766b5e4f4aab498c8f14067cd))

## [1.173.0](https://github.com/windmill-labs/windmill/compare/v1.172.1...v1.173.0) (2023-09-14)


### Features

* cli sync on windows ([#2283](https://github.com/windmill-labs/windmill/issues/2283)) ([c371cb3](https://github.com/windmill-labs/windmill/commit/c371cb397ab3d0c534e2c553d1dfb1ad5176d2a6))


### Bug Fixes

* accept jobs whose duration &gt; 24 days ([2c00894](https://github.com/windmill-labs/windmill/commit/2c00894122aa8caee59b20625935284de6902950))

## [1.172.1](https://github.com/windmill-labs/windmill/compare/v1.172.0...v1.172.1) (2023-09-14)


### Bug Fixes

* improve splitpane + improve deleting conditional tab ([1629008](https://github.com/windmill-labs/windmill/commit/1629008eb2eb48ff9cc2cf6b3a351efcf682244d))
* update to svelte 4 ([#2280](https://github.com/windmill-labs/windmill/issues/2280)) ([90c10d8](https://github.com/windmill-labs/windmill/commit/90c10d803b4c47a9e1ac5b9e49e2a614344299a9))

## [1.172.0](https://github.com/windmill-labs/windmill/compare/v1.171.0...v1.172.0) (2023-09-13)


### Features

* improve ai flow ([#2270](https://github.com/windmill-labs/windmill/issues/2270)) ([b23417a](https://github.com/windmill-labs/windmill/commit/b23417ab5b9938bbdf9db6449102760ff8c80152))
* worker groups admin panel ([#2277](https://github.com/windmill-labs/windmill/issues/2277)) ([070b162](https://github.com/windmill-labs/windmill/commit/070b16222bc666866284180b3878f4d4f27bfa85))


### Bug Fixes

* ai flow nits ([#2272](https://github.com/windmill-labs/windmill/issues/2272)) ([8f6f46d](https://github.com/windmill-labs/windmill/commit/8f6f46de199d58133b9faa77cdbcbcfd6cb962f7))

## [1.171.0](https://github.com/windmill-labs/windmill/compare/v1.170.0...v1.171.0) (2023-09-12)


### Features

* attempt to SIGTERM before SIGKILL for bash ([f40bbba](https://github.com/windmill-labs/windmill/commit/f40bbba519a97cbb1ec142c335f038dbebcd4e7c))
* zero copy result for job result ([#2263](https://github.com/windmill-labs/windmill/issues/2263)) ([22a7da5](https://github.com/windmill-labs/windmill/commit/22a7da58b1d20721892906cba2dee6fbeb1cc1fd))


### Bug Fixes

* 2257 TIME  convertion in pg_executor.rs ([#2267](https://github.com/windmill-labs/windmill/issues/2267)) ([3d71253](https://github.com/windmill-labs/windmill/commit/3d71253abdb0dff1670a796d07a53ecd0a98414e))
* fix field duplicate in app background settings ([164cdaf](https://github.com/windmill-labs/windmill/commit/164cdaf09464646dee4e70a699222a454eb0d898))
* improve bun lockfile resolution ([9103ec4](https://github.com/windmill-labs/windmill/commit/9103ec445db81395a5851202eecb87301d0b4987))
* remove result and args from list completed and list queue jobs ([e7e63e1](https://github.com/windmill-labs/windmill/commit/e7e63e111a73e0986050a8fe7fdc18784ba902b0))

## [1.170.0](https://github.com/windmill-labs/windmill/compare/v1.169.0...v1.170.0) (2023-09-08)


### Features

* display jobs currently waiting for a worker ([3c950c0](https://github.com/windmill-labs/windmill/commit/3c950c03de0bc71974eb29985381adba8c098660))
* snowflake schema explorer + refactoring ([#2260](https://github.com/windmill-labs/windmill/issues/2260)) ([5cca583](https://github.com/windmill-labs/windmill/commit/5cca5833e94fc4c8a80e210164da09f2a1ceb677))


### Bug Fixes

* fix get_result for python-client ([fe41f4f](https://github.com/windmill-labs/windmill/commit/fe41f4ff4ce596cf394bd69a0ba48e88db8d2328))

## [1.169.0](https://github.com/windmill-labs/windmill/compare/v1.168.3...v1.169.0) (2023-09-08)


### Features

* benchmarks graph ([#2244](https://github.com/windmill-labs/windmill/issues/2244)) ([c496602](https://github.com/windmill-labs/windmill/commit/c496602e9e2e0dfecaaffe731e58e551d039d02f))
* big query schema explorer ([#2247](https://github.com/windmill-labs/windmill/issues/2247)) ([ec7d923](https://github.com/windmill-labs/windmill/commit/ec7d923cca0f6050855473ababd1bb27d668711b))
* flow copilot ([#2219](https://github.com/windmill-labs/windmill/issues/2219)) ([2f3138c](https://github.com/windmill-labs/windmill/commit/2f3138c65d9d3f0161bf3e069c6eec0c32ac3b86))
* **frontend:** fix runs page when the row has a parent ([#2255](https://github.com/windmill-labs/windmill/issues/2255)) ([2271263](https://github.com/windmill-labs/windmill/commit/22712632f683fb63ad6d4b475a01c63800a9559d))
* introduce container groups ([49c5553](https://github.com/windmill-labs/windmill/commit/49c5553f3b496c2aaf03376689ee0fd42ecbd2bf))


### Bug Fixes

* benchmark svg ([#2249](https://github.com/windmill-labs/windmill/issues/2249)) ([24c5802](https://github.com/windmill-labs/windmill/commit/24c580211572d6447ca502db141e90c5e084d790))
* pass TZ from env to runtimes ([75a1490](https://github.com/windmill-labs/windmill/commit/75a149009a5a13230b4d6de6eac8bba0618629d6))

## [1.168.3](https://github.com/windmill-labs/windmill/compare/v1.168.2...v1.168.3) (2023-09-07)


### Bug Fixes

* add list resource types names ([fbbab5c](https://github.com/windmill-labs/windmill/commit/fbbab5c874748547a9ff3e58c1b7b22c90766f4f))
* add stable ids to rows in AppTable ([0c91581](https://github.com/windmill-labs/windmill/commit/0c91581fcdf3a141f36e34610935aa100fcfee52))
* reduce aggregate period to list users in workspace ([6bc0e37](https://github.com/windmill-labs/windmill/commit/6bc0e373fc6088636f09d217e8800a32337291ea))

## [1.168.2](https://github.com/windmill-labs/windmill/compare/v1.168.1...v1.168.2) (2023-09-06)


### Bug Fixes

* fix sqlx build ([64e7fb5](https://github.com/windmill-labs/windmill/commit/64e7fb56e41b45bc2476d0e98fa99dcbc355cfe0))

## [1.168.1](https://github.com/windmill-labs/windmill/compare/v1.168.0...v1.168.1) (2023-09-06)


### Bug Fixes

* fix sqlx build ([92c8146](https://github.com/windmill-labs/windmill/commit/92c8146a5778290b5a76c2ea5685f95b85be2e38))

## [1.168.0](https://github.com/windmill-labs/windmill/compare/v1.167.0...v1.168.0) (2023-09-06)


### Features

* dedicated workers for native-throughput performance  (EE only)  ([#2239](https://github.com/windmill-labs/windmill/issues/2239)) ([c80f155](https://github.com/windmill-labs/windmill/commit/c80f155602eca972842be7bd560395a06e4e0ae6))


### Bug Fixes

* **frontend:** add virtual list ([#2218](https://github.com/windmill-labs/windmill/issues/2218)) ([e4c896b](https://github.com/windmill-labs/windmill/commit/e4c896b4b9f28b2fa219be249a2794faf3f1b7d0))

## [1.167.1](https://github.com/windmill-labs/windmill/compare/v1.167.0...v1.167.1) (2023-09-05)


### Bug Fixes

* **frontend:** add virtual list ([#2218](https://github.com/windmill-labs/windmill/issues/2218)) ([e4c896b](https://github.com/windmill-labs/windmill/commit/e4c896b4b9f28b2fa219be249a2794faf3f1b7d0))

## [1.167.0](https://github.com/windmill-labs/windmill/compare/v1.166.1...v1.167.0) (2023-09-04)


### Features

* submit result in background thread (unify architecture for dedicated worker) ([#2226](https://github.com/windmill-labs/windmill/issues/2226)) ([dff1cd9](https://github.com/windmill-labs/windmill/commit/dff1cd9a64f755f239eb57599c104c47f4d33b12))


### Bug Fixes

* **cli:** prioritize correctly content file to resolve for ts types ([2906d53](https://github.com/windmill-labs/windmill/commit/2906d535a126f4fe2cfe6dffda46e5fe841056da))

## [1.166.1](https://github.com/windmill-labs/windmill/compare/v1.166.0...v1.166.1) (2023-09-03)


### Bug Fixes

* fix setting is ready for s3 workers ([b0ed0f9](https://github.com/windmill-labs/windmill/commit/b0ed0f964843247d11ecfe586f1565589df95ff6))

## [1.166.0](https://github.com/windmill-labs/windmill/compare/v1.165.0...v1.166.0) (2023-09-03)


### Features

* **frontend:** App stepper debug ([#2202](https://github.com/windmill-labs/windmill/issues/2202)) ([77f8eac](https://github.com/windmill-labs/windmill/commit/77f8eac21e0edfa1eada617d78a498a3a6ae1dce))


### Bug Fixes

* fix datetime handling for python ([b35ffd4](https://github.com/windmill-labs/windmill/commit/b35ffd435de97ed34fcda69490abd734ea3229fa))
* **frontend:** Fix App Modal z-index ([#2210](https://github.com/windmill-labs/windmill/issues/2210)) ([9787edb](https://github.com/windmill-labs/windmill/commit/9787edb67c329265bf179fe304d00cdc1df7042e))
* see run detail in a new tab ([719a7b1](https://github.com/windmill-labs/windmill/commit/719a7b11da81f68452ba9fc22ff456fe1ddde1de))
* update wmill python generator thus updating windmill-api ([f912f1d](https://github.com/windmill-labs/windmill/commit/f912f1de86e91c5cdbc0012e2362467c4965936a))


### Performance Improvements

* improve queue performance ([#2222](https://github.com/windmill-labs/windmill/issues/2222)) ([069e2d1](https://github.com/windmill-labs/windmill/commit/069e2d18d586aa3d407e3b089d1ad94b2b838af0))

## [1.165.0](https://github.com/windmill-labs/windmill/compare/v1.164.0...v1.165.0) (2023-08-31)


### Features

* improve queue performance when queue grows large ([ada88a2](https://github.com/windmill-labs/windmill/commit/ada88a2bf94fec71187bbdb210065de43d4cd3fb))
* support partial go dependency pinning ([41107c7](https://github.com/windmill-labs/windmill/commit/41107c7cfa7b56099a9c8b08cfb16ff3cf840ff2))


### Bug Fixes

* uniformize that all job links specify the workspace ([d311d76](https://github.com/windmill-labs/windmill/commit/d311d76557432a72a5d6d7ab010aeb1fe0e599de))

## [1.164.0](https://github.com/windmill-labs/windmill/compare/v1.163.1...v1.164.0) (2023-08-31)


### Features

* add workspace variable to worker tag ([276cd6d](https://github.com/windmill-labs/windmill/commit/276cd6dac39b7cb181ac46e3edea79a3a3bcff8d))


### Bug Fixes

* **frontend:** allow using Docker in Flow ([#2201](https://github.com/windmill-labs/windmill/issues/2201)) ([bb749c1](https://github.com/windmill-labs/windmill/commit/bb749c14f877f7cb1e8642b881a00aedfeb08f7d))

## [1.163.1](https://github.com/windmill-labs/windmill/compare/v1.163.0...v1.163.1) (2023-08-30)


### Bug Fixes

* avoid perpetual spinning of recompute all component ([11e1ecb](https://github.com/windmill-labs/windmill/commit/11e1ecbcda92f5ab643b776094ef10005d51b579))

## [1.163.0](https://github.com/windmill-labs/windmill/compare/v1.162.2...v1.163.0) (2023-08-30)


### Features

* add global cache configuration ([7c5ea56](https://github.com/windmill-labs/windmill/commit/7c5ea569a8102ef052d42216e2ff8d4c3169a7a5))


### Bug Fixes

* fix cyclical loop in apps ([61df339](https://github.com/windmill-labs/windmill/commit/61df339343767e63cbe7a4e75f1fd4f848dbd7e0))

## [1.162.2](https://github.com/windmill-labs/windmill/compare/v1.162.1...v1.162.2) (2023-08-29)


### Bug Fixes

* fix incorrect bump ([4704899](https://github.com/windmill-labs/windmill/commit/4704899a81cb281b99949c934184e23b199b2ed8))

## [1.162.1](https://github.com/windmill-labs/windmill/compare/v1.162.0...v1.162.1) (2023-08-29)


### Bug Fixes

* fix deps incompatibilities ([6c5a8a3](https://github.com/windmill-labs/windmill/commit/6c5a8a3613b4608e6d2b57e7f40cd4ab2d1af9ae))

## [1.162.0](https://github.com/windmill-labs/windmill/compare/v1.161.0...v1.162.0) (2023-08-29)


### Features

* add cache to inline scripts ([bf0014c](https://github.com/windmill-labs/windmill/commit/bf0014c387361ce358d31c7cbc44a9c4c97606df))
* add caching to flows and scripts ([#2193](https://github.com/windmill-labs/windmill/issues/2193)) ([03e48a4](https://github.com/windmill-labs/windmill/commit/03e48a4ca557cd2c385988d3a935cea38bc6e81e))
* **frontend:** Filter runs by user ([#2187](https://github.com/windmill-labs/windmill/issues/2187)) ([095969f](https://github.com/windmill-labs/windmill/commit/095969f125e9186cb4f02f75e914ef9a70e3abc4))


### Bug Fixes

* add setState, getState to client ([67f868f](https://github.com/windmill-labs/windmill/commit/67f868f08ed10f3f7c185af67bff7080c339e974))
* relative imports in deno ([30ea354](https://github.com/windmill-labs/windmill/commit/30ea354cae91ea040b3112c4138a1e5f0d7ab530))

## [1.161.0](https://github.com/windmill-labs/windmill/compare/v1.160.0...v1.161.0) (2023-08-28)


### Features

* concurrency limits for flows ([d0d041f](https://github.com/windmill-labs/windmill/commit/d0d041fde37ceda5e3a04e5da9c87d6b7e5691b3))
* early stop for flows ([6354c95](https://github.com/windmill-labs/windmill/commit/6354c95bb74c5d1af838234c0146176a0d3e408e))
* **frontend:** rework premium plans ([#2155](https://github.com/windmill-labs/windmill/issues/2155)) ([272ff63](https://github.com/windmill-labs/windmill/commit/272ff63e4072b4c25a46c133b518649f88b7598e))


### Bug Fixes

* allow deno to --write lock when using lockfiles ([770a3e8](https://github.com/windmill-labs/windmill/commit/770a3e8835637af1b1e017ecc1675e526ca40345))
* fix refresh init in presence of app stepper ([840fbbc](https://github.com/windmill-labs/windmill/commit/840fbbcbb1f969ef3b000f9e50d5c5dde8371995))

## [1.160.0](https://github.com/windmill-labs/windmill/compare/v1.159.0...v1.160.0) (2023-08-27)


### Features

* add parallelism control to forloops ([34e2a80](https://github.com/windmill-labs/windmill/commit/34e2a8001afa8bb948bf907383bffbc8aa11901f))

## [1.159.0](https://github.com/windmill-labs/windmill/compare/v1.158.2...v1.159.0) (2023-08-27)


### Features

* add support for root certificate in postgresql ([b492fd9](https://github.com/windmill-labs/windmill/commit/b492fd98846ff4b4e073bb41de91dd84f0bd7031))
* support to set linked secret variable to any field of a newly created resource ([fe1e419](https://github.com/windmill-labs/windmill/commit/fe1e419fa83db6a9db59aac23490e52cd3649f51))


### Bug Fixes

* canceling jobs ([0dfdf8f](https://github.com/windmill-labs/windmill/commit/0dfdf8fa1be88d601f7dbf7b348aaf8a3ae8e2fd))
* fix app table footer label when -1 ([24ac1e2](https://github.com/windmill-labs/windmill/commit/24ac1e25ff87eef591e9f766bd0e7991b3668723))
* operation are redacted instead of username which fix audit logs for non admin users ([487d56c](https://github.com/windmill-labs/windmill/commit/487d56cb0fedde47c77cdb7a4b5424b51c4a2e10))

## [1.158.2](https://github.com/windmill-labs/windmill/compare/v1.158.1...v1.158.2) (2023-08-26)


### Bug Fixes

* expose getResumeUrls in windmill-client ([3142bc9](https://github.com/windmill-labs/windmill/commit/3142bc932c8ca915b9dda8879d31ef19ecfaa07f))

## [1.158.1](https://github.com/windmill-labs/windmill/compare/v1.158.0...v1.158.1) (2023-08-26)


### Bug Fixes

* fix windmill-client ([7defd45](https://github.com/windmill-labs/windmill/commit/7defd451ac847b9824d503d0b7685344221ff564))

## [1.158.0](https://github.com/windmill-labs/windmill/compare/v1.157.0...v1.158.0) (2023-08-26)


### Features

* add lockfile for deno + use npm module for deno for windmill-client ([9547a06](https://github.com/windmill-labs/windmill/commit/9547a061da0b80a4bc278ee09a0004d410ec7410))

## [1.157.0](https://github.com/windmill-labs/windmill/compare/v1.156.1...v1.157.0) (2023-08-26)


### Features

* lock inline scripts for apps on deploy ([f5121e9](https://github.com/windmill-labs/windmill/commit/f5121e9066e1a93ad6f928daad891a08ae840d81))


### Bug Fixes

* make workspace error handler picker accept any script ([53976da](https://github.com/windmill-labs/windmill/commit/53976da8ae70de3f8e251564220312541604d77b))

## [1.156.1](https://github.com/windmill-labs/windmill/compare/v1.156.0...v1.156.1) (2023-08-25)


### Bug Fixes

* fix python client ([7649a53](https://github.com/windmill-labs/windmill/commit/7649a53f3c792ceba8f2a0fc8535c512b25bf969))

## [1.156.0](https://github.com/windmill-labs/windmill/compare/v1.155.0...v1.156.0) (2023-08-24)


### Features

* schedule recovery handler ([#2126](https://github.com/windmill-labs/windmill/issues/2126)) ([0dcb425](https://github.com/windmill-labs/windmill/commit/0dcb425e4a9cf241ed301f794680b36a7f17cc34))

## [1.155.0](https://github.com/windmill-labs/windmill/compare/v1.154.2...v1.155.0) (2023-08-24)


### Features

* add templatev2 using new eval ([13d870f](https://github.com/windmill-labs/windmill/commit/13d870f16370a74fe481a1701eda27109a776c75))
* eval v2, blazing fast eval triggered only upon the right changes ([#2164](https://github.com/windmill-labs/windmill/issues/2164)) ([5207a7a](https://github.com/windmill-labs/windmill/commit/5207a7a6aa1520c987d26d5c1f99f653c1c81cf6))
* remove connect in favor of eval ([e7aaa17](https://github.com/windmill-labs/windmill/commit/e7aaa177b72749ca9d0d78c452ec8e47d6514186))


### Bug Fixes

* bump bun to 0.8.0 ([4825519](https://github.com/windmill-labs/windmill/commit/4825519ac94a4992cf21fbf4a21fbea8038058d9))
* fix tables not updating inputs on creation ([a419bc4](https://github.com/windmill-labs/windmill/commit/a419bc41bfadce1ac75383d1824ff9fef3404aad))
* **frontend:** Fix code display + use async/await in fetch examples ([#2150](https://github.com/windmill-labs/windmill/issues/2150)) ([2f9177f](https://github.com/windmill-labs/windmill/commit/2f9177f6cec0a676c774ee426482f55227e6e388))
* **frontend:** fix copyToClipboard on non-HTTPS site ([#2046](https://github.com/windmill-labs/windmill/issues/2046)) ([95ea0e8](https://github.com/windmill-labs/windmill/commit/95ea0e8f87195816dde3f9554b3cb92791b63a37))
* update go to 1.12.0 and deno to 1.36.2 ([4317573](https://github.com/windmill-labs/windmill/commit/431757339bbfff6d67f484439d87255acc5c62ff))
* update python client with by_path methods ([8a25a86](https://github.com/windmill-labs/windmill/commit/8a25a86e586485e7949bb208fa94db906e983b6c))

## [1.154.2](https://github.com/windmill-labs/windmill/compare/v1.154.1...v1.154.2) (2023-08-22)


### Bug Fixes

* fix cancel job for flows in some edge cases ([58bb19a](https://github.com/windmill-labs/windmill/commit/58bb19a4471ce8cfced4b144fca40069b5ce0820))

## [1.154.1](https://github.com/windmill-labs/windmill/compare/v1.154.0...v1.154.1) (2023-08-22)


### Bug Fixes

* **frontend:** Fix hub navigation ([#2151](https://github.com/windmill-labs/windmill/issues/2151)) ([d0ed8f0](https://github.com/windmill-labs/windmill/commit/d0ed8f0fefe3176b9bab621a6b3e9231254504e2))
* show for-loop settings ([ab8a27f](https://github.com/windmill-labs/windmill/commit/ab8a27f123fbca187eee3b372d512797f8a03916))

## [1.154.0](https://github.com/windmill-labs/windmill/compare/v1.153.0...v1.154.0) (2023-08-21)


### Features

* deploy folders as well in the UI deployer ([bcf5d4e](https://github.com/windmill-labs/windmill/commit/bcf5d4e5d42a7d17e2d1932b030cca101d9de9b4))


### Bug Fixes

* avoid stack-overflow on jsruntime for recursive objects ([127eea3](https://github.com/windmill-labs/windmill/commit/127eea3c8144b14b8f78a196f5c2cd245d2caad9))
* do not require auth for OPTIONS requests ([bdd59c9](https://github.com/windmill-labs/windmill/commit/bdd59c94a9bde10e808427ef529d1b6ab6e78a45))

## [1.153.0](https://github.com/windmill-labs/windmill/compare/v1.152.0...v1.153.0) (2023-08-20)


### Features

* multiline support in bash ([e1469cc](https://github.com/windmill-labs/windmill/commit/e1469cc64d672b5fc42edac313bc11a017812511))


### Bug Fixes

* update deno-client to use new Resource and Variable endpoints ([c13428a](https://github.com/windmill-labs/windmill/commit/c13428ad089999e38768b86bfd251d747759dc69))

## [1.152.0](https://github.com/windmill-labs/windmill/compare/v1.151.2...v1.152.0) (2023-08-20)


### Features

* handle drift in every time referencing db times ([b9fb206](https://github.com/windmill-labs/windmill/commit/b9fb206c112798f3776ba0e6da70e86e7c769a1f))
* prometheus metrics are now ee only ([2afea50](https://github.com/windmill-labs/windmill/commit/2afea504977f9cd08d62c5f85be1fd2cefe8a691))


### Bug Fixes

* improve progress bar UX ([85d2d47](https://github.com/windmill-labs/windmill/commit/85d2d4782779d981a131f48db6e1058fe79daeef))
* reinit retry to undefined in flow steps ([75f4723](https://github.com/windmill-labs/windmill/commit/75f472381cfa73d77295b29a202efbd58c79918d))

## [1.151.2](https://github.com/windmill-labs/windmill/compare/v1.151.1...v1.151.2) (2023-08-18)


### Bug Fixes

* **frontend:** Fix app multiselect dark mode ([#2121](https://github.com/windmill-labs/windmill/issues/2121)) ([be577e5](https://github.com/windmill-labs/windmill/commit/be577e561dff33a404bb6f29f178b01f20aa0121))
* **frontend:** Fix JSON pane scroll issues ([#2123](https://github.com/windmill-labs/windmill/issues/2123)) ([d367716](https://github.com/windmill-labs/windmill/commit/d367716b0a8198573b26a3c82ac7e4fd9cefe753))

## [1.151.1](https://github.com/windmill-labs/windmill/compare/v1.151.0...v1.151.1) (2023-08-18)


### Bug Fixes

* at UTC Time ([0193fcc](https://github.com/windmill-labs/windmill/commit/0193fcc1d7c24147e553a0e3f9f0ab8d6f5d5996))
* improve flow progress bar ([67cb451](https://github.com/windmill-labs/windmill/commit/67cb4516c913926c1755e46bc7acf46340fdb692))
* show help on empty cli args ([237460b](https://github.com/windmill-labs/windmill/commit/237460b121846d160a40e849bf85fabbb7c14fdc))

## [1.151.0](https://github.com/windmill-labs/windmill/compare/v1.150.0...v1.151.0) (2023-08-17)


### Features

* **frontend:** Fix workspace switch + always displays confirmation modal on top of splitpanel separator ([#2115](https://github.com/windmill-labs/windmill/issues/2115)) ([eea9ce9](https://github.com/windmill-labs/windmill/commit/eea9ce93b918115e9ed6b951d000049ca66bd5fd))


### Bug Fixes

* fix python get_resource ([cb00a13](https://github.com/windmill-labs/windmill/commit/cb00a1358d0e47575d8315e70695a9693190f211))

## [1.150.0](https://github.com/windmill-labs/windmill/compare/v1.149.0...v1.150.0) (2023-08-17)


### Features

* copilot tokens streaming + cancel ([#2107](https://github.com/windmill-labs/windmill/issues/2107)) ([82612c3](https://github.com/windmill-labs/windmill/commit/82612c35bd4cd15af21582f9650b615d3e12c06c))
* graphql custom headers ([#2111](https://github.com/windmill-labs/windmill/issues/2111)) ([6733b85](https://github.com/windmill-labs/windmill/commit/6733b8552b1128663c8fb8086c85ad0406d9b999))


### Bug Fixes

* powershell icon ([#2109](https://github.com/windmill-labs/windmill/issues/2109)) ([c817af7](https://github.com/windmill-labs/windmill/commit/c817af769457a069617fafb2d3fcf38a85212690))
* set NETRC at init and not for every job ([359845f](https://github.com/windmill-labs/windmill/commit/359845fa9dd14e8445cc95e73cc646dce1f45ddb))
* unify clients to use server-side interpolation to retrieve full resources ([067908c](https://github.com/windmill-labs/windmill/commit/067908c0b59f1e73222cad0e5f214f3605006ef3))
* unify clients to use server-side interpolation to retrieve full resources ([930839a](https://github.com/windmill-labs/windmill/commit/930839aad22eaeee0737f1d057b8cfb538d26d3f))
* unify clients to use server-side interpolation to retrieve full resources ([e9c19b5](https://github.com/windmill-labs/windmill/commit/e9c19b5b985c0e03524b2d12b1f26a0e6fdc6e0b))

## [1.149.0](https://github.com/windmill-labs/windmill/compare/v1.148.0...v1.149.0) (2023-08-17)


### Features

* **frontend:** Add List pagination + add loading state in tables ([#2096](https://github.com/windmill-labs/windmill/issues/2096)) ([9b15e40](https://github.com/windmill-labs/windmill/commit/9b15e409a5b902874d0cf1566b57db6fc23a87ec))


### Bug Fixes

* appgrid refresh selected on row on result changes ([0af264f](https://github.com/windmill-labs/windmill/commit/0af264f6f8d0ff018094b97a2af9fe6f02e6ccfe))
* fix folder creation if job folder already exist ([c320ea8](https://github.com/windmill-labs/windmill/commit/c320ea865f1632e517d4c597491517da89ff77e7))
* fix go envs passing ([ed6494f](https://github.com/windmill-labs/windmill/commit/ed6494ff7a1f6102eaad8c0052c1ac3f82d4cadf))
* **frontend:** Fix toast when adding a user + set default vscoode the… ([#2080](https://github.com/windmill-labs/windmill/issues/2080)) ([801f2a8](https://github.com/windmill-labs/windmill/commit/801f2a8299956f0debe95bb13faef798a0ea0b08))

## [1.148.0](https://github.com/windmill-labs/windmill/compare/v1.147.3...v1.148.0) (2023-08-14)


### Features

* add s3 snippets ([#2052](https://github.com/windmill-labs/windmill/issues/2052)) ([beb4a00](https://github.com/windmill-labs/windmill/commit/beb4a000e3631a1b0a27a68923361652317aec63))


### Bug Fixes

* allow multiple db schema explorers ([#2054](https://github.com/windmill-labs/windmill/issues/2054)) ([e1b4f0a](https://github.com/windmill-labs/windmill/commit/e1b4f0a8328bc62a19e693bac99589711d08d566))
* **frontend:** Fix Dark mode in the sleep helpbox ([#2072](https://github.com/windmill-labs/windmill/issues/2072)) ([c6ef1a6](https://github.com/windmill-labs/windmill/commit/c6ef1a6d4fbe5661f6b9018121e21061952908d0))
* handle object pat in sig of typescript ([1d8213a](https://github.com/windmill-labs/windmill/commit/1d8213a25ba90f3d4af952e03c74196f8ce908ab))
* remove ansi codes from result ([#2069](https://github.com/windmill-labs/windmill/issues/2069)) ([a3fa174](https://github.com/windmill-labs/windmill/commit/a3fa174cd46ce1bd67a69f7781dbdfa0719d3d06))
* script fix no resource + error handling ([#2053](https://github.com/windmill-labs/windmill/issues/2053)) ([00b1afb](https://github.com/windmill-labs/windmill/commit/00b1afb1c90773408d1dc3233a25fa93e24d4da0))

## [1.147.3](https://github.com/windmill-labs/windmill/compare/v1.147.2...v1.147.3) (2023-08-13)


### Bug Fixes

* **bun:** correctly handle empty deps script bun to deploy ([46b25f9](https://github.com/windmill-labs/windmill/commit/46b25f9b550f5f8e804cabeeeb575daea46cba31))

## [1.147.2](https://github.com/windmill-labs/windmill/compare/v1.147.1...v1.147.2) (2023-08-13)


### Bug Fixes

* **bun:** add npm type acquisition ([3284245](https://github.com/windmill-labs/windmill/commit/32842457fef73f654ca89c3a232265927cf40961))

## [1.147.1](https://github.com/windmill-labs/windmill/compare/v1.147.0...v1.147.1) (2023-08-13)


### Bug Fixes

* **bun:** only install -p dependencies ([23164c8](https://github.com/windmill-labs/windmill/commit/23164c83494ee6f42e77b181de0df26b4fba22dc))
* **bun:** only install when requirements are missing if using nsjail ([3bc1050](https://github.com/windmill-labs/windmill/commit/3bc1050258bd7a9ba2be739144260037d2274b87))

## [1.147.0](https://github.com/windmill-labs/windmill/compare/v1.146.1...v1.147.0) (2023-08-13)


### Features

* add lsp to bun and remove experimental status ([891c9dc](https://github.com/windmill-labs/windmill/commit/891c9dc266edea4f5239f1a82c884437b7df89e4))

## [1.146.1](https://github.com/windmill-labs/windmill/compare/v1.146.0...v1.146.1) (2023-08-13)


### Bug Fixes

* **bun:** windmill-client does not require set to be initalized ([993a145](https://github.com/windmill-labs/windmill/commit/993a14502fb16387b174d1af19c87d3ae65c317c))
* enable bun to do resolution as fallback to allow specifier ([9c97828](https://github.com/windmill-labs/windmill/commit/9c978281cdbfefa7d11213a181ffcbfdfac8115e))
* powershell escape backticks ([#2044](https://github.com/windmill-labs/windmill/issues/2044)) ([cddef1a](https://github.com/windmill-labs/windmill/commit/cddef1a50a48e7cb60a69762a579b95e0018aa17))
* really use bun in flow builder ([#2045](https://github.com/windmill-labs/windmill/issues/2045)) ([c2281ef](https://github.com/windmill-labs/windmill/commit/c2281ef5da7aa0222e70c5f6ca91d066d79d3862))

## [1.146.0](https://github.com/windmill-labs/windmill/compare/v1.145.3...v1.146.0) (2023-08-12)


### Features

* respect lockfiles for bun ([2ba132b](https://github.com/windmill-labs/windmill/commit/2ba132bd05fc1b01e6de19ac13e98100f55f8895))


### Bug Fixes

* fix array static input editor initialization ([4dcf7ae](https://github.com/windmill-labs/windmill/commit/4dcf7ae088d336171d58aa8914c6b58ec522cc14))

## [1.145.3](https://github.com/windmill-labs/windmill/compare/v1.145.2...v1.145.3) (2023-08-11)


### Bug Fixes

* fix bun client ([611d42d](https://github.com/windmill-labs/windmill/commit/611d42db2caa7cf366d7c67ee1434d8de2be8a97))

## [1.145.2](https://github.com/windmill-labs/windmill/compare/v1.145.1...v1.145.2) (2023-08-11)


### Bug Fixes

* **bun:** remove need for manual setClient ([4794bd0](https://github.com/windmill-labs/windmill/commit/4794bd0b60268db7c679b2faa2692f6fceb5769f))

## [1.145.1](https://github.com/windmill-labs/windmill/compare/v1.145.0...v1.145.1) (2023-08-11)


### Bug Fixes

* sqlx build ([169c413](https://github.com/windmill-labs/windmill/commit/169c413c8d0519e7c11d4d0847585aff59da23e5))

## [1.145.0](https://github.com/windmill-labs/windmill/compare/v1.144.4...v1.145.0) (2023-08-11)


### Features

* add native powershell support ([#2025](https://github.com/windmill-labs/windmill/issues/2025)) ([8a1f9a7](https://github.com/windmill-labs/windmill/commit/8a1f9a7c6aadf735f3d6f118fbc8a344a675ec6a))
* **frontend:** Runs rework v2 ([#2012](https://github.com/windmill-labs/windmill/issues/2012)) ([7d88a2d](https://github.com/windmill-labs/windmill/commit/7d88a2d13ade2265532a222ca2b0e804bd3b2e02))
* migrate state path to new schema ([de8a727](https://github.com/windmill-labs/windmill/commit/de8a7279b644cd1eb7999b9da2900b760acd7297))


### Bug Fixes

* **frontend:** Fix lagging issues when resizing ([#2027](https://github.com/windmill-labs/windmill/issues/2027)) ([c2a92b6](https://github.com/windmill-labs/windmill/commit/c2a92b69ef0b5acacbda38261e654fe7d7cf36f6))
* **frontend:** Handle invalid string defaults for date values. ([#2033](https://github.com/windmill-labs/windmill/issues/2033)) ([7cdd6db](https://github.com/windmill-labs/windmill/commit/7cdd6db3feeb99a0055ab187348aabfc7a979915))
* modify snake case numbers resource types ([#2029](https://github.com/windmill-labs/windmill/issues/2029)) ([a4ba4af](https://github.com/windmill-labs/windmill/commit/a4ba4af478d2cebf1b4840091446be65f2f9d224))
* restrict furthermore when the summary is transformed into a path ([2de4192](https://github.com/windmill-labs/windmill/commit/2de4192cac84336e0b812862b7dca3769a0ba4fc))
* sync dark-mode icon across multiple renders ([#2024](https://github.com/windmill-labs/windmill/issues/2024)) ([27a8e52](https://github.com/windmill-labs/windmill/commit/27a8e526f79c6b0d7e0d8f8ceb34d4355b5df46b))

## [1.144.4](https://github.com/windmill-labs/windmill/compare/v1.144.3...v1.144.4) (2023-08-10)


### Bug Fixes

* revert monaco update ([785e172](https://github.com/windmill-labs/windmill/commit/785e172e6eb83c107cad2c843a15234a6c6f9f6b))

## [1.144.3](https://github.com/windmill-labs/windmill/compare/v1.144.2...v1.144.3) (2023-08-10)


### Bug Fixes

* fix monaco initialize api error ([fb64ba0](https://github.com/windmill-labs/windmill/commit/fb64ba034442fa52ecf2fb88c8974ba184b58ef9))
* revert monaco update ([f4de5ea](https://github.com/windmill-labs/windmill/commit/f4de5ea436b2bdf8c92e27ce43f684116f47d1ff))

## [1.144.2](https://github.com/windmill-labs/windmill/compare/v1.144.1...v1.144.2) (2023-08-09)


### Bug Fixes

* make path changeable even if linked to summary ([f3b674a](https://github.com/windmill-labs/windmill/commit/f3b674acd1a0e76c12c321fa7d9d131716622ae5))

## [1.144.1](https://github.com/windmill-labs/windmill/compare/v1.144.0...v1.144.1) (2023-08-09)


### Bug Fixes

* make path changeable even if linked to summary ([003da78](https://github.com/windmill-labs/windmill/commit/003da78a46cce3a3376e375a74b9e5f31f4b6256))

## [1.144.0](https://github.com/windmill-labs/windmill/compare/v1.143.0...v1.144.0) (2023-08-09)


### Features

* add graphql support ([#2014](https://github.com/windmill-labs/windmill/issues/2014)) ([e4534d2](https://github.com/windmill-labs/windmill/commit/e4534d2dc329d307ca7690ab58bf3b063ad81539))
* **frontend:** Add disable prop to App Toggles ([#2010](https://github.com/windmill-labs/windmill/issues/2010)) ([40c86e4](https://github.com/windmill-labs/windmill/commit/40c86e4f4b5a511fc8059051f326e930f9bc7839))
* implement binary caching for go ([933021a](https://github.com/windmill-labs/windmill/commit/933021ad8d1d7cf70f9b3f56e1671046675dec3c))
* v0 of relative imports in bun ([383793f](https://github.com/windmill-labs/windmill/commit/383793f7991ff4c1024e1b86b418f01f2557d5e0))


### Bug Fixes

* **frontend:** Fix flow preview ([#2013](https://github.com/windmill-labs/windmill/issues/2013)) ([0b8d37a](https://github.com/windmill-labs/windmill/commit/0b8d37a2486df5756148645a630213d16e5998bc))
* graphql api not db ([#2017](https://github.com/windmill-labs/windmill/issues/2017)) ([356b1f2](https://github.com/windmill-labs/windmill/commit/356b1f2242d7bbe4c71e021cb441e29b652c5126))
* hide AI Gen btn when language not supported ([#2016](https://github.com/windmill-labs/windmill/issues/2016)) ([46ff76f](https://github.com/windmill-labs/windmill/commit/46ff76fc86884e11986edc998e06f37c43102d1f))
* make flow editor more resilient to id duplicates ([83d1d11](https://github.com/windmill-labs/windmill/commit/83d1d11a934843d91c76912018a3c057a97de101))

## [1.143.0](https://github.com/windmill-labs/windmill/compare/v1.142.0...v1.143.0) (2023-08-08)


### Features

* **frontend:** add disabled prop to select input ([#2007](https://github.com/windmill-labs/windmill/issues/2007)) ([f6c9e34](https://github.com/windmill-labs/windmill/commit/f6c9e349fc82a74efed6fb8ddb6d79889b8b031b))


### Bug Fixes

* add BASE_URL and WM_TOKEN to native scripts ([b5ba9da](https://github.com/windmill-labs/windmill/commit/b5ba9daffce8891ba54697cd595ac935a7266e4d))
* fix clear schedule to be workspace specific ([1d1cd31](https://github.com/windmill-labs/windmill/commit/1d1cd31252c6619441219cdb2bb6ba064d029ac9))
* **frontend:** Fix auto invite overflow ([#2009](https://github.com/windmill-labs/windmill/issues/2009)) ([c22e3b5](https://github.com/windmill-labs/windmill/commit/c22e3b54025153a9d28831c2fdacc9bd6d558c2c))

## [1.142.0](https://github.com/windmill-labs/windmill/compare/v1.141.0...v1.142.0) (2023-08-07)


### Features

* add magic tag part ([90dfda0](https://github.com/windmill-labs/windmill/commit/90dfda0d1f00e1f11a82d12d2466eb2252c6e5fb))
* **frontend:** Audit logs rework ([#1997](https://github.com/windmill-labs/windmill/issues/1997)) ([57110b9](https://github.com/windmill-labs/windmill/commit/57110b93c942024099538143f695c6c9294d0097))
* **frontend:** make diff editor editable ([#1999](https://github.com/windmill-labs/windmill/issues/1999)) ([dee1096](https://github.com/windmill-labs/windmill/commit/dee1096bc0cb094932320c4a7801106a0eba2d59))


### Bug Fixes

* custom config layout get priority for plotly components ([e7febc7](https://github.com/windmill-labs/windmill/commit/e7febc759676c1f0f5030874abc7382ec87d47a2))
* **frontend:** Download as CSV ([#2000](https://github.com/windmill-labs/windmill/issues/2000)) ([5f3b2ea](https://github.com/windmill-labs/windmill/commit/5f3b2eacbf1d10fe870074ea079ce66e6dca0d5d))
* refresh token on login and regularly ([9337716](https://github.com/windmill-labs/windmill/commit/933771651e9dde1c3489aaa9f31d9331ac4d5f7f))

## [1.141.0](https://github.com/windmill-labs/windmill/compare/v1.140.1...v1.141.0) (2023-08-05)


### Features

* add support for custom import map on deno ([23a5bfa](https://github.com/windmill-labs/windmill/commit/23a5bfa36824c48694dbe42080b14d8969cbf3da))

## [1.140.1](https://github.com/windmill-labs/windmill/compare/v1.140.0...v1.140.1) (2023-08-05)


### Bug Fixes

* **cli:** handle extra headers in zip call ([7a731dc](https://github.com/windmill-labs/windmill/commit/7a731dc838fae1664ca80ed572e5e986b331d874))

## [1.140.0](https://github.com/windmill-labs/windmill/compare/v1.139.0...v1.140.0) (2023-08-05)


### Features

* add azure openAI support ([#1989](https://github.com/windmill-labs/windmill/issues/1989)) ([0b7d639](https://github.com/windmill-labs/windmill/commit/0b7d6398cbddfd65306542a8300881517b1413cb))
* add snowflake ([#1987](https://github.com/windmill-labs/windmill/issues/1987)) ([d57b8d7](https://github.com/windmill-labs/windmill/commit/d57b8d79ad7493905e11e42470a8bfaa59e68709))
* add test connection for bigquery ([#1988](https://github.com/windmill-labs/windmill/issues/1988)) ([c585377](https://github.com/windmill-labs/windmill/commit/c585377c2a7b42a1e74ff55b37cac7afceee318d))
* add toggle for postgres between public and all schemas ([#1991](https://github.com/windmill-labs/windmill/issues/1991)) ([8d550a7](https://github.com/windmill-labs/windmill/commit/8d550a7ea5708ccae1136f4c0445fd9e4573341c))
* **frontend:** Add flow steps details ([#1986](https://github.com/windmill-labs/windmill/issues/1986)) ([6d89121](https://github.com/windmill-labs/windmill/commit/6d89121ff951b3f192138c3c453c6d78f4bb6285))
* **frontend:** Settings rework ([#1983](https://github.com/windmill-labs/windmill/issues/1983)) ([b8e9338](https://github.com/windmill-labs/windmill/commit/b8e9338d722fe0ec166df3f3b7c895f2ed8ea7ac))
* support native jobs from hub ([af29692](https://github.com/windmill-labs/windmill/commit/af29692ee1231b202d3c11b65559ae14421c472d))


### Bug Fixes

* add more indexes for performance reasons ([4e21b1a](https://github.com/windmill-labs/windmill/commit/4e21b1ac1780ba966f030c537b4b6d9650a12e61))
* ai code block regex ([#1992](https://github.com/windmill-labs/windmill/issues/1992)) ([8289afd](https://github.com/windmill-labs/windmill/commit/8289afd8ff7b5b35f123c43941232ffae1602c27))
* **frontend:** Fix flow editor panel sizes ([#1985](https://github.com/windmill-labs/windmill/issues/1985)) ([911162a](https://github.com/windmill-labs/windmill/commit/911162a1d2c7444dd4d4e98e96fbb542e004130b))
* **frontend:** Fix image loading animation + app preview select scrolling issues ([#1990](https://github.com/windmill-labs/windmill/issues/1990)) ([ae79216](https://github.com/windmill-labs/windmill/commit/ae79216d5322c237250aad272bc7b73864ac7c62))
* **frontend:** Fix log bg color + add style to the supabase connect button ([#1981](https://github.com/windmill-labs/windmill/issues/1981)) ([b2f23fb](https://github.com/windmill-labs/windmill/commit/b2f23fbaa167f10ae36ebe8a70cc35830051ddc2))
* **frontend:** View runs+ fix flow graph overflow issues ([#1984](https://github.com/windmill-labs/windmill/issues/1984)) ([923504f](https://github.com/windmill-labs/windmill/commit/923504f2b40781a857ce08ff9ae7d74d73afe02d))
* make plotly dynamically change on layout change ([c31118c](https://github.com/windmill-labs/windmill/commit/c31118c270c69f6d54a9ff3e706ac175f7996f9e))
* reset with minimal code ([#1982](https://github.com/windmill-labs/windmill/issues/1982)) ([c031b9f](https://github.com/windmill-labs/windmill/commit/c031b9f3525855c695d557ecb8c8e93b695e2eaa))

## [1.139.0](https://github.com/windmill-labs/windmill/compare/v1.138.1...v1.139.0) (2023-08-01)


### Features

* add bun to flow and apps ([0081f54](https://github.com/windmill-labs/windmill/commit/0081f54c777e7586a6b55a020cd9134fc66837d9))
* add SECRET_SALT for secure environments ([7afb686](https://github.com/windmill-labs/windmill/commit/7afb6869d0cbdded2f0c0e395f77c9f9889788a3))
* add step's custom timeout ([4c87027](https://github.com/windmill-labs/windmill/commit/4c870272d487e8deef9b22c2dfe829b0a92afc44))
* add support for postgresql numeric ([e51d67f](https://github.com/windmill-labs/windmill/commit/e51d67f843b6a6849dd9b8fb496d0c20c34d9c9c))
* **frontend:** Add config to optionally include mimetype ([#1978](https://github.com/windmill-labs/windmill/issues/1978)) ([654efb7](https://github.com/windmill-labs/windmill/commit/654efb7ec47887d61b25b4fcbf6d03d42882b240))
* **frontend:** add markdown component ([#1959](https://github.com/windmill-labs/windmill/issues/1959)) ([a69aa22](https://github.com/windmill-labs/windmill/commit/a69aa2275f04eca82eff6590cc6296f0ed8d6fc1))
* **frontend:** App carousel ([#1956](https://github.com/windmill-labs/windmill/issues/1956)) ([3a40b19](https://github.com/windmill-labs/windmill/commit/3a40b19cdbf608f7aa3cd81e10ed583bb5e24394))
* **frontend:** Sanitize Supabase resource name ([#1975](https://github.com/windmill-labs/windmill/issues/1975)) ([aeb1131](https://github.com/windmill-labs/windmill/commit/aeb1131a3d553f128295ae11338a9d454bbe85c4))
* unveil windmill AI ([#1972](https://github.com/windmill-labs/windmill/issues/1972)) ([b479cd6](https://github.com/windmill-labs/windmill/commit/b479cd6fca8ac74bb8df4f126552f455d689b75f))


### Bug Fixes

* **cli:** add support for inlining native ts ([87326b7](https://github.com/windmill-labs/windmill/commit/87326b7d16c8c4c2ae1d0a369ab621db23e8d664))
* fix draft permissions (require writer instead of owner) ([bf57c3a](https://github.com/windmill-labs/windmill/commit/bf57c3a628d78af18bcc4c4051e2425313d2d6f7))
* **frontend:** Display transformer errors ([#1971](https://github.com/windmill-labs/windmill/issues/1971)) ([d67cfa4](https://github.com/windmill-labs/windmill/commit/d67cfa4aa9fc09834a3704a37ffd9df539283cc4))
* **frontend:** Fix app icons ([#1977](https://github.com/windmill-labs/windmill/issues/1977)) ([1a15372](https://github.com/windmill-labs/windmill/commit/1a1537265accb4be7f24ea0e755979ff1333f9b1))
* **frontend:** Fix dropdown buttons ([#1970](https://github.com/windmill-labs/windmill/issues/1970)) ([eea36b5](https://github.com/windmill-labs/windmill/commit/eea36b5bfc541d10e1adfbfdf9b97883a6d3fd7e))
* **frontend:** Fix script settings overflow ([#1969](https://github.com/windmill-labs/windmill/issues/1969)) ([b576686](https://github.com/windmill-labs/windmill/commit/b57668610cae73d85a974493d8c0a5f3125f7007))
* improve code structure to reduce unecessary dependency of apppreview on heavy packages ([3410e66](https://github.com/windmill-labs/windmill/commit/3410e66b22b4b0d8fdf12ed9144ff694bd258656))

## [1.138.1](https://github.com/windmill-labs/windmill/compare/v1.138.0...v1.138.1) (2023-07-30)


### Bug Fixes

* **cli:** reassign -d to --verbose and --data ([5a354fc](https://github.com/windmill-labs/windmill/commit/5a354fcc2d166a4c98749f21e1026ff32a2fb111))
* **frontend:** fix rename for runnable inputs ([3c0c05a](https://github.com/windmill-labs/windmill/commit/3c0c05a2eb16c9c37ffe334ff17fa976d7d0d74e))
* **postgres:** add uuid support as input ([a3801d0](https://github.com/windmill-labs/windmill/commit/a3801d086de1fa7ca6afb7854ccfa86410341bd7))
* **postgres:** add uuid support as input ([3dac295](https://github.com/windmill-labs/windmill/commit/3dac295d41666a3766bf1843e757e7946958c527))

## [1.138.0](https://github.com/windmill-labs/windmill/compare/v1.137.1...v1.138.0) (2023-07-28)


### Features

* add bigquery ([#1934](https://github.com/windmill-labs/windmill/issues/1934)) ([fd4c978](https://github.com/windmill-labs/windmill/commit/fd4c978874e6020d59e85b209d418435a0bcda1b))
* add supabaze wizard ([24b0658](https://github.com/windmill-labs/windmill/commit/24b0658460453b6e8d241be3be9f11946c3cf84b))
* **frontend:** Make app from scripts and flows ([#1938](https://github.com/windmill-labs/windmill/issues/1938)) ([9f9498d](https://github.com/windmill-labs/windmill/commit/9f9498dbd90349ad641487824d4d85ed73c43260))
* **frontend:** schema explorer, autocomplete and db aware AI for mysql ([#1944](https://github.com/windmill-labs/windmill/issues/1944)) ([5061a87](https://github.com/windmill-labs/windmill/commit/5061a873760f232d7824f407d2d0fad5ee6891db))


### Bug Fixes

* add sync method for flows ([e03da23](https://github.com/windmill-labs/windmill/commit/e03da23f17a63dea30a93607a2986d9ddeb6c213))
* **frontend:** AI gen popup ([#1950](https://github.com/windmill-labs/windmill/issues/1950)) ([029d017](https://github.com/windmill-labs/windmill/commit/029d0170995f3bc1f0fe43f3e5991b7513121439))
* **frontend:** Fix Account settings unreadable texts ([#1958](https://github.com/windmill-labs/windmill/issues/1958)) ([3b90580](https://github.com/windmill-labs/windmill/commit/3b905800bff45eaa23dd69e5b60619bf1d289e3d))
* **frontend:** Fix App Table select ([#1955](https://github.com/windmill-labs/windmill/issues/1955)) ([16d6815](https://github.com/windmill-labs/windmill/commit/16d6815945eccd1c671b73ffd2163973874bea5c))
* **frontend:** Fix build app from flow ([#1954](https://github.com/windmill-labs/windmill/issues/1954)) ([5c66afe](https://github.com/windmill-labs/windmill/commit/5c66afeb8fec3829e1fcdc95afcc4c4050470793))
* **frontend:** Fix dark mode issues ([#1953](https://github.com/windmill-labs/windmill/issues/1953)) ([4f0c94a](https://github.com/windmill-labs/windmill/commit/4f0c94aafbef08b7c5f44f4073a3adfb17956a95))
* **frontend:** reset btn for all langs ([#1949](https://github.com/windmill-labs/windmill/issues/1949)) ([265b7d7](https://github.com/windmill-labs/windmill/commit/265b7d7fbe1402986492c02d200342596925bcab))
* improve webhooks panel correctness ([adea8ff](https://github.com/windmill-labs/windmill/commit/adea8ff1b484e8653ae189312775cd0f34e321dd))
* prevent error if json editor not mounted ([#1945](https://github.com/windmill-labs/windmill/issues/1945)) ([bdde59d](https://github.com/windmill-labs/windmill/commit/bdde59d7b385fbdbbac722f918672c7e3d601d56))
* schema modal behavior when pressing enter ([#1947](https://github.com/windmill-labs/windmill/issues/1947)) ([3d54790](https://github.com/windmill-labs/windmill/commit/3d5479000a3732f7299ba79a57bd06303a359d90))

## [1.137.1](https://github.com/windmill-labs/windmill/compare/v1.137.0...v1.137.1) (2023-07-27)


### Bug Fixes

* pin deno backend versions ([acf2765](https://github.com/windmill-labs/windmill/commit/acf27659a9fd619bfbb1f2edf9c6895bdabed083))

## [1.137.0](https://github.com/windmill-labs/windmill/compare/v1.136.0...v1.137.0) (2023-07-27)


### Features

* add workspace specific tags ([52f28b5](https://github.com/windmill-labs/windmill/commit/52f28b5173daffdbffeb45dbe94574fe54c73f4b))
* extra_requirements ([93ac794](https://github.com/windmill-labs/windmill/commit/93ac7944b04b0e39043ed149df0dd3f50ff0e02a))
* **frontend:** Add an output format ([#1939](https://github.com/windmill-labs/windmill/issues/1939)) ([e4506fe](https://github.com/windmill-labs/windmill/commit/e4506fef0ed3ece7702d677d4a82c87e8e7616a4))
* **frontend:** AI edit / fix improvements ([#1923](https://github.com/windmill-labs/windmill/issues/1923)) ([0aa81e3](https://github.com/windmill-labs/windmill/commit/0aa81e39705d8c2109c8ec30855bb5f68eae133b))
* **frontend:** App components dark mode ([#1937](https://github.com/windmill-labs/windmill/issues/1937)) ([71502c2](https://github.com/windmill-labs/windmill/commit/71502c2e0eced308fec3783450466c37007292e4))
* **frontend:** Make Plotly layout dynamic ([#1942](https://github.com/windmill-labs/windmill/issues/1942)) ([9a539f9](https://github.com/windmill-labs/windmill/commit/9a539f909dd9e960f29901861dff674c416b4601))
* handle worker groups with redis ([6f47bf9](https://github.com/windmill-labs/windmill/commit/6f47bf98065ff42d35078b9376fc670dbc868ea6))
* lock depedency for the entire flow + dependency job depend on script/flow's tag ([90d57e2](https://github.com/windmill-labs/windmill/commit/90d57e2fadd9459d7fda6fad35aeb603e5074a65))
* resolve dependencies across relative imports for python ([0f31ffe](https://github.com/windmill-labs/windmill/commit/0f31ffe174a8414393f8a2c3d0d9a0b4256667b6))
* resolve dependencies across relative imports for python ([31141ce](https://github.com/windmill-labs/windmill/commit/31141ce52a73cdfa89127b9c4a03428bab6029cc))
* use flock to avoid concurrency issues on pip and shared volume ([c22d2b9](https://github.com/windmill-labs/windmill/commit/c22d2b91a1d4257a6daeae1e29d77e9cc7fd3be5))
* worker group for flows ([a099791](https://github.com/windmill-labs/windmill/commit/a0997911bf9da8651ddb830e9e09f2d3f82c73e4))


### Bug Fixes

* add property while viewing as JSON (+ ui tweaks) ([#1941](https://github.com/windmill-labs/windmill/issues/1941)) ([4f3b483](https://github.com/windmill-labs/windmill/commit/4f3b4836c2834d1f9975b92d8605bc6b046319fa))
* respect FIFO order for concurrency limit ([601da7f](https://github.com/windmill-labs/windmill/commit/601da7f878ca039729e2ba1be734530b63bd773f))

## [1.136.0](https://github.com/windmill-labs/windmill/compare/v1.135.1...v1.136.0) (2023-07-24)


### Features

* add SCIM instances groups to group page ([6517caf](https://github.com/windmill-labs/windmill/commit/6517caf7d5e5a905d251dfcc3055308487e644f8))


### Bug Fixes

* **frontend:** Fix fetch webhook code + add copy to clipboard button ([#1928](https://github.com/windmill-labs/windmill/issues/1928)) ([7799e4e](https://github.com/windmill-labs/windmill/commit/7799e4e73283d51b7dff8a27f70ecf29be298c13))
* improve SCIM sync ([c05b138](https://github.com/windmill-labs/windmill/commit/c05b13804f21cb02d5f27df2a046e37a6ccfcce7))

## [1.135.1](https://github.com/windmill-labs/windmill/compare/v1.135.0...v1.135.1) (2023-07-23)


### Bug Fixes

* fix database migration ([0b019bc](https://github.com/windmill-labs/windmill/commit/0b019bc8a917a76c7631a20fb4a21f7252c418ba))

## [1.135.0](https://github.com/windmill-labs/windmill/compare/v1.134.2...v1.135.0) (2023-07-22)


### Features

* add SCIM support ([ebb9235](https://github.com/windmill-labs/windmill/commit/ebb92356febadd4a0576b1bb88f59dc79da3b7e4))
* add SCIM support ([c4d1d50](https://github.com/windmill-labs/windmill/commit/c4d1d50f817c2b0d014b925056d6f404415f004f))
* **frontend:** db schema explorer + db aware AI ([#1920](https://github.com/windmill-labs/windmill/issues/1920)) ([a6025ae](https://github.com/windmill-labs/windmill/commit/a6025ae75e47f1f66abd865604a991c42c4920f1))


### Bug Fixes

* **frontend:** Fix show archived button position ([#1921](https://github.com/windmill-labs/windmill/issues/1921)) ([713f3e8](https://github.com/windmill-labs/windmill/commit/713f3e84c94a0c9a0bddc504702833974d7f70d9))
* off by one concurrency limit fix ([a054bdd](https://github.com/windmill-labs/windmill/commit/a054bdd0438567996b551b1b00a4c0697ce61986))

## [1.134.2](https://github.com/windmill-labs/windmill/compare/v1.134.1...v1.134.2) (2023-07-20)


### Bug Fixes

* **frontend:** Prevent options from closing when an option is selected ([#1912](https://github.com/windmill-labs/windmill/issues/1912)) ([b2b3249](https://github.com/windmill-labs/windmill/commit/b2b3249e51c3340b8a819e037ba68984a35d90a8))
* remove lockfile on any rawinput change in flows ([8c58752](https://github.com/windmill-labs/windmill/commit/8c58752a16e66d74981eb5eab4763198d4775905))
* remove lockfile on any rawinput change in flows ([dfb1d8f](https://github.com/windmill-labs/windmill/commit/dfb1d8fa44222f52b285a37d867a42cb1f27450d))

## [1.134.1](https://github.com/windmill-labs/windmill/compare/v1.134.0...v1.134.1) (2023-07-20)


### Bug Fixes

* handle pip requirements to git commits ([a48edf4](https://github.com/windmill-labs/windmill/commit/a48edf435fb1df876c8012bf49a4c4265847d10e))
* s/paylod/payload ([#1910](https://github.com/windmill-labs/windmill/issues/1910)) ([8f3960c](https://github.com/windmill-labs/windmill/commit/8f3960c93556301f6fdf9825a6e6b2e4d389dd2c))

## [1.134.0](https://github.com/windmill-labs/windmill/compare/v1.133.0...v1.134.0) (2023-07-19)


### Features

* **frontend:** add deployment history + script path ([#1896](https://github.com/windmill-labs/windmill/issues/1896)) ([3a805d1](https://github.com/windmill-labs/windmill/commit/3a805d1e4b85009fae3f81d97b918b3c6bd551b5))
* make row information available from table rows' evals ([ad1b92d](https://github.com/windmill-labs/windmill/commit/ad1b92d59df5aba39d7ae29e902c55b1f2411458))
* use openai resource for windmill AI ([#1902](https://github.com/windmill-labs/windmill/issues/1902)) ([ddd8049](https://github.com/windmill-labs/windmill/commit/ddd8049b0aa74c9431cd01ff8a6e10e8a0196b3d))


### Bug Fixes

* **backend:** openai resource not only variable ([#1906](https://github.com/windmill-labs/windmill/issues/1906)) ([778ac92](https://github.com/windmill-labs/windmill/commit/778ac92411fc1dd5686087797be19fb602c55d46))
* parse bash args with same-line comments ([#1907](https://github.com/windmill-labs/windmill/issues/1907)) ([0f7ed87](https://github.com/windmill-labs/windmill/commit/0f7ed8798be7ef33f91fd5c4cd751beec28601a1))

## [1.133.0](https://github.com/windmill-labs/windmill/compare/v1.132.0...v1.133.0) (2023-07-19)


### Features

* add SAML support in EE ([d715ec5](https://github.com/windmill-labs/windmill/commit/d715ec58f251765ad2071809161eab8ad189d92d))
* **frontend:** generate scripts in the flow and app builders ([#1886](https://github.com/windmill-labs/windmill/issues/1886)) ([2416805](https://github.com/windmill-labs/windmill/commit/24168056293d4e570f78fbd13068bb94b76d9d9c))

## [1.132.0](https://github.com/windmill-labs/windmill/compare/v1.131.0...v1.132.0) (2023-07-16)


### Features

* add powershell as a template ([b71362f](https://github.com/windmill-labs/windmill/commit/b71362fc7f9eb8a4506d231eb6687eb26696da24))
* add schedule to syncable resources ([1956c43](https://github.com/windmill-labs/windmill/commit/1956c43705f11e809abf113f7af8deb708e5ccd2))
* add whitelist envs to passthrough the workers ([ff0048a](https://github.com/windmill-labs/windmill/commit/ff0048afabad865898cda4be3a599f8d9ef569e8))
* **frontend:** Eval for Drawer titles ([#1882](https://github.com/windmill-labs/windmill/issues/1882)) ([fee2b47](https://github.com/windmill-labs/windmill/commit/fee2b47ebe47a625e0f2b0672f232b54b544200e))


### Bug Fixes

* **frontend:** fix bg script selection ([#1881](https://github.com/windmill-labs/windmill/issues/1881)) ([df5a4db](https://github.com/windmill-labs/windmill/commit/df5a4dbdc877ef4f8fd0c105d8bbc8a5d601eeb3))
* **frontend:** fix payload query parameter in get by path webhook ([#1875](https://github.com/windmill-labs/windmill/issues/1875)) ([e5027cd](https://github.com/windmill-labs/windmill/commit/e5027cd9a38685cd7ee9ac8f67514524dda2cffc))

## [1.131.0](https://github.com/windmill-labs/windmill/compare/v1.130.0...v1.131.0) (2023-07-14)


### Features

* **frontend:** add missing link to job run page ([#1878](https://github.com/windmill-labs/windmill/issues/1878)) ([b3d61ad](https://github.com/windmill-labs/windmill/commit/b3d61ad67865128114f2c58491aa99f87189dc8c))
* **frontend:** add modal component controls ([#1877](https://github.com/windmill-labs/windmill/issues/1877)) ([c0e1852](https://github.com/windmill-labs/windmill/commit/c0e18526987b07373e73566118cb7edf2a27ab15))


### Bug Fixes

* fix REST job potential double execution ([70bc56a](https://github.com/windmill-labs/windmill/commit/70bc56a68bdf8d53b5ae6bb8995572509bea954d))
* global cache now cache symlinks ([da9c634](https://github.com/windmill-labs/windmill/commit/da9c6340a2ba4a8aaf1ae5d6c16b05583da6860d))

## [1.130.0](https://github.com/windmill-labs/windmill/compare/v1.129.1...v1.130.0) (2023-07-13)


### Features

* add transformer to background scripts ([8547125](https://github.com/windmill-labs/windmill/commit/85471252a5ec136f240048b71e94427bfcacd846))

## [1.129.1](https://github.com/windmill-labs/windmill/compare/v1.129.0...v1.129.1) (2023-07-13)


### Bug Fixes

* add configurable HEADERS for CLI ([53f57e0](https://github.com/windmill-labs/windmill/commit/53f57e027235f36f7678594a9f869072e8439fca))

## [1.129.0](https://github.com/windmill-labs/windmill/compare/v1.128.0...v1.129.0) (2023-07-13)


### Features

* add jumpcloud support for sso ([9fcd37c](https://github.com/windmill-labs/windmill/commit/9fcd37cf436f40e719059843aa27d8bb9d2d70da))
* add powershell to base image ([06d15bf](https://github.com/windmill-labs/windmill/commit/06d15bfa45a78aad5af3cfe874cc445e816982ee))
* **frontend:** Add manual calendar button + add shortcuts ([#1866](https://github.com/windmill-labs/windmill/issues/1866)) ([4017407](https://github.com/windmill-labs/windmill/commit/4017407df545092921c4ef231e90583bac84327b))
* **frontend:** use typed dict for resource types in python ([#1869](https://github.com/windmill-labs/windmill/issues/1869)) ([da70133](https://github.com/windmill-labs/windmill/commit/da701336577049d72375e72e603313114534a63f))
* generate and fix scripts using Autopilot powered by OpenAI [#1827](https://github.com/windmill-labs/windmill/issues/1827)) ([012ea2d](https://github.com/windmill-labs/windmill/commit/012ea2dc0a3ce4685a50d5250b37003f40bfd0c8))
* Per script concurrency limit with time window ([#1816](https://github.com/windmill-labs/windmill/issues/1816)) ([e2fb35a](https://github.com/windmill-labs/windmill/commit/e2fb35a487608c6d5a35896f1fb17a8698d2d552))


### Bug Fixes

* fix initial reactivity double trigger ([dfcb6eb](https://github.com/windmill-labs/windmill/commit/dfcb6eb28467e890664b8f6dc09754a811031ad2))
* **frontend:** Fix App multi select render ([#1867](https://github.com/windmill-labs/windmill/issues/1867)) ([9f1d630](https://github.com/windmill-labs/windmill/commit/9f1d63059be8e744b67d60a0d984591636140528))
* **frontend:** fix conditional portal ([#1868](https://github.com/windmill-labs/windmill/issues/1868)) ([8345b38](https://github.com/windmill-labs/windmill/commit/8345b389a65a86cec296e6544df264b167dfaeab))
* **frontend:** store exists openai key ([#1870](https://github.com/windmill-labs/windmill/issues/1870)) ([16b0e28](https://github.com/windmill-labs/windmill/commit/16b0e281cb785a3820ec6256873c8423449610f3))
* improve bash flushing ([1fc36c9](https://github.com/windmill-labs/windmill/commit/1fc36c9b074d66d615906b6e3bf0b5cd71dde97b))
* make workers bind their http servers on any available port for OCI compliance ([08e3502](https://github.com/windmill-labs/windmill/commit/08e3502126f9727301fc2609740ecfa30beb3e9e))
* Other schedules only display schedules related to script/flow ([2be0714](https://github.com/windmill-labs/windmill/commit/2be071482202ecf295e713339be442f0d0d45b58))

## [1.128.0](https://github.com/windmill-labs/windmill/compare/v1.127.1...v1.128.0) (2023-07-11)


### Features

* add mysql as native integration ([#1859](https://github.com/windmill-labs/windmill/issues/1859)) ([a048e0d](https://github.com/windmill-labs/windmill/commit/a048e0d7e221aa0162d33197566bcd4036da1b67))


### Bug Fixes

* **frontend:** App errors array ([#1851](https://github.com/windmill-labs/windmill/issues/1851)) ([06a8772](https://github.com/windmill-labs/windmill/commit/06a8772dde84a872982e6a1e7d16170c6dc906fe))
* **frontend:** Fix app drawer display + add missing flattent ([#1853](https://github.com/windmill-labs/windmill/issues/1853)) ([4093939](https://github.com/windmill-labs/windmill/commit/4093939936203f2603bb999618f4810d33c3ecb7))
* **frontend:** Fix select width in app table to avoid content jump ([#1850](https://github.com/windmill-labs/windmill/issues/1850)) ([1ebc86c](https://github.com/windmill-labs/windmill/commit/1ebc86c2a7edfb182d1723bf06cbca0058154622))
* **frontend:** only forward css variable ([#1856](https://github.com/windmill-labs/windmill/issues/1856)) ([4034ab0](https://github.com/windmill-labs/windmill/commit/4034ab07df47f1eee5772144879858f64cd7b116))
* **frontend:** Support both copying the key and the value in the ObjectViewer ([#1854](https://github.com/windmill-labs/windmill/issues/1854)) ([f2101c0](https://github.com/windmill-labs/windmill/commit/f2101c05efa5f691f3b3e6d0abcbe1f78082e90f))

## [1.127.1](https://github.com/windmill-labs/windmill/compare/v1.127.0...v1.127.1) (2023-07-10)


### Bug Fixes

* **frontend:** Fix debug runs zIndex ([#1822](https://github.com/windmill-labs/windmill/issues/1822)) ([ce9088e](https://github.com/windmill-labs/windmill/commit/ce9088e7a847834522890ed53c96794773ced491))
* **frontend:** Fix graph view when mulitple graphs are displayed ([#1821](https://github.com/windmill-labs/windmill/issues/1821)) ([5e4e52a](https://github.com/windmill-labs/windmill/commit/5e4e52a10941c83b54da730ed51fc982f44f8ac8))

## [1.127.0](https://github.com/windmill-labs/windmill/compare/v1.126.0...v1.127.0) (2023-07-10)


### Features

* add test connection to resource editor ([9d5cfaf](https://github.com/windmill-labs/windmill/commit/9d5cfafb281c1cc7dd3eb18e5eb7bf9f7423957c))
* **frontend:** add mobile view ([#1819](https://github.com/windmill-labs/windmill/issues/1819)) ([47d211b](https://github.com/windmill-labs/windmill/commit/47d211b21807d688fe631be8c4027285a2932cfc))


### Bug Fixes

* **frontend:** support special chars in postgresql client [[#1775](https://github.com/windmill-labs/windmill/issues/1775)] ([#1818](https://github.com/windmill-labs/windmill/issues/1818)) ([9e385d9](https://github.com/windmill-labs/windmill/commit/9e385d9467a554070e375fc406a6762879a582cb))

## [1.126.0](https://github.com/windmill-labs/windmill/compare/v1.125.1...v1.126.0) (2023-07-09)


### Features

* add support for pg uuid ([79bc1da](https://github.com/windmill-labs/windmill/commit/79bc1da5ea8f0ae0985612515ef99279f93634ff))
* bun support ([#1800](https://github.com/windmill-labs/windmill/issues/1800)) ([2921649](https://github.com/windmill-labs/windmill/commit/2921649c3cc68e4f388c2b81e3707613bc737d1e))
* **frontend:** Fix App Select styles ([#1811](https://github.com/windmill-labs/windmill/issues/1811)) ([5af82e4](https://github.com/windmill-labs/windmill/commit/5af82e4afd2bec68607969eab09510581eda5aeb))
* workspace error handler ([#1799](https://github.com/windmill-labs/windmill/issues/1799)) ([54cd5ce](https://github.com/windmill-labs/windmill/commit/54cd5ce569823df8a4dd391a7267c7aec7435f11))


### Bug Fixes

* **frontend:** add missing required argument to correctly compute isValue ([#1807](https://github.com/windmill-labs/windmill/issues/1807)) ([94a0820](https://github.com/windmill-labs/windmill/commit/94a08209c71899c7ae447bc92ac0f4137cd13f51))
* **frontend:** Fix multi select custom css ([#1813](https://github.com/windmill-labs/windmill/issues/1813)) ([518bf23](https://github.com/windmill-labs/windmill/commit/518bf23005c2d52db6c0dc89ec1356635bbdf32b))
* **frontend:** isValid when no properties ([#1806](https://github.com/windmill-labs/windmill/issues/1806)) ([8e7db51](https://github.com/windmill-labs/windmill/commit/8e7db51cff5ea6f604f52d22db4e0ea0f514b95c))
* **frontend:** unselect ScriptPicker + slack script ([#1802](https://github.com/windmill-labs/windmill/issues/1802)) ([ec6fbab](https://github.com/windmill-labs/windmill/commit/ec6fbabe888d937416030485f8de533ffab908f8))
* update deno to 1.35.0 ([18f4dc0](https://github.com/windmill-labs/windmill/commit/18f4dc079933f160e729586379cc2a55191d0d65))

## [1.125.1](https://github.com/windmill-labs/windmill/compare/v1.125.0...v1.125.1) (2023-07-05)


### Bug Fixes

* fix go and py resolution cache overlap ([5b7c796](https://github.com/windmill-labs/windmill/commit/5b7c7965e5d43e3a0f9d7ad481eb520123a799e0))
* **frontend:** Fix Quill component ([#1797](https://github.com/windmill-labs/windmill/issues/1797)) ([8ece51c](https://github.com/windmill-labs/windmill/commit/8ece51c6888b16019e589d451ac77ea5adce5b82))

## [1.125.0](https://github.com/windmill-labs/windmill/compare/v1.124.0...v1.125.0) (2023-07-04)


### Features

* add groups to app ctx ([499dd5b](https://github.com/windmill-labs/windmill/commit/499dd5b8ea2a7bf0484e2ee472b7f07af9a19b9e))
* improve debug runs wrt to frontend scripts ([dda9920](https://github.com/windmill-labs/windmill/commit/dda99206fa3d9ab31357e5766e2ff56635221759))
* native fetch + native postgresql jobs ([#1796](https://github.com/windmill-labs/windmill/issues/1796)) ([c669e99](https://github.com/windmill-labs/windmill/commit/c669e9940bddb74163bc049e0951b91b7e31c8ed))


### Bug Fixes

* fix global error handler ([f98c199](https://github.com/windmill-labs/windmill/commit/f98c199b63b4428532c2710a0d19215cccd4abbf))
* fix go and python cache resolution conflict ([54c6aed](https://github.com/windmill-labs/windmill/commit/54c6aed31cc1f344a345f19f9aa583cb55c1b944))
* **frontend:** Allow AppSelectTab ([#1787](https://github.com/windmill-labs/windmill/issues/1787)) ([080e244](https://github.com/windmill-labs/windmill/commit/080e2443ab49a101bea819d08b48090a1d988b98))
* **frontend:** Fix script builder ([#1795](https://github.com/windmill-labs/windmill/issues/1795)) ([c6d520b](https://github.com/windmill-labs/windmill/commit/c6d520bb59f7ba204fb448ea95bca1c04311c97d))
* **frontend:** Forked svelte-select to fix overflow issues using a po… ([#1778](https://github.com/windmill-labs/windmill/issues/1778)) ([bd481ad](https://github.com/windmill-labs/windmill/commit/bd481adbfc5dedce0db9ee5ac7bb2097048a767a))
* tooltip and copy button in text ([30b041e](https://github.com/windmill-labs/windmill/commit/30b041e2205ed9e3fbbcd4e7be58e10d84e67d2e))

## [1.124.0](https://github.com/windmill-labs/windmill/compare/v1.123.1...v1.124.0) (2023-06-30)


### Features

* add configurable global error handler ([8c566a2](https://github.com/windmill-labs/windmill/commit/8c566a2e46e5136f6fb3783b6fbb65833b5f202c))

## [1.123.1](https://github.com/windmill-labs/windmill/compare/v1.123.0...v1.123.1) (2023-06-29)


### Bug Fixes

* add CREATE_WORKSPACE_REQUIRE_SUPERADMIN ([ff942f4](https://github.com/windmill-labs/windmill/commit/ff942f4d06ed06877ec2512e6940c346e3484c47))

## [1.123.0](https://github.com/windmill-labs/windmill/compare/v1.122.0...v1.123.0) (2023-06-29)


### Features

* cancel non-yet-running jobs and rework force cancellation ([4763242](https://github.com/windmill-labs/windmill/commit/4763242780fcc65aca857d0e476d19e7ba5f5bb7))
* **frontend:** Add documentation link in the component settings ([#1773](https://github.com/windmill-labs/windmill/issues/1773)) ([3b25fd9](https://github.com/windmill-labs/windmill/commit/3b25fd9748c958e41e84cdbeede0f259fc46593d))
* **frontend:** add resources warning ([#1776](https://github.com/windmill-labs/windmill/issues/1776)) ([a8af158](https://github.com/windmill-labs/windmill/commit/a8af158b9f9c4f0bb3f7d3a7f7d0f86238919d07))
* smtp support to invite users ([#1777](https://github.com/windmill-labs/windmill/issues/1777)) ([7851e93](https://github.com/windmill-labs/windmill/commit/7851e932eca9904c1e192a9bea9ae4002a46fdf2))


### Bug Fixes

* **frontend:** Fix typing ([#1774](https://github.com/windmill-labs/windmill/issues/1774)) ([99d19f6](https://github.com/windmill-labs/windmill/commit/99d19f6c36b6cd03bebb2ca6af01ca506a0cf5cc))
* improve list component force recompute ([13e049a](https://github.com/windmill-labs/windmill/commit/13e049af60d25c8bac05be6c87a850447b1d9d31))

## [1.122.0](https://github.com/windmill-labs/windmill/compare/v1.121.0...v1.122.0) (2023-06-23)


### Features

* release wmillbench publicly ([161f793](https://github.com/windmill-labs/windmill/commit/161f793ae6a67761709d4ced2de060c9546b2d3b))

## [1.121.0](https://github.com/windmill-labs/windmill/compare/v1.120.0...v1.121.0) (2023-06-22)


### Features

* download logs from backend ([7a1f999](https://github.com/windmill-labs/windmill/commit/7a1f999cea6d068d8971a7196fc9ce39e8273aed))
* script versions history ([ee433bd](https://github.com/windmill-labs/windmill/commit/ee433bdd4b00a6b4d45df4332203554682c51bc1))

## [1.120.0](https://github.com/windmill-labs/windmill/compare/v1.119.0...v1.120.0) (2023-06-22)


### Features

* add ability to copy job args ([29a2eeb](https://github.com/windmill-labs/windmill/commit/29a2eeb382b1d9359eb385fc21fc332c861ea2ff))
* add update checker on version info ([f9341af](https://github.com/windmill-labs/windmill/commit/f9341af2feaf3bf2e0681c82350cdf24adfd7e8d))


### Bug Fixes

* **cli:** expose --skip-secrets --skip-variables --skip-resources ([a1b5c14](https://github.com/windmill-labs/windmill/commit/a1b5c142bd1012e83b2f194d073a1d1531753618))

## [1.119.0](https://github.com/windmill-labs/windmill/compare/v1.118.0...v1.119.0) (2023-06-22)


### Features

* **cli:** add skipSecrets, skipVariables, skipResources ([2df29a1](https://github.com/windmill-labs/windmill/commit/2df29a131e2c3a556b50be6c73234ce8e752a7e7))


### Bug Fixes

* bump dependencies ([66ca3f1](https://github.com/windmill-labs/windmill/commit/66ca3f1522b3838707681d553b1612169619bddd))

## [1.118.0](https://github.com/windmill-labs/windmill/compare/v1.117.0...v1.118.0) (2023-06-22)


### Features

* add dynamic args for input list ([05d1b20](https://github.com/windmill-labs/windmill/commit/05d1b20b663a3b0cf38638472fb4f7823d56db4c))
* add preselect first config for app selects ([11c6ff7](https://github.com/windmill-labs/windmill/commit/11c6ff7481f351a0e9549d3ac8e2dbc8ce2ca4d8))
* editable resource types + rt in deployments ([fdb7ab7](https://github.com/windmill-labs/windmill/commit/fdb7ab7f51f739094e785438a5bff45d983556d5))
* resume and approvers available in iterator and branch expr ([a98e146](https://github.com/windmill-labs/windmill/commit/a98e146aedfa39539bd86685dbe9c4f5a7e8f1df))
* step mocking for flows ([4c594c0](https://github.com/windmill-labs/windmill/commit/4c594c0e649d8b416a53823e457e43a029e5f940))


### Bug Fixes

* correctly handle deeply nested results for out-of-order loops ([82f20d3](https://github.com/windmill-labs/windmill/commit/82f20d3ef4fe3c43adc9489d5fe950c3504f2425))

## [1.117.0](https://github.com/windmill-labs/windmill/compare/v1.116.0...v1.117.0) (2023-06-20)


### Features

* add dynamic default args to approval page form ([a4365cb](https://github.com/windmill-labs/windmill/commit/a4365cb864120b3545564871c507c8224a85b749))
* add schema form to approval steps ([59e395a](https://github.com/windmill-labs/windmill/commit/59e395a92ad13a1d2d09d4f6bbdc400257087c22))
* list component for apps ([#1740](https://github.com/windmill-labs/windmill/issues/1740)) ([dd03f33](https://github.com/windmill-labs/windmill/commit/dd03f33337c2787b56981ad1c6e1b7200c94376a))


### Bug Fixes

* make postgresql attempt to create users regardless of if superadmin or not ([6dabc93](https://github.com/windmill-labs/windmill/commit/6dabc933890709746aab83cbbd0cad41a42723bc))
* remove __index from aggrid ([258943c](https://github.com/windmill-labs/windmill/commit/258943cb8590f51e1af725b68ea727705288ac93))

## [1.116.0](https://github.com/windmill-labs/windmill/compare/v1.115.0...v1.116.0) (2023-06-19)


### Features

* add delete draft from home ([4b7f681](https://github.com/windmill-labs/windmill/commit/4b7f681e5a0a87a0e6922595b1e5aa7d142b4415))
* add diff viewer to script autosave discard menu ([80c07ad](https://github.com/windmill-labs/windmill/commit/80c07ad905c51a1e247d95238126a10a9d2bab75))
* add enums to array args ([1060d32](https://github.com/windmill-labs/windmill/commit/1060d3271cb5ed3f7bc518a2baf8bf1dbbabf971))


### Bug Fixes

* deploy dev/staging/prod small fixes ([848c03b](https://github.com/windmill-labs/windmill/commit/848c03ba50cd4e7643791644443778073f92b95c))

## [1.115.0](https://github.com/windmill-labs/windmill/compare/v1.114.2...v1.115.0) (2023-06-18)


### Features

* add dataflow view for workflows ([d31959b](https://github.com/windmill-labs/windmill/commit/d31959b30b6b888d5ae0c75d24311bb4a555a7e6))
* add dataflow view for workflows ([d7d5bce](https://github.com/windmill-labs/windmill/commit/d7d5bce499fb65091692926d65e47fadcc6c7bb0))
* add extra config to aggrid ([1a75641](https://github.com/windmill-labs/windmill/commit/1a75641d08fd94344036fec963b2a0c70274191c))
* dev/staging/prod and deploy from web ([#1733](https://github.com/windmill-labs/windmill/issues/1733)) ([ac1a432](https://github.com/windmill-labs/windmill/commit/ac1a432bb9a7033a068fe77c92ffb54e3ec43806))
* **frontend:** vscode extension dark mode ([#1730](https://github.com/windmill-labs/windmill/issues/1730)) ([157d722](https://github.com/windmill-labs/windmill/commit/157d722c1e7bc3ee3c3c902543b622976504ca62))
* new default encoding for resource types in deno ([a16798b](https://github.com/windmill-labs/windmill/commit/a16798b4d666dfc074088b31d423e935abcdfc6f))


### Bug Fixes

* autosize app inputs ([5210150](https://github.com/windmill-labs/windmill/commit/5210150722ead3015f6ce4ee8a6f3ec7c9dec7eb))
* flow editor design improvements ([d87e5ea](https://github.com/windmill-labs/windmill/commit/d87e5ea4fe3b5356275ff8268ed7ea3ab063679c))
* flow editor design improvements ([eafb6ed](https://github.com/windmill-labs/windmill/commit/eafb6edb45ffcacc8cb30748df5cc33093e98699))
* flow viewer ([6ccbf2d](https://github.com/windmill-labs/windmill/commit/6ccbf2d791ff4bbf47a836221fc48923fa321d3e))
* improve agGrid persistence when result change + setSelectedIndex ([fe9c757](https://github.com/windmill-labs/windmill/commit/fe9c757add83747bbec304a8ea0f5f775a19d1d9))
* infer schema for script without schema in flows ([2db5337](https://github.com/windmill-labs/windmill/commit/2db533774cb1e6dd7dc9f3317c1441294c623724))

## [1.114.2](https://github.com/windmill-labs/windmill/compare/v1.114.1...v1.114.2) (2023-06-12)


### Bug Fixes

* improve dev cli ([afce4ef](https://github.com/windmill-labs/windmill/commit/afce4ef77aa598b2ed7c6785ee7ca61a89eb64ab))

## [1.114.1](https://github.com/windmill-labs/windmill/compare/v1.114.0...v1.114.1) (2023-06-12)


### Bug Fixes

* fix app button form modal ([a121ca0](https://github.com/windmill-labs/windmill/commit/a121ca08759194dd33fed6b034c84aea8ce4703c))
* fix use input from input library + make selected subgrid clearer ([e942c43](https://github.com/windmill-labs/windmill/commit/e942c437cba3dd5e02ebf7f25173442cc14a6236))
* **frontend:** Fix text input ([#1712](https://github.com/windmill-labs/windmill/issues/1712)) ([f495cf0](https://github.com/windmill-labs/windmill/commit/f495cf0b045e99f324d5616ffc0ac826a2aa23fa))

## [1.114.0](https://github.com/windmill-labs/windmill/compare/v1.113.2...v1.114.0) (2023-06-12)


### Features

* remove the need for BASE_INTERNAL_URL ([263e03c](https://github.com/windmill-labs/windmill/commit/263e03c2bd508dd94ae6f30fd4cbc67b416b7ef4))

## [1.113.2](https://github.com/windmill-labs/windmill/compare/v1.113.1...v1.113.2) (2023-06-12)


### Bug Fixes

* correct schedule jobs ordering + avoid cdn for quill css ([7418923](https://github.com/windmill-labs/windmill/commit/7418923e950f376e94a6d7c9235c62f6d83f44e5))

## [1.113.1](https://github.com/windmill-labs/windmill/compare/v1.113.0...v1.113.1) (2023-06-12)


### Bug Fixes

* fix retrieving last jobs of schedules ([e266337](https://github.com/windmill-labs/windmill/commit/e2663371d5a4c0b6bb27546c9847ea7707f64536))

## [1.113.0](https://github.com/windmill-labs/windmill/compare/v1.112.0...v1.113.0) (2023-06-12)


### Features

* add rich text editor as component to apps (quill) ([1a7aa4c](https://github.com/windmill-labs/windmill/commit/1a7aa4cda31426f0a960cc243c8c5d0da7065e8d))
* rework schedule page entirely to display jobs informations ([4963286](https://github.com/windmill-labs/windmill/commit/4963286edde771a9e6aa17b1f105060828cb1ebc))

## [1.112.0](https://github.com/windmill-labs/windmill/compare/v1.111.3...v1.112.0) (2023-06-10)


### Features

* local dev page on the web and compatible with vscode extension ([8342ed8](https://github.com/windmill-labs/windmill/commit/8342ed855b5d8576760b6df7efa10ef299615211))


### Bug Fixes

* pip install repsect proxy settings ([ebb6311](https://github.com/windmill-labs/windmill/commit/ebb631190d3cf537d82c11b11e892afbfd16e4ed))
* use app for dev setup directly ([8b6e5a3](https://github.com/windmill-labs/windmill/commit/8b6e5a347e13311637f4e4f4205a5d3f758e8445))

## [1.111.3](https://github.com/windmill-labs/windmill/compare/v1.111.2...v1.111.3) (2023-06-09)


### Bug Fixes

* add NO_PROXY and make pip respect proxy args ([b6a037a](https://github.com/windmill-labs/windmill/commit/b6a037aa049ae71924df6c9a7b8abf2b9d5e9210))

## [1.111.2](https://github.com/windmill-labs/windmill/compare/v1.111.1...v1.111.2) (2023-06-09)


### Bug Fixes

* add job execution time and mem everywhere applicable ([98d6b21](https://github.com/windmill-labs/windmill/commit/98d6b21b309ac5d7f2fb8677f69ab2ea66c560d7))
* add more options to aggrid ([2e190f3](https://github.com/windmill-labs/windmill/commit/2e190f3c0b2a28f1ec0d69300684734458930096))
* add support for http_proxy and https_proxy ([67b3b06](https://github.com/windmill-labs/windmill/commit/67b3b0635b4e04eefdd1f23081a1e04d9818ff5c))
* toggle self-signed certs support in oauth2 using env variable ACCEPT_INVALID_CERTS ([#1694](https://github.com/windmill-labs/windmill/issues/1694)) ([bfe88de](https://github.com/windmill-labs/windmill/commit/bfe88def346e5de14f68104c6d8ea138d63ac83e))

## [1.111.1](https://github.com/windmill-labs/windmill/compare/v1.111.0...v1.111.1) (2023-06-09)


### Bug Fixes

* add cancel button to flowpreview ([6b50a2b](https://github.com/windmill-labs/windmill/commit/6b50a2bb6d5076919b28569ce498068fae042813))

## [1.111.0](https://github.com/windmill-labs/windmill/compare/v1.110.0...v1.111.0) (2023-06-09)


### Features

* wmill dev v0 ([ee77bee](https://github.com/windmill-labs/windmill/commit/ee77bee80f3da75f0be3ab6586f4fabc140bf760))

### Bug Fixes

* be more specific about replacing nan in python deser ([9cd73ab](https://github.com/windmill-labs/windmill/commit/9cd73ab32bdc64029445aad4bae634e945393923))

## [1.110.0](https://github.com/windmill-labs/windmill/compare/v1.109.1...v1.110.0) (2023-06-07)


### Features

* add suggested results to prop picker ([67b05d3](https://github.com/windmill-labs/windmill/commit/67b05d38719714fd219977bee02b13b0ce1a0a77))
* **apps:** copy paste across apps ([7f81abd](https://github.com/windmill-labs/windmill/commit/7f81abd545f0261e366963cf9ae8c41c485ee749))
* deleting a flow step show confirmation modal with refs ([c7fac8c](https://github.com/windmill-labs/windmill/commit/c7fac8c6d282d8f513971ed05fb552c338368bde))
* migrate ts parser to wasm ([#1686](https://github.com/windmill-labs/windmill/issues/1686)) ([c702f40](https://github.com/windmill-labs/windmill/commit/c702f40980a397319aa02de3f67176a2762651f4))
* support custom env variables ([#1675](https://github.com/windmill-labs/windmill/issues/1675)) ([98e1fdd](https://github.com/windmill-labs/windmill/commit/98e1fdd898f916f71c4e07e1029fb828a9891bbd))


### Bug Fixes

* empty flows not return their inputs ([253fd91](https://github.com/windmill-labs/windmill/commit/253fd910249a58db4697d67233fb1b2ba558090e))

## [1.109.1](https://github.com/windmill-labs/windmill/compare/v1.109.0...v1.109.1) (2023-06-04)


### Bug Fixes

* fix go-client generation ([a0401ac](https://github.com/windmill-labs/windmill/commit/a0401ac8f12782277674dfce7b32b292d33de8bc))

## [1.109.0](https://github.com/windmill-labs/windmill/compare/v1.108.2...v1.109.0) (2023-06-04)


### Features

* add cache as a primitive for flows ([#1671](https://github.com/windmill-labs/windmill/issues/1671)) ([7e466b7](https://github.com/windmill-labs/windmill/commit/7e466b771565207344365068e09d784b2ea31473))

## [1.108.2](https://github.com/windmill-labs/windmill/compare/v1.108.1...v1.108.2) (2023-06-03)


### Bug Fixes

* improve websockets handling for flow editor ([ce94426](https://github.com/windmill-labs/windmill/commit/ce944264415cea66f90a5448fc90de6b7d2184e4))
* optimize object viewer to handle large data ([ae5b11a](https://github.com/windmill-labs/windmill/commit/ae5b11aba5b6e1be141c51afdfc3c4918b118126))

## [1.108.1](https://github.com/windmill-labs/windmill/compare/v1.108.0...v1.108.1) (2023-06-02)


### Bug Fixes

* **frontend:** Fix currency input ([#1667](https://github.com/windmill-labs/windmill/issues/1667)) ([3e7dd0d](https://github.com/windmill-labs/windmill/commit/3e7dd0d179cc516a8bb68b9435bded48df0c405a))
* renaming app + improve flow rendering ([f7e23ac](https://github.com/windmill-labs/windmill/commit/f7e23acfdcd19e0af19b5d6416a2843d72e3a067))

## [1.108.0](https://github.com/windmill-labs/windmill/compare/v1.107.0...v1.108.0) (2023-05-31)


### Features

* add app presence ([e9fe595](https://github.com/windmill-labs/windmill/commit/e9fe595de40deca44cde1b26a5654caa6919094d))
* add multiplayer support for webeditor ([#1562](https://github.com/windmill-labs/windmill/issues/1562)) ([428e0ab](https://github.com/windmill-labs/windmill/commit/428e0ab2f8632dc7a6cefb83f2d3c5d8d1c4508a))


### Bug Fixes

* **frontend:** Fix app table actions ([#1665](https://github.com/windmill-labs/windmill/issues/1665)) ([1634ee6](https://github.com/windmill-labs/windmill/commit/1634ee635ed8400dc67683395449d7b7448a073b))

## [1.107.0](https://github.com/windmill-labs/windmill/compare/v1.106.0...v1.107.0) (2023-05-29)


### Features

* **backend:** webhook specific tokens ([8c33599](https://github.com/windmill-labs/windmill/commit/8c335996631b7512e7699ffd0aebe04e43c498ab))


### Bug Fixes

* **backend:** fix initial worker ping issue ([1816252](https://github.com/windmill-labs/windmill/commit/1816252f03cb4c45a1211f1b2641f79bc679421f))

## [1.106.1](https://github.com/windmill-labs/windmill/compare/v1.106.0...v1.106.1) (2023-05-29)


### Bug Fixes

* **backend:** fix initial worker ping issue ([1816252](https://github.com/windmill-labs/windmill/commit/1816252f03cb4c45a1211f1b2641f79bc679421f))

## [1.106.0](https://github.com/windmill-labs/windmill/compare/v1.105.0...v1.106.0) (2023-05-28)


### Features

* **apps:** add setValue to frontend script's SDK ([8c9b080](https://github.com/windmill-labs/windmill/commit/8c9b080875cc734d37621bc140b2c2fad135edbb))
* **cli:** add resolveDefaultResource ([e19fabb](https://github.com/windmill-labs/windmill/commit/e19fabb02ff9a3d4044c5a208a2f8d0692c0aa81))

## [1.105.0](https://github.com/windmill-labs/windmill/compare/v1.104.2...v1.105.0) (2023-05-27)


### Features

* **apps:** added deployment history browser ([7cb1d12](https://github.com/windmill-labs/windmill/commit/7cb1d12d4ea9c82b96a759878af77a96b5222ad1))
* **cli:** add variables add to CLI ([6f1d5c4](https://github.com/windmill-labs/windmill/commit/6f1d5c497f52004342234c226d2e36bd3f11b915))


### Bug Fixes

* **cli:** expose an encrypt value endpoint ([1fff16b](https://github.com/windmill-labs/windmill/commit/1fff16bbb8e71566155d860a7c5f768b2aedbede))
* **frontend:** Check whether the source has the right type ([#1647](https://github.com/windmill-labs/windmill/issues/1647)) ([7fd5543](https://github.com/windmill-labs/windmill/commit/7fd5543d1a452466be9515f8b5f8fb709569c77b))

## [1.104.2](https://github.com/windmill-labs/windmill/compare/v1.104.1...v1.104.2) (2023-05-24)


### Bug Fixes

* **python:** fix python execution ([3e19be1](https://github.com/windmill-labs/windmill/commit/3e19be10039ec21f207499361af0920da42607df))

## [1.104.1](https://github.com/windmill-labs/windmill/compare/v1.104.0...v1.104.1) (2023-05-24)


### Bug Fixes

* **cli:** avoid looping infinitely and avoid prompt if interactive ([97b4403](https://github.com/windmill-labs/windmill/commit/97b4403b7aaae80e4801487d7edfce62ccf116da))
* **cli:** fix hub pull ([d892ca5](https://github.com/windmill-labs/windmill/commit/d892ca56b7d9fd4f006dfb9f666995d710036422))
* **cli:** parse schema when pulling from hub ([6851b86](https://github.com/windmill-labs/windmill/commit/6851b86eb5781cc7c652458503be9374f123f53e))
* **frontend:** Fix app toolbar z-index ([#1641](https://github.com/windmill-labs/windmill/issues/1641)) ([42af285](https://github.com/windmill-labs/windmill/commit/42af2854b28c4149c1def8f7e60c9cb4360a7182))

## [1.104.0](https://github.com/windmill-labs/windmill/compare/v1.103.0...v1.104.0) (2023-05-24)


### Features

* schedule error handler ([#1636](https://github.com/windmill-labs/windmill/issues/1636)) ([34048f9](https://github.com/windmill-labs/windmill/commit/34048f9ea655a0afb1983a169b69b454023ec6a8))


### Bug Fixes

* **cli:** do not rely on x.nest.land ([ad66bfa](https://github.com/windmill-labs/windmill/commit/ad66bfadaf0c3153975f7452779ac664c0d0dd41))
* **python:** handle nan ([de4042e](https://github.com/windmill-labs/windmill/commit/de4042e9dcc813d88ef872f694cf6568b087bd1f))

## [1.103.0](https://github.com/windmill-labs/windmill/compare/v1.102.1...v1.103.0) (2023-05-22)


### Features

* docker as a new supported language ([b8da43d](https://github.com/windmill-labs/windmill/commit/b8da43db2c31225b0ade8cd9995aeacf2c0eae86))
* **frontend:** add flowstatus and log component for apps ([11a52f2](https://github.com/windmill-labs/windmill/commit/11a52f2d593a9b233fd138c7af52fc34fa1e6173))
* **frontend:** add plain chartjs component ([#1621](https://github.com/windmill-labs/windmill/issues/1621)) ([eb99b73](https://github.com/windmill-labs/windmill/commit/eb99b73346a02993fcaeb6df906fcaf663db259d))
* **frontend:** disable tabs ([#1623](https://github.com/windmill-labs/windmill/issues/1623)) ([5905d3b](https://github.com/windmill-labs/windmill/commit/5905d3b103b0d1466c4d11b248aec9adbe3bfaad))

## [1.102.1](https://github.com/windmill-labs/windmill/compare/v1.102.0...v1.102.1) (2023-05-21)


### Bug Fixes

* add setVariable to deno-client ([501bb11](https://github.com/windmill-labs/windmill/commit/501bb11d9676439062be7a96c9f6655c2b609ee1))

## [1.102.0](https://github.com/windmill-labs/windmill/compare/v1.101.1...v1.102.0) (2023-05-19)


### Features

* add  ability to pass the full raw body ([#1611](https://github.com/windmill-labs/windmill/issues/1611)) ([b91f7d5](https://github.com/windmill-labs/windmill/commit/b91f7d501390358b01b6656297f56a9f24ef4683))
* add GOPROXY + fix on saved inputs ([cdc4f29](https://github.com/windmill-labs/windmill/commit/cdc4f29ec1231820cfb2e0882d167b7dae3ae06e))
* **backend:** add REQUEST_SIZE_LIMIT env variable ([1cbd704](https://github.com/windmill-labs/windmill/commit/1cbd704a257bcf2bd7b344958104e5c626c52a79))
* **backend:** non mapped values are passed as arg 'body' ([a13d283](https://github.com/windmill-labs/windmill/commit/a13d2832d47d262f0b3ac222a8eb889fb17c75ad))
* expose a react sdk to integrate windmill into react apps ([#1605](https://github.com/windmill-labs/windmill/issues/1605)) ([632be3b](https://github.com/windmill-labs/windmill/commit/632be3b8fb547ca4a2d976f868ee931218b653b3))
* **frontend:** add presets components ([#1589](https://github.com/windmill-labs/windmill/issues/1589)) ([f7338c9](https://github.com/windmill-labs/windmill/commit/f7338c9c9a4cfa10d9c22d32a5ae70c4e3504ef3))
* **lsp:** add  black + ruff + shellcheck ([#1597](https://github.com/windmill-labs/windmill/issues/1597)) ([89e55e0](https://github.com/windmill-labs/windmill/commit/89e55e0226d50951c5c99ce789af80ccaa5c1e25))


### Bug Fixes

* **frontend:** Add missing tooltips + multilpe small fix in the app builder ([#1590](https://github.com/windmill-labs/windmill/issues/1590)) ([fff2b5a](https://github.com/windmill-labs/windmill/commit/fff2b5a24abdd70371e2b8a49ff810c217c01bb1))
* **frontend:** Fix inputValue connection to ensure done event is always sent ([#1607](https://github.com/windmill-labs/windmill/issues/1607)) ([f27abec](https://github.com/windmill-labs/windmill/commit/f27abecbaca4be46715ef15216492cb0984fe32b))
* **frontend:** fix pdf header + icon picker ([#1586](https://github.com/windmill-labs/windmill/issues/1586)) ([a1cdf13](https://github.com/windmill-labs/windmill/commit/a1cdf13cb33494457e9f3cba24d5c7398565881f))
* **frontend:** Fix rx ([#1609](https://github.com/windmill-labs/windmill/issues/1609)) ([c687a77](https://github.com/windmill-labs/windmill/commit/c687a775eb8efdb792c495bec72e7e547b82c068))
* **frontend:** Fix the spinning logo position ([#1595](https://github.com/windmill-labs/windmill/issues/1595)) ([94b8bc4](https://github.com/windmill-labs/windmill/commit/94b8bc47380ea537512042ae412a3ace1ef709e7))
* rework multiselect as app component ([#1599](https://github.com/windmill-labs/windmill/issues/1599)) ([85576b0](https://github.com/windmill-labs/windmill/commit/85576b00836225656b88c6751fdc619034b1ebca))

## [1.101.1](https://github.com/windmill-labs/windmill/compare/v1.101.0...v1.101.1) (2023-05-16)


### Bug Fixes

* **backend:** make result job endpoints public ([41f2d35](https://github.com/windmill-labs/windmill/commit/41f2d35c971c42b9a4842b1411dd21603cabf084))
* **frontend:** add temp hidden span to compute the text max length ([#1573](https://github.com/windmill-labs/windmill/issues/1573)) ([2a17d60](https://github.com/windmill-labs/windmill/commit/2a17d60caaef11f4b6cce464e1905a52095fe228))
* **frontend:** fix app multi select ([#1574](https://github.com/windmill-labs/windmill/issues/1574)) ([45acdc8](https://github.com/windmill-labs/windmill/commit/45acdc895b6b5047a17e59dfcd9ca8cba2dd234a))

## [1.101.0](https://github.com/windmill-labs/windmill/compare/v1.100.2...v1.101.0) (2023-05-15)


### Features

* **backend:** add job_id as a query arg to force set the new job_id ([b6c0018](https://github.com/windmill-labs/windmill/commit/b6c0018e2acaaed324832dfc715853ea58a4a268))
* **frontend:** stepper standalone ([#1558](https://github.com/windmill-labs/windmill/issues/1558)) ([ad6e967](https://github.com/windmill-labs/windmill/commit/ad6e967205550b86cc8744f1ce08bb86215ce3e6))


### Bug Fixes

* **frontend:** Handle empty required in SchemaForm ([#1571](https://github.com/windmill-labs/windmill/issues/1571)) ([efc4e9c](https://github.com/windmill-labs/windmill/commit/efc4e9ce8a988aacb8e8dda264702dc08d25f7e0))

## [1.100.2](https://github.com/windmill-labs/windmill/compare/v1.100.1...v1.100.2) (2023-05-14)


### Bug Fixes

* **cli:** update wmill script push ([678b574](https://github.com/windmill-labs/windmill/commit/678b574efcae66801a115d576db9d00aa9e4145d))
* discriminate execute apps by component ([908358e](https://github.com/windmill-labs/windmill/commit/908358eb08614d07b5e846630743242b68b9e149))
* transform_inputs now only support single line expressions ([c252b76](https://github.com/windmill-labs/windmill/commit/c252b765f1b1fd38f07cbe06548ca5cbe4047ea1))

## [1.100.1](https://github.com/windmill-labs/windmill/compare/v1.100.0...v1.100.1) (2023-05-12)


### Bug Fixes

* update setup step ([178ed6f](https://github.com/windmill-labs/windmill/commit/178ed6f426020c9966380392088562e27aa77cf3))

## [1.100.0](https://github.com/windmill-labs/windmill/compare/v1.99.0...v1.100.0) (2023-05-12)


### Features

* **frontend:** add download button ([9b9730d](https://github.com/windmill-labs/windmill/commit/9b9730d2b7239827fd8dfe8f46b6bd98d535e8d0))


### Bug Fixes

* **backend:** handle Date type ([5e7e46e](https://github.com/windmill-labs/windmill/commit/5e7e46e0259bfc11e92f2446858ddbe9f1b4b08e))
* **frontend:** apps rendering should not depend on local time ([8e785d8](https://github.com/windmill-labs/windmill/commit/8e785d8ba6da16d06816d0379cadfb899be99b06))
* **frontend:** only download result for apps ([6bbd937](https://github.com/windmill-labs/windmill/commit/6bbd9374cbd2c516dd3b56551103fcfeba01f80f))

## [1.99.0](https://github.com/windmill-labs/windmill/compare/v1.98.0...v1.99.0) (2023-05-10)


### Features

* **backend:** run endpoints also support support x-www-form-urlencoded encoded payloads ([2b57418](https://github.com/windmill-labs/windmill/commit/2b57418427e9417599f9f969cb78088c5166958a))
* **frontend:** add hide refresh button ([ef089ab](https://github.com/windmill-labs/windmill/commit/ef089ab56c5ef493118574076d9512ae3b6a42bf))
* **frontend:** add input library to flow builder ([957fd81](https://github.com/windmill-labs/windmill/commit/957fd81576dfe65326cad2ed8487121e157e0953))
* **frontend:** allow copy pasting nested containers ([742ee3a](https://github.com/windmill-labs/windmill/commit/742ee3a5181fdcfba1f59889a8d99347fd0c4610))
* **frontend:** app on error ([#1556](https://github.com/windmill-labs/windmill/issues/1556)) ([6c2ba05](https://github.com/windmill-labs/windmill/commit/6c2ba053a1a023e454296a5ebf2842abf90362a8))
* **frontend:** App select tabs ([#1557](https://github.com/windmill-labs/windmill/issues/1557)) ([4ad530f](https://github.com/windmill-labs/windmill/commit/4ad530f2f004cb33cbc95d5c3b1591a44f93bdee))
* **frontend:** conditional rendering ([#1555](https://github.com/windmill-labs/windmill/issues/1555)) ([3d371d5](https://github.com/windmill-labs/windmill/commit/3d371d5b6524a4ec0152b15d001d8758900de457))
* increase timeout to 900 ([018b504](https://github.com/windmill-labs/windmill/commit/018b504986a6c36c1e5ecbc5e92a763a6b6e613b))


### Bug Fixes

* **backend:** run endpoints also support support x-www-form-urlencoded encoded payloads ([5601d04](https://github.com/windmill-labs/windmill/commit/5601d047fe4736a996d064dc8ff34af5d70706a5))

## [1.98.0](https://github.com/windmill-labs/windmill/compare/v1.97.0...v1.98.0) (2023-05-09)


### Features

* **frontend:** if member of a single workspace, autoset at login ([2dfb74e](https://github.com/windmill-labs/windmill/commit/2dfb74e7e45b279f5169ac89b483ed336e0bd109))


### Bug Fixes

* **backend:** grant all on raw_app ([c62670f](https://github.com/windmill-labs/windmill/commit/c62670f735da8378a896b538f3c3afeef100f7ab))

## [1.97.0](https://github.com/windmill-labs/windmill/compare/v1.96.3...v1.97.0) (2023-05-09)


### Features

* **backend:** add windmill_status_code to run_wait_result ([38ec7d3](https://github.com/windmill-labs/windmill/commit/38ec7d3a857a19e474f0a1b07b73b85aa5f10f41))
* **backend:** cache hub scripts in the worker cache ([7537f1a](https://github.com/windmill-labs/windmill/commit/7537f1a1d7162610f78a7e84a53d57f8478a5965))
* **backend:** in python, if a value is bytes, it is encoded to base64 automaticaly ([6b5ceed](https://github.com/windmill-labs/windmill/commit/6b5ceed6525d4251517627351a15a4fe604629fc))


### Bug Fixes

* **lsp:** handle write_message errors ([9392890](https://github.com/windmill-labs/windmill/commit/939289030ba667afc9b517dfdc90f26378fa44a6))

## [1.96.3](https://github.com/windmill-labs/windmill/compare/v1.96.2...v1.96.3) (2023-05-08)


### Bug Fixes

* **cli:** add folder listing ([c598083](https://github.com/windmill-labs/windmill/commit/c5980839251cdc759c5afc688e7084f5d58ad57f))
* **cli:** show diffs only with --show-diffs ([d254088](https://github.com/windmill-labs/windmill/commit/d254088fce00091352ad95888deeaf88bc6c9d6f))
* **cli:** show diffs only with --show-diffs ([37f08e9](https://github.com/windmill-labs/windmill/commit/37f08e9357c7a74a1c6cdc18bad2d6dc4de5d33d))
* **cli:** variable updating ([2639250](https://github.com/windmill-labs/windmill/commit/2639250b43c3c55f5b9a43f4020fc2f0747e792b))

## [1.96.2](https://github.com/windmill-labs/windmill/compare/v1.96.1...v1.96.2) (2023-05-08)


### Bug Fixes

* **cli:** add debug mode to CLI + improve error output ([8f1cdf1](https://github.com/windmill-labs/windmill/commit/8f1cdf1d61adf80bf0d7c4a5160fd3085d3814ac))

## [1.96.1](https://github.com/windmill-labs/windmill/compare/v1.96.0...v1.96.1) (2023-05-08)


### Bug Fixes

* **cli:** fix cli folder sync ([239f401](https://github.com/windmill-labs/windmill/commit/239f40199955d47e4943be4c72c3d150a58f2dd9))
* **cli:** fix cli folder sync ([a90514b](https://github.com/windmill-labs/windmill/commit/a90514b8e99a419c26512eb370895322088b6aa9))

## [1.96.0](https://github.com/windmill-labs/windmill/compare/v1.95.1...v1.96.0) (2023-05-08)


### Features

* add support for full fleged apps (react, svelte, vue) ([#1536](https://github.com/windmill-labs/windmill/issues/1536)) ([13242ab](https://github.com/windmill-labs/windmill/commit/13242abff153b021cac1ecaa3cbf65ae9d87fb69))
* **frontend:** Add a custom deepEqualWithOrderedArray to handle orde… ([#1537](https://github.com/windmill-labs/windmill/issues/1537)) ([3a291f7](https://github.com/windmill-labs/windmill/commit/3a291f7108623b5c7194f0a7f6a3774499669313))
* **frontend:** Add label, description, input style + add displayType… ([#1540](https://github.com/windmill-labs/windmill/issues/1540)) ([bef829d](https://github.com/windmill-labs/windmill/commit/bef829d4805bb6c5330b13dc17c9a89a84ad48ca))
* **frontend:** app modal ([#1518](https://github.com/windmill-labs/windmill/issues/1518)) ([686f5bb](https://github.com/windmill-labs/windmill/commit/686f5bbe1847cb2a92678e7cfdbb51ecf6bbe2b6))

## [1.95.1](https://github.com/windmill-labs/windmill/compare/v1.95.0...v1.95.1) (2023-05-06)


### Bug Fixes

* **cli:** cli flow sync improvements ([e585e3a](https://github.com/windmill-labs/windmill/commit/e585e3aea2b18b6dc0c9fa7ffa1e6c1dfb2a3ce2))

## [1.95.0](https://github.com/windmill-labs/windmill/compare/v1.94.0...v1.95.0) (2023-05-05)


### Features

* **backend:** default parameters are used in python if missing from args ([8791a86](https://github.com/windmill-labs/windmill/commit/8791a86a936301d44ae05ea09d26c9815abf8929))
* **frontend:** App Schema Form component ([#1533](https://github.com/windmill-labs/windmill/issues/1533)) ([85c0d93](https://github.com/windmill-labs/windmill/commit/85c0d939f59411d023cd4b173ce11224d3cbc9db))
* **frontend:** App stepper ([#1529](https://github.com/windmill-labs/windmill/issues/1529)) ([15f1c94](https://github.com/windmill-labs/windmill/commit/15f1c947bb233147f7da261fd32054a51a9c6efa))
* **frontend:** Merge run configuration + triggers ([#1530](https://github.com/windmill-labs/windmill/issues/1530)) ([1be4658](https://github.com/windmill-labs/windmill/commit/1be4658150ef20a9f1f0fe57b5f30ba3c2d4d94e))

## [1.94.0](https://github.com/windmill-labs/windmill/compare/v1.93.1...v1.94.0) (2023-05-04)


### Features

* **frontend:** add eval badge + alert ([#1522](https://github.com/windmill-labs/windmill/issues/1522)) ([32f04c7](https://github.com/windmill-labs/windmill/commit/32f04c796856fa48ddc1548752ba1e7a8802083a))


### Bug Fixes

* **backend:** fix python transformers ([a07e3e8](https://github.com/windmill-labs/windmill/commit/a07e3e84386c0895a7209fc87a4b07218271feca))
* **frontend:** fix ArrayStaticInputEditor width ([#1528](https://github.com/windmill-labs/windmill/issues/1528)) ([b423eec](https://github.com/windmill-labs/windmill/commit/b423eec019785a62c279db01fc93eb3fe08f7f1f))
* **frontend:** fix select width ([#1526](https://github.com/windmill-labs/windmill/issues/1526)) ([f248c09](https://github.com/windmill-labs/windmill/commit/f248c09655889ddace24f451597a56e81443be3c))
* **frontend:** preserve customise arguments ([b4867f1](https://github.com/windmill-labs/windmill/commit/b4867f12bb4f595b5b0e8142ab5d720307ecadd3))

## [1.93.1](https://github.com/windmill-labs/windmill/compare/v1.93.0...v1.93.1) (2023-05-03)


### Bug Fixes

* **cli:** add yaml support for cli ([03e6017](https://github.com/windmill-labs/windmill/commit/03e6017860526784f1a8696eceed5750b25f1c5c))

## [1.93.0](https://github.com/windmill-labs/windmill/compare/v1.92.2...v1.93.0) (2023-05-03)


### Features

* **frontend:** add recompute others to background scripts ([392d0f8](https://github.com/windmill-labs/windmill/commit/392d0f8b876c9b587fe85421098f3eceb8a74dec))


### Bug Fixes

* **frontend:** deploy path for apps ([7ac9677](https://github.com/windmill-labs/windmill/commit/7ac96771a5c3d44234c790e8cea3d621d8c1d00e))

## [1.93.0](https://github.com/windmill-labs/windmill/compare/v1.92.2...v1.93.0) (2023-05-03)


### Features

* **frontend:** add recompute others to background scripts ([392d0f8](https://github.com/windmill-labs/windmill/commit/392d0f8b876c9b587fe85421098f3eceb8a74dec))


### Bug Fixes

* **frontend:** deploy path for apps ([7ac9677](https://github.com/windmill-labs/windmill/commit/7ac96771a5c3d44234c790e8cea3d621d8c1d00e))

## [1.92.2](https://github.com/windmill-labs/windmill/compare/v1.92.1...v1.92.2) (2023-05-02)


### Bug Fixes

* **go-client:** fix go-client gen ([82c4d66](https://github.com/windmill-labs/windmill/commit/82c4d6629e134f00389c87a948c52878e5a3f4f5))

## [1.92.1](https://github.com/windmill-labs/windmill/compare/v1.92.0...v1.92.1) (2023-05-02)


### Bug Fixes

* **go-client:** fix go-client gen ([df333d9](https://github.com/windmill-labs/windmill/commit/df333d9739f601714f7a0124f47422dfb113d320))

## [1.92.0](https://github.com/windmill-labs/windmill/compare/v1.91.0...v1.92.0) (2023-05-02)


### Features

* **frontend:** add labels as table action ([64065c1](https://github.com/windmill-labs/windmill/commit/64065c17f305fb4c7c078c7fa4935d5423da8f66))
* **frontend:** add labels as table action ([2ab1714](https://github.com/windmill-labs/windmill/commit/2ab1714dfa7cffc46b4b6aa40dabdd92c5a6270f))
* **frontend:** allow running eval in every field ([62acbb5](https://github.com/windmill-labs/windmill/commit/62acbb5ab3a0727b306e25a80b74ac8216619501))
* **frontend:** background script can run script and flows ([#1515](https://github.com/windmill-labs/windmill/issues/1515)) ([607c803](https://github.com/windmill-labs/windmill/commit/607c803be91921b53f329a2c2c3c129ce53d6c0c))


### Bug Fixes

* **frontend:** fix small ui issues ([#1513](https://github.com/windmill-labs/windmill/issues/1513)) ([f6ff8ca](https://github.com/windmill-labs/windmill/commit/f6ff8ca232f5725f86a36379956da2731def2580))

## [1.91.0](https://github.com/windmill-labs/windmill/compare/v1.90.0...v1.91.0) (2023-05-01)


### Features

* add drafts for apps ([f7374c8](https://github.com/windmill-labs/windmill/commit/f7374c8204f85b4371e61f34dcd4b66857c0f8ab))
* introduce backend persisted draft systems for scripts ([88e37fe](https://github.com/windmill-labs/windmill/commit/88e37fe0bed58f396690622e925d5e078c60140c))
* introduce draft for flows ([a196642](https://github.com/windmill-labs/windmill/commit/a1966427e893dc8a58c8f2862ded752884843813))

## [1.90.0](https://github.com/windmill-labs/windmill/compare/v1.89.0...v1.90.0) (2023-04-28)


### Features

* **backend:** add EXIT_AFTER_NO_JOB_FOR_SECS for ephemeral workers ([de9abd1](https://github.com/windmill-labs/windmill/commit/de9abd129db13dcdf0e69e2c1e2d3aa558fb783a))
* **backend:** add JOB_RETENTION_SECS to delete completed jobs completed after expiration period ([0b7bad3](https://github.com/windmill-labs/windmill/commit/0b7bad3816e61841ef4765db7881227274c20b23))
* **backend:** expose tag in the job  ([#1486](https://github.com/windmill-labs/windmill/issues/1486)) ([324d4f5](https://github.com/windmill-labs/windmill/commit/324d4f5e9e89e6de600e882f23bf545c0b1dc539))
* **frontend:** adapt style ([#1488](https://github.com/windmill-labs/windmill/issues/1488)) ([41a24ec](https://github.com/windmill-labs/windmill/commit/41a24ecd36d9cc537cbd1dd0cd1de6f689be1b8c))
* **frontend:** add an eval input component for flow ([#1494](https://github.com/windmill-labs/windmill/issues/1494)) ([2815f1e](https://github.com/windmill-labs/windmill/commit/2815f1ec71177bb6e89d0d62a8df89030d37b1fc))
* **frontend:** Add new integration icons ([#1479](https://github.com/windmill-labs/windmill/issues/1479)) ([7adacd4](https://github.com/windmill-labs/windmill/commit/7adacd4c9f03d17b47abc515bc391e348a7e6ec1))
* **frontend:** refactor inline script ([#1480](https://github.com/windmill-labs/windmill/issues/1480)) ([05c837e](https://github.com/windmill-labs/windmill/commit/05c837e64f61bfb22ae1f80263deb1c879030985))
* **frontend:** Schedules run now ([#1475](https://github.com/windmill-labs/windmill/issues/1475)) ([47f0f35](https://github.com/windmill-labs/windmill/commit/47f0f35236e02958f6bc00b5652e06e25eabeaf5))
* **frontend:** Small style fix ([#1473](https://github.com/windmill-labs/windmill/issues/1473)) ([7ad496a](https://github.com/windmill-labs/windmill/commit/7ad496ad3f746ffd782856a35c1456999792fa94))
* **frontend:** Support TS union type with a select field ([#1457](https://github.com/windmill-labs/windmill/issues/1457)) ([8b76324](https://github.com/windmill-labs/windmill/commit/8b763249cb1360c122cc81d50a1a95d1ad3ddd5b))


### Bug Fixes

* **frontend:** Allow 0 as select default value ([#1474](https://github.com/windmill-labs/windmill/issues/1474)) ([d8529ff](https://github.com/windmill-labs/windmill/commit/d8529ff3ed6168de60bb24626a3e36ab4beae15c))
* **frontend:** close the modal before deleting a form modal ([#1484](https://github.com/windmill-labs/windmill/issues/1484)) ([430c733](https://github.com/windmill-labs/windmill/commit/430c73399b7e1524ee81d0ce8d7d2eaf16117f9a))
* **frontend:** fix apply connection ([#1487](https://github.com/windmill-labs/windmill/issues/1487)) ([cf59cc0](https://github.com/windmill-labs/windmill/commit/cf59cc04efb853fe06a23042115128689b5d26ee))
* **frontend:** Fix frontend script ([#1476](https://github.com/windmill-labs/windmill/issues/1476)) ([b60a7f6](https://github.com/windmill-labs/windmill/commit/b60a7f63d04b1fc851479060e77613c77d20198a))
* **frontend:** fix recomputa all ([#1491](https://github.com/windmill-labs/windmill/issues/1491)) ([fb05a09](https://github.com/windmill-labs/windmill/commit/fb05a09955f937000fbba0826c03c52e65aa146e))
* **frontend:** Flow editor design updates ([#1477](https://github.com/windmill-labs/windmill/issues/1477)) ([50d814c](https://github.com/windmill-labs/windmill/commit/50d814c3dc55841b16cd1df8ae86d021de4e880c))
* **frontend:** Minor app editor updates ([#1458](https://github.com/windmill-labs/windmill/issues/1458)) ([8fd10b1](https://github.com/windmill-labs/windmill/commit/8fd10b1f5813b3f4200a980de4a85e7d701660a7))
* **frontend:** register applyConnection as a callback to remove unnecessary reactivit ([#1485](https://github.com/windmill-labs/windmill/issues/1485)) ([d915f6b](https://github.com/windmill-labs/windmill/commit/d915f6b004ea7cc27a3c55b3504df902f5db1aef))
* **frontend:** reset ui job loading state when submitting preview job triggers error ([#1483](https://github.com/windmill-labs/windmill/issues/1483)) ([6f8616f](https://github.com/windmill-labs/windmill/commit/6f8616fb273b1f4a1489878a617d239f30ecb1c0))
* **frontend:** Update CLI login request styling ([#1454](https://github.com/windmill-labs/windmill/issues/1454)) ([c77393c](https://github.com/windmill-labs/windmill/commit/c77393c15444868308b44a72bee89e49fc23d80f))
* **frontend:** Update direct exports ([#1456](https://github.com/windmill-labs/windmill/issues/1456)) ([4a2af13](https://github.com/windmill-labs/windmill/commit/4a2af1359ee29001236591f685cacfa9df6715df))

## [1.89.0](https://github.com/windmill-labs/windmill/compare/v1.88.1...v1.89.0) (2023-04-23)


### Features

* **backend:** global cache refactor for pip using tar for each dependency ([#1443](https://github.com/windmill-labs/windmill/issues/1443)) ([369dd0d](https://github.com/windmill-labs/windmill/commit/369dd0dac61e5430856ed9abf7129bbad3b75860))
* **backend:** only run fully deployed scripts ([3d031c7](https://github.com/windmill-labs/windmill/commit/3d031c701705459f418b11d2ca83e71943e4079b))
* **backend:** worker groups ([#1452](https://github.com/windmill-labs/windmill/issues/1452)) ([722783f](https://github.com/windmill-labs/windmill/commit/722783f7f630e3123ffd2605deb21a915188bd20))
* **backend:** workers are instantly ready and sync with global cache in background ([670ba51](https://github.com/windmill-labs/windmill/commit/670ba51d9bd9a9a0b07a2ed064c316234bb5819d))
* **ee:** sync cache in background ([c919827](https://github.com/windmill-labs/windmill/commit/c919827cf8eb9437b6d9bd57b3d3ad883a66de3b))
* **ee:** sync cache in background ([0e77e37](https://github.com/windmill-labs/windmill/commit/0e77e37fbddbb517d5e1b1f07a27a40b63371439))
* **frontend:** Add documentation links ([#1399](https://github.com/windmill-labs/windmill/issues/1399)) ([36acbf7](https://github.com/windmill-labs/windmill/commit/36acbf793b6714dffe4dbb0e2501b9438f034858))
* **frontend:** Add seconds input ([#1445](https://github.com/windmill-labs/windmill/issues/1445)) ([30bf7ad](https://github.com/windmill-labs/windmill/commit/30bf7ad3e9785420b4dcd814ce3c7a444d23cc9f))
* **frontend:** add toast actions ([#1411](https://github.com/windmill-labs/windmill/issues/1411)) ([d173232](https://github.com/windmill-labs/windmill/commit/d17323286a05aa0b1680ef94d7058d2c8902782f))
* **frontend:** reorder array items in app editor ([#1426](https://github.com/windmill-labs/windmill/issues/1426)) ([3615fb2](https://github.com/windmill-labs/windmill/commit/3615fb26fb91d64626d82ace6e6275e424ece832))
* **frontend:** support showing metadata on script add via query param ([#1438](https://github.com/windmill-labs/windmill/issues/1438)) ([3c98452](https://github.com/windmill-labs/windmill/commit/3c98452f50913ef639eba96996f3a6c80508bd63))


### Bug Fixes

* **backend:** avoid potential conflict between pull from tar and background sync ([d76e907](https://github.com/windmill-labs/windmill/commit/d76e90757e209263da7f79fa85052969e7efd63d))
* **backend:** global cache synco only start if all piptars have been downloaded ([5f8a730](https://github.com/windmill-labs/windmill/commit/5f8a730fdfbb9e3d518555f7272a3bb297725f28))
* **frontend:** App color picker overflow issue ([#1449](https://github.com/windmill-labs/windmill/issues/1449)) ([32903d2](https://github.com/windmill-labs/windmill/commit/32903d2839a082d53168bb9177fc88f6ab0ec482))
* **frontend:** fix copy content button width ([#1428](https://github.com/windmill-labs/windmill/issues/1428)) ([d96d4a5](https://github.com/windmill-labs/windmill/commit/d96d4a524edebea65bc602194c8f11f5d69e920a))
* **frontend:** Minor update of app default codes ([#1440](https://github.com/windmill-labs/windmill/issues/1440)) ([fe75aa1](https://github.com/windmill-labs/windmill/commit/fe75aa18f2f27745db35329aa60938694640a8c6))
* **frontend:** Update app default codes ([#1432](https://github.com/windmill-labs/windmill/issues/1432)) ([c8acfbc](https://github.com/windmill-labs/windmill/commit/c8acfbc1ff0f6c23e5a2229ca83a3b09eec826c3))
* **frontend:** Update app mobile preview width ([#1431](https://github.com/windmill-labs/windmill/issues/1431)) ([1764613](https://github.com/windmill-labs/windmill/commit/17646130bcf8cf646a4ccdfa39f9a8791876a137))
* **frontend:** Update flow tooltip z-indexes ([#1433](https://github.com/windmill-labs/windmill/issues/1433)) ([17cb8fc](https://github.com/windmill-labs/windmill/commit/17cb8fc3fa0b39a9750e61ca2731a15bbda690ec))
* **frontend:** Update flow viewer styling ([#1441](https://github.com/windmill-labs/windmill/issues/1441)) ([46a29b5](https://github.com/windmill-labs/windmill/commit/46a29b5d27b8d9f7ea38c1063fc081ed5933db5d))

## [1.88.1](https://github.com/windmill-labs/windmill/compare/v1.88.0...v1.88.1) (2023-04-18)


### Bug Fixes

* **frontend:** fix hub list ([1144329](https://github.com/windmill-labs/windmill/commit/1144329972fb61e2df62873ca1e485c88fabc478))

## [1.88.0](https://github.com/windmill-labs/windmill/compare/v1.87.0...v1.88.0) (2023-04-17)


### Features

* **backend:** install python scripts on save ([cb7e686](https://github.com/windmill-labs/windmill/commit/cb7e686dd95397d5b37edd5aac50b6d1429c4a71))
* **frontend:** Add runs preview popup ([#1405](https://github.com/windmill-labs/windmill/issues/1405)) ([4ab023f](https://github.com/windmill-labs/windmill/commit/4ab023f95085958ab1ad01dc249d308c7ebf423e))
* **frontend:** cancellable inline script editor run ([e828d26](https://github.com/windmill-labs/windmill/commit/e828d2673e62e95e5e1235eeca8107ac7cfb7e45))
* **frontend:** Remove gap when button label is empty ([#1402](https://github.com/windmill-labs/windmill/issues/1402)) ([568f59e](https://github.com/windmill-labs/windmill/commit/568f59eefb104047b8ef063f273fe238075d6407))
* **frontend:** Unify main lists ([#1406](https://github.com/windmill-labs/windmill/issues/1406)) ([48bbbd0](https://github.com/windmill-labs/windmill/commit/48bbbd0e872a12ed1c562a6d14967a2a0f7c4735))
* **frontend:** Update airtable instructions ([#1403](https://github.com/windmill-labs/windmill/issues/1403)) ([7dc7ece](https://github.com/windmill-labs/windmill/commit/7dc7ecef55b465fc096f71fc9de5c8b543136ff7))
* inputs library on run page ([92a2934](https://github.com/windmill-labs/windmill/commit/92a293488e8e58350229931ab69f7924d58474be))


### Bug Fixes

* **backend:** deno uses --no-check ([a5499c2](https://github.com/windmill-labs/windmill/commit/a5499c26f3ebd8b07541a7e0cbf33a7008a8f476))
* **backend:** do not fail on schedule not existing anymore ([a5f6d73](https://github.com/windmill-labs/windmill/commit/a5f6d73f7d53d7af9d285a85460509763263c508))
* **frontend:** Fix app file uploads ([#1408](https://github.com/windmill-labs/windmill/issues/1408)) ([ac489ac](https://github.com/windmill-labs/windmill/commit/ac489ac2da0fbf01f5e2877612c14cfaf1ef79c2))
* **frontend:** fix buttons width ([#1407](https://github.com/windmill-labs/windmill/issues/1407)) ([75a0482](https://github.com/windmill-labs/windmill/commit/75a0482ef046dd7e30f6d6039dbc66880182dc5e))
* **frontend:** fix enum sync ([#1410](https://github.com/windmill-labs/windmill/issues/1410)) ([98060ce](https://github.com/windmill-labs/windmill/commit/98060ce55d5efa59a8989cf9357935976d57650b))
* **frontend:** Handle scheduled runs in preview ([#1413](https://github.com/windmill-labs/windmill/issues/1413)) ([accdc1a](https://github.com/windmill-labs/windmill/commit/accdc1ac59ce9611f66567222a73995d3c0a3f9d))
* **frontend:** Keep selected tab during renaming ([#1409](https://github.com/windmill-labs/windmill/issues/1409)) ([82cd048](https://github.com/windmill-labs/windmill/commit/82cd048ef4d08f31660f6f31a96940676a28996c))
* **frontend:** Queued-running jobs preview ([#1414](https://github.com/windmill-labs/windmill/issues/1414)) ([b2a40a0](https://github.com/windmill-labs/windmill/commit/b2a40a05805344c1c34f2ba917b4cdd52dfffc3f))
* **frontend:** Remove output when deleting a component ([#1397](https://github.com/windmill-labs/windmill/issues/1397)) ([6aa1008](https://github.com/windmill-labs/windmill/commit/6aa100893352870d5a99fdd56d7f1425a221a273))

## [1.87.0](https://github.com/windmill-labs/windmill/compare/v1.86.0...v2.0.0) (2023-04-11)


### ⚠ BREAKING CHANGES

* **frontend:** Add option to return file names ([#1380](https://github.com/windmill-labs/windmill/issues/1380))

### Features

* **backend:** add instance events webhook ([f2d3c82](https://github.com/windmill-labs/windmill/commit/f2d3c8208b6daa49f304f355752145de47138a3c))
* **backend:** extend cached resolution for go ([dac61d1](https://github.com/windmill-labs/windmill/commit/dac61d1c982576d7589e16ab01c8cc8bad6e1686))
* **backend:** Redis based queue ([#1324](https://github.com/windmill-labs/windmill/issues/1324)) ([d45e6c9](https://github.com/windmill-labs/windmill/commit/d45e6c94abed609357b18d4daa7de6b2ea0ba978))
* **frontend:** Add option to return file names ([#1380](https://github.com/windmill-labs/windmill/issues/1380)) ([3dabac1](https://github.com/windmill-labs/windmill/commit/3dabac153f302f48210d15ebaec514e72717300f))
* **python:** cache dependency resolution ([facb670](https://github.com/windmill-labs/windmill/commit/facb67093ce7d3b0874d0d559fb272ed822ce360))


### Bug Fixes

* **backend:** nested deno relative imports ([955a213](https://github.com/windmill-labs/windmill/commit/955a213a504c1f3b8811c930823e87fe7dba101a))
* **cli:** overwrite archived scripts ([1f705ca](https://github.com/windmill-labs/windmill/commit/1f705cab2ce8c79829f22fc6af9e06ecba7450b1))
* **frontend:** Add missing stopPropagation ([#1394](https://github.com/windmill-labs/windmill/issues/1394)) ([58d4b55](https://github.com/windmill-labs/windmill/commit/58d4b556ebbd76c6f07f1a16d601a9d824b99f7e))
* **frontend:** fix app init issue ([d0e0e1f](https://github.com/windmill-labs/windmill/commit/d0e0e1fdf27d9a7fb86c66e43398786b64d8b6b7))
* **frontend:** Fix frontend dependencies ([#1379](https://github.com/windmill-labs/windmill/issues/1379)) ([8e9c491](https://github.com/windmill-labs/windmill/commit/8e9c49165060a4a7f831b8be075593f89d867784))
* **frontend:** Fix icon picker input ([#1389](https://github.com/windmill-labs/windmill/issues/1389)) ([8a44f8e](https://github.com/windmill-labs/windmill/commit/8a44f8e7796f13698e2a99af9f3772f5e676604b))
* **frontend:** Fix mac shortcuts ([#1381](https://github.com/windmill-labs/windmill/issues/1381)) ([41831d5](https://github.com/windmill-labs/windmill/commit/41831d58ed593bb283600b76170f6e76783e0eae))
* **frontend:** fix popover configuration to avoid content shift ([#1377](https://github.com/windmill-labs/windmill/issues/1377)) ([2031e1e](https://github.com/windmill-labs/windmill/commit/2031e1ebd0dc020da104ee84a0294c86babcefaf))
* **frontend:** remove stopPropagation that was preventing components dnd ([#1378](https://github.com/windmill-labs/windmill/issues/1378)) ([de8dc1e](https://github.com/windmill-labs/windmill/commit/de8dc1e9cd7beea2ce62656e9e7676214f77a110))


### Performance Improvements

* parallelize more operations for deno jobs ([e911869](https://github.com/windmill-labs/windmill/commit/e911869d990956463834ac9ff35c52ba8236e362))

## [1.86.0](https://github.com/windmill-labs/windmill/compare/v1.85.0...v1.86.0) (2023-04-08)


### Features

* **backend:** add /ready endpoint for workers ([94eecea](https://github.com/windmill-labs/windmill/commit/94eecea02b6295ad5674db4b010bf6ab7984fa17))
* **backend:** add GET endpoint to trigger scripts ([15c75d9](https://github.com/windmill-labs/windmill/commit/15c75d9d00a69ae97123ed371b9657e298345bdb))
* **backend:** lowercase all emails in relevant endpoints ([#1361](https://github.com/windmill-labs/windmill/issues/1361)) ([7f9050b](https://github.com/windmill-labs/windmill/commit/7f9050b285cf8f7f6baf05452b673f58988c452c))
* **cli:** add getFullResource ([3a232db](https://github.com/windmill-labs/windmill/commit/3a232dbb5792c28b26747e1ba260fffcdd4a8416))
* do cache bucket syncing in background + check tar before pushing it ([#1360](https://github.com/windmill-labs/windmill/issues/1360)) ([3e5ff86](https://github.com/windmill-labs/windmill/commit/3e5ff8682a298ba9e59b2662c4c04c5698447204))
* **frontend:** add flow expand button ([34a8b01](https://github.com/windmill-labs/windmill/commit/34a8b01b762c0b210d76101e7da7bd2397258e8d))
* **frontend:** add impersonate api + local resolution of import by lsp v0 ([7675f08](https://github.com/windmill-labs/windmill/commit/7675f08b7bfe319e496a86a7ef1ab7cc8c1d12d2))
* **frontend:** add workspace to ctx ([8f7a11b](https://github.com/windmill-labs/windmill/commit/8f7a11b8964e2c3405ce3689f9cf2298f9e71c75))
* **frontend:** Improve login + toasts ([#1363](https://github.com/windmill-labs/windmill/issues/1363)) ([92be102](https://github.com/windmill-labs/windmill/commit/92be102a070b1f17b9d3e40524cd21b54301b5a7))
* **frontend:** make script editor a single page ([b84be60](https://github.com/windmill-labs/windmill/commit/b84be60c53ca1ef65826123f39099d33c1f549c0))
* **frontend:** Tone down text + display whole text ([#1366](https://github.com/windmill-labs/windmill/issues/1366)) ([f214d5f](https://github.com/windmill-labs/windmill/commit/f214d5f96b6ac26cd3ef90a6ab696a6dfe02b3f0))
* improved cron/schedule editor ([#1362](https://github.com/windmill-labs/windmill/issues/1362)) ([17176bb](https://github.com/windmill-labs/windmill/commit/17176bb8d112b35228ce9183f4b2f81abe9e5b6e))


### Bug Fixes

* **backend:** allow cors ([8a594a8](https://github.com/windmill-labs/windmill/commit/8a594a89adba9915508884f900f58c4ab53cdfec))
* **backend:** allow longer name/company ([eff61bb](https://github.com/windmill-labs/windmill/commit/eff61bb8d3496bc1c5be4b1051f99ed4470a47ff))
* **backend:** always flush bash output ([517b2c9](https://github.com/windmill-labs/windmill/commit/517b2c9cca54628c8ee692d65c05bc2513eaaf22))
* **backend:** always flush bash output ([7a9091f](https://github.com/windmill-labs/windmill/commit/7a9091fed6aa99201b75bab88d4faddbe041eee4))
* **backend:** inline script app python fix ([8c72722](https://github.com/windmill-labs/windmill/commit/8c72722710db8e3720b01180b504cbc66e79f5ca))
* **frontend:** Add FlowGraph display on Safari ([#1351](https://github.com/windmill-labs/windmill/issues/1351)) ([2819b09](https://github.com/windmill-labs/windmill/commit/2819b09ce5011a467e994ee8b1f09cf33145003d))
* **frontend:** Fix button poppup ([#1368](https://github.com/windmill-labs/windmill/issues/1368)) ([a344928](https://github.com/windmill-labs/windmill/commit/a344928f251d697f53e40c517b0b86bd90e0ad52))
* **frontend:** Fix connected property ([#1371](https://github.com/windmill-labs/windmill/issues/1371)) ([4af39f0](https://github.com/windmill-labs/windmill/commit/4af39f081bf3d07aaade39e5a5a221741fe8f973))
* **frontend:** Fix flow templateEditor ([#1367](https://github.com/windmill-labs/windmill/issues/1367)) ([51fc436](https://github.com/windmill-labs/windmill/commit/51fc436456104c2d6a3cd6f6d62f08929e40d450))
* **frontend:** make croninput a builder rather than a tab ([266b5b0](https://github.com/windmill-labs/windmill/commit/266b5b00da3bd7643eaa5dba1b8c1456f11c5e30))
* **frontend:** Minor fixes ([#1374](https://github.com/windmill-labs/windmill/issues/1374)) ([76a2a1d](https://github.com/windmill-labs/windmill/commit/76a2a1db363facbaf9a0e9618f169d6cc66e946f))
* no need to map internal ports to hosts ([#1365](https://github.com/windmill-labs/windmill/issues/1365)) ([4ec035b](https://github.com/windmill-labs/windmill/commit/4ec035b09a58f8859bc576b03c24cc73f335f32d))

## [1.85.0](https://github.com/windmill-labs/windmill/compare/v1.84.1...v1.85.0) (2023-04-03)


### Features

* add local cache for folder path used + invalidate cache on folder creation ([018b051](https://github.com/windmill-labs/windmill/commit/018b051781e3f40b9d1da8ccdd5edb1cd49877ba))
* **frontend:** add agGrid api hooks + ready ([de1e294](https://github.com/windmill-labs/windmill/commit/de1e29492c9aefdfc59f605ba81f7c51a96bf2f3))
* **frontend:** Add ID renaming popup ([#1344](https://github.com/windmill-labs/windmill/issues/1344)) ([0b8a08c](https://github.com/windmill-labs/windmill/commit/0b8a08cb49644da7c354c3631751e925ac5353b9))


### Bug Fixes

* **backend:** improve handling subflow with many depth using tailrec ([8c53598](https://github.com/windmill-labs/windmill/commit/8c53598aba3fb89f4174d1c0ab3912096ac07c96))
* **backend:** improve subflow processing ([390a988](https://github.com/windmill-labs/windmill/commit/390a988d4c96256a4fbd6a9302fc47a5648c2c43))
* **frontend:** PDF reader header positioning ([#1350](https://github.com/windmill-labs/windmill/issues/1350)) ([daf8276](https://github.com/windmill-labs/windmill/commit/daf827666b13917f8c9abeab5bb2b072bd0fef0b))

## [1.84.1](https://github.com/windmill-labs/windmill/compare/v1.84.0...v1.84.1) (2023-03-31)


### Bug Fixes

* **cli:** overwrite instead of smart diff ([b6d5eef](https://github.com/windmill-labs/windmill/commit/b6d5eef5479e38cc36af2db67d4c45f78c622b9a))

## [1.84.0](https://github.com/windmill-labs/windmill/compare/v1.83.1...v1.84.0) (2023-03-31)


### Features

* add force cancel ([fbe5c18](https://github.com/windmill-labs/windmill/commit/fbe5c18da02763371e6f32c898b31a6a29984b45))
* add the ability to edit previous versions ([2368da2](https://github.com/windmill-labs/windmill/commit/2368da214660ff1835b49b4c2c87256c9bd565cf))
* **backend:** reduce memory allocation for big forloops of flows ([c7506e4](https://github.com/windmill-labs/windmill/commit/c7506e4daec5b12bf908e6954bf6f3521a97b3ba))
* **frontend:** App component style input grouping ([#1334](https://github.com/windmill-labs/windmill/issues/1334)) ([01564f0](https://github.com/windmill-labs/windmill/commit/01564f0a1c26ee9f065bb0adeb7d5e8df0b2e5b5))
* **frontend:** Display frontend execution result in Debug Runs ([#1341](https://github.com/windmill-labs/windmill/issues/1341)) ([57f8dd9](https://github.com/windmill-labs/windmill/commit/57f8dd9570577a58fe91d93c7a9d1a9b4dc69598))
* **frontend:** improve input connection UI ([#1333](https://github.com/windmill-labs/windmill/issues/1333)) ([5ac646e](https://github.com/windmill-labs/windmill/commit/5ac646e859a07efb65542aae9365aa7791ce1097))


### Bug Fixes

* **backend:** add a refresh button to workspace script/hub ([bb61cef](https://github.com/windmill-labs/windmill/commit/bb61cef0e56bf7fa7f8a5f91dabd590afd5db791))
* **backend:** backend compatability on macos ([#1340](https://github.com/windmill-labs/windmill/issues/1340)) ([dfd2abc](https://github.com/windmill-labs/windmill/commit/dfd2abc76466cddca98f93fd82be91ba5d3076e0))
* **frontend:** Export python code as string ([#1339](https://github.com/windmill-labs/windmill/issues/1339)) ([2779891](https://github.com/windmill-labs/windmill/commit/277989141100b033b26b496b8a55d97d48cf7e81))
* **frontend:** improve app tables ([cd1f9b6](https://github.com/windmill-labs/windmill/commit/cd1f9b6baa0dadfb14fee3a586a4b6b164e5e402))
* **frontend:** improve loading of big args in job details ([71619ac](https://github.com/windmill-labs/windmill/commit/71619acdfac010822c1eac496a6f3f869e6ca6fb))
* **frontend:** improve loading of big jobs in run form ([b325493](https://github.com/windmill-labs/windmill/commit/b3254938fe58d8c00a0c4347e7ef519e3a6e4031))

## [1.83.1](https://github.com/windmill-labs/windmill/compare/v1.83.0...v1.83.1) (2023-03-28)


### Bug Fixes

* **cli:** plain secrets might be undefined ([569a55e](https://github.com/windmill-labs/windmill/commit/569a55e45b34641b0fb4569387166f3aa89ce35f))

## [1.83.0](https://github.com/windmill-labs/windmill/compare/v1.82.0...v1.83.0) (2023-03-28)


### Features

* **backend:** allow relative imports for python ([a5500ea](https://github.com/windmill-labs/windmill/commit/a5500ea40a77b2e0408e2a644190a8f65b18cd1d))
* **backend:** execute /bin/bash instead of /bin/sh for bash scripts ([021fa23](https://github.com/windmill-labs/windmill/commit/021fa23f9ffcd11548977a4589eb9bc2815243cf))
* **backend:** improve relative importsfor deno ([eaac598](https://github.com/windmill-labs/windmill/commit/eaac598af308cedea8f0f8fc7c189a4640b4366b))
* **backend:** increase timeout for premium workspace ([00b70d9](https://github.com/windmill-labs/windmill/commit/00b70d9aaac8ae979782492d7754060a3c2c9567))
* **frontend:** add pagination ([33c07d3](https://github.com/windmill-labs/windmill/commit/33c07d3e63f96673719ecb15e45f4cd9e18be80e))
* **frontend:** Add quick style settings to app editor ([#1308](https://github.com/windmill-labs/windmill/issues/1308)) ([ac24862](https://github.com/windmill-labs/windmill/commit/ac2486219cd91df3a7fe11d37894797a881cac6c))
* **frontend:** add recompute as a primitive ([449d3ae](https://github.com/windmill-labs/windmill/commit/449d3ae5ddeceef3fbcb7a815a4dba16c9639fb3))
* **frontend:** add textareacomponent + fix multiselect style + select multi components ([2b31653](https://github.com/windmill-labs/windmill/commit/2b31653a8aa06807678e8609cfa62cf0f2f55dce))
* **frontend:** multiselect components for apps ([577dec5](https://github.com/windmill-labs/windmill/commit/577dec5c5733cdf88e8586ce6c27159920c69c8a))
* **frontend:** use rich json editor for arrays of objects and for object in ArgInput ([b95afaa](https://github.com/windmill-labs/windmill/commit/b95afaa9bb41b102181657453a564f44f4511983))


### Bug Fixes

* **apps:** improve app table actionButtons behavior under many clicks ([8e3d8ac](https://github.com/windmill-labs/windmill/commit/8e3d8acc80de971ee115d6903d24864d8263f08b))
* **cli:** add --plain-secrets ([98d51e2](https://github.com/windmill-labs/windmill/commit/98d51e219df1680507114f9b57ec0b0a4a234b5c))
* **frontend:** add a modal that is always mounted to make sure compon… ([#1328](https://github.com/windmill-labs/windmill/issues/1328)) ([a527cb8](https://github.com/windmill-labs/windmill/commit/a527cb8222a2ff80dae38ebae7dc5ea0979d74c5))
* **frontend:** Disable app keyboard navigation on focused inputs ([#1326](https://github.com/windmill-labs/windmill/issues/1326)) ([da24e9a](https://github.com/windmill-labs/windmill/commit/da24e9ab0625a7503c498c179022ea4011a03170))
* **frontend:** persist description for schemas ([1a48673](https://github.com/windmill-labs/windmill/commit/1a4867302f72aaae8f422ac8f53812c116cc383d))
* **frontend:** Revert app upload input ([#1330](https://github.com/windmill-labs/windmill/issues/1330)) ([fa457bb](https://github.com/windmill-labs/windmill/commit/fa457bb7099bd31c2315eaf7f7f2c40900b2ae39))
* **frontend:** Small app fixes ([#1331](https://github.com/windmill-labs/windmill/issues/1331)) ([75306c8](https://github.com/windmill-labs/windmill/commit/75306c831616d9a01cc3a4681732aab93153f1a9))

## [1.82.0](https://github.com/windmill-labs/windmill/compare/v1.81.0...v1.82.0) (2023-03-24)


### Features

* **backend:** introduce RESTART_ZOMBIE_JOBS and ZOMBIE_JOB_TIMEOUT ([47a7f71](https://github.com/windmill-labs/windmill/commit/47a7f7163aae3fe807e766c824085b4d1b75c8c8))


### Bug Fixes

* **backend:** do not consider FlowPreview as potential zombie job ([f7c30b5](https://github.com/windmill-labs/windmill/commit/f7c30b5d2f16e15f36208e07126557fd7ed84801))
* **backend:** increase dynamic js timeout + improve client passing ([34e25f0](https://github.com/windmill-labs/windmill/commit/34e25f0f96fe637cc42f4017a064c40def5d67ef))
* **cli:** improve diff speed + fix replacing cli ([b999c98](https://github.com/windmill-labs/windmill/commit/b999c9894b4011b735f37df485fe403c22c00512))
* **frontend:** Fix AppTable error display + clear errors when removing a component + properly detect that latest component run had an error ([#1322](https://github.com/windmill-labs/windmill/issues/1322)) ([c15bc8a](https://github.com/windmill-labs/windmill/commit/c15bc8a7bfb3bef2634e6093088967137cd06239))
* **frontend:** fix refresh with manual dependencies ([#1319](https://github.com/windmill-labs/windmill/issues/1319)) ([a47031a](https://github.com/windmill-labs/windmill/commit/a47031a41e6a3392101e280dcd1aea098f898447))
* **frontend:** fix settings panel ([#1323](https://github.com/windmill-labs/windmill/issues/1323)) ([30b8e47](https://github.com/windmill-labs/windmill/commit/30b8e474df5b71b7e7b36d3fe5974a289cf0dfae))
* **frontend:** Fix transformer ([#1321](https://github.com/windmill-labs/windmill/issues/1321)) ([addabcc](https://github.com/windmill-labs/windmill/commit/addabcceb0c90782ba4a934bb3822f8cc9865069))
* **frontend:** remove unnecessary div ([#1318](https://github.com/windmill-labs/windmill/issues/1318)) ([e193a0b](https://github.com/windmill-labs/windmill/commit/e193a0bcdf6690b007594d2f1325a7ec26603129))

## [1.81.0](https://github.com/windmill-labs/windmill/compare/v1.80.1...v1.81.0) (2023-03-21)


### Features

* **apps:** add action on form/button/formbutton ([2593218](https://github.com/windmill-labs/windmill/commit/2593218cbf07c05521a270797055ddb22dc22b8d))


### Bug Fixes

* **frontend:** Remove action outline on preview mode ([#1313](https://github.com/windmill-labs/windmill/issues/1313)) ([a7c4f1a](https://github.com/windmill-labs/windmill/commit/a7c4f1a12e02e8627a5955b75d572e9cf11d8122))

## [1.80.1](https://github.com/windmill-labs/windmill/compare/v1.80.0...v1.80.1) (2023-03-21)


### Bug Fixes

* **cli:** add support for non metadataed scripts ([42f6d2e](https://github.com/windmill-labs/windmill/commit/42f6d2e0ee6294f8a1d97f5f62f2adb6edfd2fed))

## [1.80.0](https://github.com/windmill-labs/windmill/compare/v1.79.0...v1.80.0) (2023-03-20)


### Features

* **apps:** add transformers for data sources ([0abacac](https://github.com/windmill-labs/windmill/commit/0abacac06c7dd586b48c66ff47b7589fe692205b))
* **frontend:** App set tab ([#1307](https://github.com/windmill-labs/windmill/issues/1307)) ([48413a7](https://github.com/windmill-labs/windmill/commit/48413a78c5e7e0ee8208711f15135d81136b7386))


### Bug Fixes

* **frontend:** add missing optional chaining ([#1306](https://github.com/windmill-labs/windmill/issues/1306)) ([29b1cc6](https://github.com/windmill-labs/windmill/commit/29b1cc6ff0ebc5edcad24a1780113889c507075d))
* **frontend:** App button triggered by ([#1304](https://github.com/windmill-labs/windmill/issues/1304)) ([cf2d031](https://github.com/windmill-labs/windmill/commit/cf2d031e8e89faa2cd7fa58436cbe7cf4d9045f9))

## [1.79.0](https://github.com/windmill-labs/windmill/compare/v1.78.0...v1.79.0) (2023-03-17)


### Features

* **frontend:** add listeners for frontend scripts ([597e38e](https://github.com/windmill-labs/windmill/commit/597e38ef367d38fa97fc443ccb2c721e5964fece))
* **frontend:** add table actions navigation ([#1298](https://github.com/windmill-labs/windmill/issues/1298)) ([c3ba1a6](https://github.com/windmill-labs/windmill/commit/c3ba1a6ab97484a08a5a20187bb858a5af7025cb))
* **frontend:** App component triggers ([#1303](https://github.com/windmill-labs/windmill/issues/1303)) ([078cb1b](https://github.com/windmill-labs/windmill/commit/078cb1bf3e4de08cb018578f04d24392a6462f69))
* **frontend:** Component control ([#1293](https://github.com/windmill-labs/windmill/issues/1293)) ([bd927a2](https://github.com/windmill-labs/windmill/commit/bd927a27ed9581dbf67ea3694f9d989f8d71d2ed))


### Bug Fixes

* **frontend:** App panel styling ([#1284](https://github.com/windmill-labs/windmill/issues/1284)) ([c1dd35c](https://github.com/windmill-labs/windmill/commit/c1dd35c3f0fcbc1be43273f82a873c3c07863417))
* **frontend:** Display app context search on top ([#1300](https://github.com/windmill-labs/windmill/issues/1300)) ([bd3ee81](https://github.com/windmill-labs/windmill/commit/bd3ee81b14846f16ccd16461de99b46fe68be6ba))
* **frontend:** fix horizontal splitpanes ([#1301](https://github.com/windmill-labs/windmill/issues/1301)) ([ea3dab4](https://github.com/windmill-labs/windmill/commit/ea3dab411b3d5dd772e04c8831e789e2470aaf28))
* **frontend:** fix map render ([#1297](https://github.com/windmill-labs/windmill/issues/1297)) ([0092721](https://github.com/windmill-labs/windmill/commit/00927210fd68c31cb793ef4f0efea05711ebcf00))
* **frontend:** Hide archive toggle with empty list ([#1296](https://github.com/windmill-labs/windmill/issues/1296)) ([bac831b](https://github.com/windmill-labs/windmill/commit/bac831b23ce85a683ddbd4537900670a0b7d12a8))

## [1.78.0](https://github.com/windmill-labs/windmill/compare/v1.77.0...v1.78.0) (2023-03-16)


### Features

* **frontend:** app textcomponent editable + tooltip ([11567d6](https://github.com/windmill-labs/windmill/commit/11567d6280ea60f1a8c3c6607c724179775cbbe3))


### Bug Fixes

* **backend:** whitelist for include_header was ignored in some cases ([183a459](https://github.com/windmill-labs/windmill/commit/183a4591df700ab4720de6e92a83631256940089))
* **frontend:** improve rendering performance after component moving ([6f890f2](https://github.com/windmill-labs/windmill/commit/6f890f2120885f90d986fbd655096b45bf9de539))
* **frontend:** remove staticOutputs from apps ([dbdfd62](https://github.com/windmill-labs/windmill/commit/dbdfd626386398180ecba7976714f86365eeccd8))

## [1.77.0](https://github.com/windmill-labs/windmill/compare/v1.76.0...v1.77.0) (2023-03-14)


### Features

* **apps:** state can be used as input in apps ([2f0acb9](https://github.com/windmill-labs/windmill/commit/2f0acb9ffa8dace4a886527dcee49809d019b271))
* **apps:** tabs can be made pages or invisible + better frontend scripts reactivity ([cd645d0](https://github.com/windmill-labs/windmill/commit/cd645d0935f2d06e0ff71f14d2cf63accd378ff3))
* **deno:** add support for custom npm repo ([#1291](https://github.com/windmill-labs/windmill/issues/1291)) ([944795f](https://github.com/windmill-labs/windmill/commit/944795f6eeaa7d01ab1a35a80570a55c363723e6))
* **frontend:** add setTab to frontend scripts ([c2a97c5](https://github.com/windmill-labs/windmill/commit/c2a97c53cfff0fdb35dd8bc249490566eebdc1a9))
* **frontend:** app components output panel ([#1283](https://github.com/windmill-labs/windmill/issues/1283)) ([751edcf](https://github.com/windmill-labs/windmill/commit/751edcf9b8e0976a1d073603c9eff5dc6e714490))


### Bug Fixes

* **backend:** do not cache reference to workspace scripts ([eb73f2a](https://github.com/windmill-labs/windmill/commit/eb73f2a687f6faad301b9038ab8585450bec7481))
* **frontend:** fix app tabs ([#1288](https://github.com/windmill-labs/windmill/issues/1288)) ([c71a577](https://github.com/windmill-labs/windmill/commit/c71a577fead90c9cd01a736b54d859ec4f0b7807))
* **frontend:** fix container deletion ([#1287](https://github.com/windmill-labs/windmill/issues/1287)) ([bc870bd](https://github.com/windmill-labs/windmill/commit/bc870bd03eb76cb8bc0e0c861f6cd8a9c661186b))
* **frontend:** Update setting accordion ([#1285](https://github.com/windmill-labs/windmill/issues/1285)) ([dea12e8](https://github.com/windmill-labs/windmill/commit/dea12e8870ece998bb6607723cbaab9b9a958f22))

## [1.76.0](https://github.com/windmill-labs/windmill/compare/v1.75.0...v1.76.0) (2023-03-13)


### Features

* **frontend:** add frontend (JS)  scripts to apps ([f0b1b1f](https://github.com/windmill-labs/windmill/commit/f0b1b1f752731ba434b960a75624118152f53c00))
* **frontend:** Copy, Cut and Paste ([#1279](https://github.com/windmill-labs/windmill/issues/1279)) ([82c139e](https://github.com/windmill-labs/windmill/commit/82c139ed0992be401e250cfb7ecc0fca61b76772))
* **frontend:** disabled for action buttons can now depend on row ([75f87e7](https://github.com/windmill-labs/windmill/commit/75f87e7e1117a9c12afcf626379e94b134a9a493))
* **frontend:** improve drag-n-drop behavior ([cfd489a](https://github.com/windmill-labs/windmill/commit/cfd489a55059e7b6843f99bab261c70b3852e6a2))


### Bug Fixes

* **backend:** improve worker ping api ([c958480](https://github.com/windmill-labs/windmill/commit/c958480ce83844a989f58dd5a70eb288582e2194))
* **frontend:** General fixes and updates ([#1281](https://github.com/windmill-labs/windmill/issues/1281)) ([3e5a179](https://github.com/windmill-labs/windmill/commit/3e5a179eb8cd8001f49c92305141dade1571e20f))

## [1.75.0](https://github.com/windmill-labs/windmill/compare/v1.74.2...v1.75.0) (2023-03-11)


### Features

* add filter jobs by args or result ([3b44f9a](https://github.com/windmill-labs/windmill/commit/3b44f9a72ca0466a44963a4b9657a0ee59b44753))
* **apps:** add resource picker ([8681e83](https://github.com/windmill-labs/windmill/commit/8681e83b574141acbf7e5a389a9e8a4f340336d1))
* **bash:** add default argument handling for bash ([1d5c194](https://github.com/windmill-labs/windmill/commit/1d5c194f09ffba963d52e418c5954843d84ae337))
* **frontend-apps:** add variable picker for static string input on apps ([bc440f8](https://github.com/windmill-labs/windmill/commit/bc440f8d4154ce464c0e027d93b7a0a3b76d782e))
* **frontend:** make runs filters synced with query args ([61a5e1f](https://github.com/windmill-labs/windmill/commit/61a5e1f1accc988628b785b3b9be04c4ea719874))


### Bug Fixes

* **backend:** add killpill for lines reading ([7c825c2](https://github.com/windmill-labs/windmill/commit/7c825c212dd0f1e8be427eabd9a9756303241d1b))
* **cli:** many small fixes ([ce32370](https://github.com/windmill-labs/windmill/commit/ce323709a94d27fb24214719180ea1aafc66d646))

## [1.74.2](https://github.com/windmill-labs/windmill/compare/v1.74.1...v1.74.2) (2023-03-09)


### Bug Fixes

* **frontend:** fix splitpanes navigation ([#1276](https://github.com/windmill-labs/windmill/issues/1276)) ([8d5c5b8](https://github.com/windmill-labs/windmill/commit/8d5c5b88a35d7a3bad1d8ddf2d940026825241eb))

## [1.74.1](https://github.com/windmill-labs/windmill/compare/v1.74.0...v1.74.1) (2023-03-09)


### Bug Fixes

* **apps:** proper reactivity for non rendered static components ([ae53baf](https://github.com/windmill-labs/windmill/commit/ae53bafaf6777f928113f84b2c6ed6a2ed341844))
* **ci:** make windmill compile again by pinning swc deps ([2ea15d5](https://github.com/windmill-labs/windmill/commit/2ea15d5035e5e15473968db3c0501a4dddff5cd0))

## [1.74.0](https://github.com/windmill-labs/windmill/compare/v1.73.1...v1.74.0) (2023-03-09)


### Features

* add delete by path for scripts ([0c2cf92](https://github.com/windmill-labs/windmill/commit/0c2cf92dd3df9610e649f15e23921a4ca0d94e6a))
* **frontend:** Add color picker input to app ([#1270](https://github.com/windmill-labs/windmill/issues/1270)) ([88e537a](https://github.com/windmill-labs/windmill/commit/88e537ad1fb4c207f38fbe951c82106bef6491a3))
* **frontend:** add expand ([#1268](https://github.com/windmill-labs/windmill/issues/1268)) ([b854ee3](https://github.com/windmill-labs/windmill/commit/b854ee34393534bde104e2e6f606108fd66d38dc))
* **frontend:** add hash to ctx in apps ([b1a45b1](https://github.com/windmill-labs/windmill/commit/b1a45b1e708aa6f19f8be9c949507083e044f2d8))
* **frontend:** Add key navigation in app editor ([#1273](https://github.com/windmill-labs/windmill/issues/1273)) ([6b0fb75](https://github.com/windmill-labs/windmill/commit/6b0fb75d23e2151c88b07814139d203c1bd0578d))


### Bug Fixes

* **cli:** improve visibility of the active workspace ([e6344da](https://github.com/windmill-labs/windmill/commit/e6344dac6d1be04b46231fa8ef8579fd12ca8f37))
* **frontend:** add confirmation modal to delete script/flow/app ([a4adcb5](https://github.com/windmill-labs/windmill/commit/a4adcb5192c11f7bf47a0d259825e474779378d7))
* **frontend:** Clean up app editor ([#1267](https://github.com/windmill-labs/windmill/issues/1267)) ([0a5e181](https://github.com/windmill-labs/windmill/commit/0a5e181a3aa966fb8211bee0d9174fc16353b31f))
* **frontend:** Minor changes ([#1272](https://github.com/windmill-labs/windmill/issues/1272)) ([3b6ae0c](https://github.com/windmill-labs/windmill/commit/3b6ae0cc49461b858d9cfff79eae9a7569465235))
* **frontend:** simplify input bindings ([b2de531](https://github.com/windmill-labs/windmill/commit/b2de531a46e4b120d7106d361b727746bec516dd))

## [1.73.1](https://github.com/windmill-labs/windmill/compare/v1.73.0...v1.73.1) (2023-03-07)


### Bug Fixes

* **frontend:** load flow is not initialized ([719d475](https://github.com/windmill-labs/windmill/commit/719d4752621d462b1cfaa0d27930fba7586be779))

## [1.73.0](https://github.com/windmill-labs/windmill/compare/v1.72.0...v1.73.0) (2023-03-07)


### Features

* **frontend:** add a way to automatically resize ([#1259](https://github.com/windmill-labs/windmill/issues/1259)) ([24f58ef](https://github.com/windmill-labs/windmill/commit/24f58efd9994a2201c1b1d9bbfb11734c57068e3))
* **frontend:** add ability to move nodes ([614fb50](https://github.com/windmill-labs/windmill/commit/614fb5022aa7d5428fb96b7ee3a20794edd1e9d3))
* **frontend:** Add app PDF viewer ([#1254](https://github.com/windmill-labs/windmill/issues/1254)) ([3e5d09e](https://github.com/windmill-labs/windmill/commit/3e5d09ef0b5619186bee5ec6d442cbfd12a6e8d5))
* **frontend:** add fork/save buttons + consistent styling for slider/range ([9e9f8ef](https://github.com/windmill-labs/windmill/commit/9e9f8efb8ee389ea75e99b67ef720756959ca737))
* **frontend:** add history to flows and apps ([9e4d90a](https://github.com/windmill-labs/windmill/commit/9e4d90ad37a57ff1f515eea0c82cf603649e915d))
* **frontend:** Fix object viewer style ([#1255](https://github.com/windmill-labs/windmill/issues/1255)) ([94f1aad](https://github.com/windmill-labs/windmill/commit/94f1aadef2b09ac1962478f11b27cc708b8328f1))
* **frontend:** refactor entire flow builder UX ([2ac51b0](https://github.com/windmill-labs/windmill/commit/2ac51b0af08bdef7ce3c7e874e9983b9fc00478a))


### Bug Fixes

* **frontend:** arginput + apppreview fixes ([e2c4545](https://github.com/windmill-labs/windmill/commit/e2c45452401022b00285b21551ffaf35a114be33))
* **frontend:** fix app map reactivity ([#1260](https://github.com/windmill-labs/windmill/issues/1260)) ([2557e13](https://github.com/windmill-labs/windmill/commit/2557e136bd0df1a023819b7d9b2235e30d7140b6))
* **frontend:** fix branch deletion ([#1261](https://github.com/windmill-labs/windmill/issues/1261)) ([a999eb2](https://github.com/windmill-labs/windmill/commit/a999eb21121a7c0010621448324e0c77caf2b3f6))
* **frontend:** Side menu z-index issue ([#1265](https://github.com/windmill-labs/windmill/issues/1265)) ([c638897](https://github.com/windmill-labs/windmill/commit/c638897fdcd58f55b0929f91641b21a6f9d25ead))

## [1.72.0](https://github.com/windmill-labs/windmill/compare/v1.71.0...v1.72.0) (2023-03-02)


### Features

* **backend:** get_result_by_id do a downward pass to find node at any depth ([#1249](https://github.com/windmill-labs/windmill/issues/1249)) ([4c913dc](https://github.com/windmill-labs/windmill/commit/4c913dc4b6be03571a015c97a13829adffb61479))
* **frontend:** Add app map component ([#1251](https://github.com/windmill-labs/windmill/issues/1251)) ([ed25d9f](https://github.com/windmill-labs/windmill/commit/ed25d9f186d9925f75404cb193a025d8a41c4540))
* **frontend:** app splitpanes ([#1248](https://github.com/windmill-labs/windmill/issues/1248)) ([f4d79ee](https://github.com/windmill-labs/windmill/commit/f4d79ee2633e6cdab0fa2410108b31cfa77e10da))


### Bug Fixes

* **backend:** improve result retrieval ([c4463bb](https://github.com/windmill-labs/windmill/commit/c4463bb029907f3c8d77abb194f872aae7876bf6))
* **backend:** incorrect get_result_by_id for  list_result job ([2a75cd2](https://github.com/windmill-labs/windmill/commit/2a75cd250ea5e01849fc8bbb69bf44f147d0acb8))
* **cli:** fix workspace option + run script/flow + whoami ([35ea2b2](https://github.com/windmill-labs/windmill/commit/35ea2b27b12159c68c8507ec1f8686028c975387))
* **frontend:** background script not showing inputs ([55eb48c](https://github.com/windmill-labs/windmill/commit/55eb48c55332431304cedbf3bcbbbcff61ec3645))
* **frontend:** fix table bindings ([2679386](https://github.com/windmill-labs/windmill/commit/2679386bf87a56352269911bd89e52df5ee9f314))
* **frontend:** rework app reactivity ([94b20d2](https://github.com/windmill-labs/windmill/commit/94b20d2f5e3b551974c57ea82b6e3dc16e97b9b8))
* **frontend:** rework app reactivity ([1753cb7](https://github.com/windmill-labs/windmill/commit/1753cb7da658f47be974c15da82c71a8e19309a6))

## [1.71.0](https://github.com/windmill-labs/windmill/compare/v1.70.1...v1.71.0) (2023-02-28)


### Features

* **backend:** use counter for sleep/execution/pull durations ([e568690](https://github.com/windmill-labs/windmill/commit/e56869092a03fec4703ddd9ef65c89edb8122962))
* **cli:** add autocompletions ([287b2db](https://github.com/windmill-labs/windmill/commit/287b2db22f7b56e90bcd0c4727c00096695c2e0d))
* **frontend:** App drawer ([#1246](https://github.com/windmill-labs/windmill/issues/1246)) ([8a0d115](https://github.com/windmill-labs/windmill/commit/8a0d1158c4d7e970cb91e1adf4838e5efdbb39ff))
* **frontend:** drawer for editing workspace scripts in flows ([6adc875](https://github.com/windmill-labs/windmill/commit/6adc87561070d8aceaba1838008cd7e6be2e2660))


### Bug Fixes

* **frontend:** Add more app custom css ([#1229](https://github.com/windmill-labs/windmill/issues/1229)) ([a4e4d18](https://github.com/windmill-labs/windmill/commit/a4e4d188ad10443dd0b7f104389594efc768dc59))
* **frontend:** Add more app custom css ([#1247](https://github.com/windmill-labs/windmill/issues/1247)) ([1bb5ed9](https://github.com/windmill-labs/windmill/commit/1bb5ed9ae01fd7998b06833b6222e5dd5d774d35))
* **frontend:** display currently selected filter even if not in list ([42d1cd6](https://github.com/windmill-labs/windmill/commit/42d1cd6456620ba917c560c87d736dc93634adff))
* **frontend:** Fix deeply nested move ([#1245](https://github.com/windmill-labs/windmill/issues/1245)) ([a67f10e](https://github.com/windmill-labs/windmill/commit/a67f10eeb6fdb44bbb3a510badcc5ad0ae187a2b))
* **frontend:** invisible subgrids have h-0 + app policies fix ([2244e83](https://github.com/windmill-labs/windmill/commit/2244e83b9da803a4cf46ab0825d7cb6cb0e24872))

## [1.70.1](https://github.com/windmill-labs/windmill/compare/v1.70.0...v1.70.1) (2023-02-27)


### Bug Fixes

* **cli:** make cli resilient to systems without openable browsers ([c051ffe](https://github.com/windmill-labs/windmill/commit/c051ffeb42c1cff609f93da7745036ea722e17d4))
* **frontend:** Disable move in nested subgrid ([#1238](https://github.com/windmill-labs/windmill/issues/1238)) ([70eab30](https://github.com/windmill-labs/windmill/commit/70eab303bd45111ae198d9b710bfd6f9f59e53b0))
* **frontend:** Fix inline scripts list ([#1240](https://github.com/windmill-labs/windmill/issues/1240)) ([97602ac](https://github.com/windmill-labs/windmill/commit/97602ac6db1404d36d160a431ffcea6c0f567a48))
* **frontend:** Fix subgrid lock ([#1232](https://github.com/windmill-labs/windmill/issues/1232)) ([8ee9d67](https://github.com/windmill-labs/windmill/commit/8ee9d67f4faa91446338b41c664ef91913eb8b81))

## [1.70.1](https://github.com/windmill-labs/windmill/compare/v1.70.0...v1.70.1) (2023-02-27)


### Bug Fixes

* **cli:** make cli resilient to systems without openable browsers ([c051ffe](https://github.com/windmill-labs/windmill/commit/c051ffeb42c1cff609f93da7745036ea722e17d4))
* **frontend:** Disable move in nested subgrid ([#1238](https://github.com/windmill-labs/windmill/issues/1238)) ([70eab30](https://github.com/windmill-labs/windmill/commit/70eab303bd45111ae198d9b710bfd6f9f59e53b0))
* **frontend:** Fix subgrid lock ([#1232](https://github.com/windmill-labs/windmill/issues/1232)) ([8ee9d67](https://github.com/windmill-labs/windmill/commit/8ee9d67f4faa91446338b41c664ef91913eb8b81))

## [1.70.0](https://github.com/windmill-labs/windmill/compare/v1.69.3...v1.70.0) (2023-02-27)


### Features

* **apps:** add ag grid ([b690d80](https://github.com/windmill-labs/windmill/commit/b690d801d4aa5695ee558e81d1ed114074dfcb83))
* **frontend:** move to other grid ([#1230](https://github.com/windmill-labs/windmill/issues/1230)) ([104e4ac](https://github.com/windmill-labs/windmill/commit/104e4ac5e790c30e6fb6b27726776693038d4f19))


### Bug Fixes

* app setup and sync now uses 1.69.3 ([d38aff2](https://github.com/windmill-labs/windmill/commit/d38aff2fe228f23eb18c3991392928c064e6aca2))
* **frontend:** Fix duplication ([#1237](https://github.com/windmill-labs/windmill/issues/1237)) ([e87f4fc](https://github.com/windmill-labs/windmill/commit/e87f4fc44b847a573f5acafc0348fbcbfcb2258f))
* **frontend:** fix graph viewer id assignment ([e1f686d](https://github.com/windmill-labs/windmill/commit/e1f686d8508cfc1f73c43be08facc44217ca8de0))

## [1.69.3](https://github.com/windmill-labs/windmill/compare/v1.69.2...v1.69.3) (2023-02-24)


### Bug Fixes

* **deno:** fix denoify buffer handling ([c2e5afd](https://github.com/windmill-labs/windmill/commit/c2e5afd4e07fb63375832f308da8c744616ee188))

## [1.69.2](https://github.com/windmill-labs/windmill/compare/v1.69.1...v1.69.2) (2023-02-24)


### Bug Fixes

* **app:** fix all nested behavior ([dd28308](https://github.com/windmill-labs/windmill/commit/dd28308c3cf1877ba3f19dcd2bd20bf1c7896a99))
* **frontend:** delete grid item ([008c30f](https://github.com/windmill-labs/windmill/commit/008c30fcaad64af512407f9889a9881fafac0868))
* **frontend:** duplicate ([483407c](https://github.com/windmill-labs/windmill/commit/483407cdf0e1ed61de180a904934e950fed4adc3))
* **frontend:** Fix findGridItem ([a8295d0](https://github.com/windmill-labs/windmill/commit/a8295d0b5acd08cec42b7939d907df5c25132644))
* **frontend:** Fix findGridItem ([5bb77ed](https://github.com/windmill-labs/windmill/commit/5bb77edf45740a75e969b1bef31580271c9d5505))
* **frontend:** Fix next id ([8ddcf4d](https://github.com/windmill-labs/windmill/commit/8ddcf4d9c1a8d6dd20ee241a3f308811c49e58f1))
* **frontend:** gridtab ([fa105b4](https://github.com/windmill-labs/windmill/commit/fa105b4caeaa2d0e9704a48f6caf8d846839c23e))
* **frontend:** rewrote utils ([ea1b2c2](https://github.com/windmill-labs/windmill/commit/ea1b2c29b95282df347ef9c5973917fa3880e843))
* **frontend:** wip ([33ebe2d](https://github.com/windmill-labs/windmill/commit/33ebe2da8e81476be62a2567d5012573a8a010b6))

## [1.69.1](https://github.com/windmill-labs/windmill/compare/v1.69.0...v1.69.1) (2023-02-24)


### Bug Fixes

* **deno:** remove mysql support waiting for deno fix ([dd7e8c7](https://github.com/windmill-labs/windmill/commit/dd7e8c742c83f6a1d13e4343ca626c0b5efc06fb))
* **deno:** remove mysql support waiting for deno fix ([2f78132](https://github.com/windmill-labs/windmill/commit/2f78132e081bdf3d7468e022f0e981ebfa52cfb3))
* **frontend:** containers and tab fixes v1 ([27cac3f](https://github.com/windmill-labs/windmill/commit/27cac3ffe69c4dac160e9e55ffd1eb8ea348d487))
* **frontend:** containers and tab fixes v1 ([705703a](https://github.com/windmill-labs/windmill/commit/705703a5e2f2dc7ceb4c215221f72bf624799841))
* **frontend:** containers and tab fixes v1 ([fac31c6](https://github.com/windmill-labs/windmill/commit/fac31c6628b289ad6aae92434e312c4be281a4d2))

## [1.69.0](https://github.com/windmill-labs/windmill/compare/v1.68.0...v1.69.0) (2023-02-23)


### Features

* **frontend:** Duplicate component ([#1228](https://github.com/windmill-labs/windmill/issues/1228)) ([089a6b6](https://github.com/windmill-labs/windmill/commit/089a6b6ae52e8d28dd15e2f9a6ad900c5853d0a1))
* **frontend:** Properly delete tab content ([#1227](https://github.com/windmill-labs/windmill/issues/1227)) ([857ee5f](https://github.com/windmill-labs/windmill/commit/857ee5f318466d12bf0d41515451798df087ab74))
* **frontend:** Support deeply nested components ([#1225](https://github.com/windmill-labs/windmill/issues/1225)) ([6ad876e](https://github.com/windmill-labs/windmill/commit/6ad876ebb45a934b7a4dc980cf38a5228d7d11f1))


### Bug Fixes

* **cli:** .wmillignore whitelist behavior ([d543650](https://github.com/windmill-labs/windmill/commit/d543650b313c434e794ad800aefe4aeda83c0fed))

## [1.68.0](https://github.com/windmill-labs/windmill/compare/v1.67.4...v1.68.0) (2023-02-23)


### Features

* **frontend:** Add more app component CSS customisation ([#1218](https://github.com/windmill-labs/windmill/issues/1218)) ([6044e3b](https://github.com/windmill-labs/windmill/commit/6044e3b6ef92e89b8f15f38bc2d0986ec64105d5))


### Bug Fixes

* **cli:** better ergonomics around workspace add ([40c12e6](https://github.com/windmill-labs/windmill/commit/40c12e6139c7b42d7ab169bab2dd37f8b43bea06))
* **cli:** better ergonomics around workspaces ([3b7160e](https://github.com/windmill-labs/windmill/commit/3b7160e84aa454bdb5f343da99cfd97a6b319937))

## [1.67.4](https://github.com/windmill-labs/windmill/compare/v1.67.3...v1.67.4) (2023-02-23)


### Bug Fixes

* **backend:** workflow check for has_failure_module ([e54dc3f](https://github.com/windmill-labs/windmill/commit/e54dc3ff97e4454a15b9efe25cc12f6c9e1e176b))

## [1.67.3](https://github.com/windmill-labs/windmill/compare/v1.67.2...v1.67.3) (2023-02-23)


### Bug Fixes

* **cli:** ignone non wmill looking files ([ec57c59](https://github.com/windmill-labs/windmill/commit/ec57c5977f122b629a07e05bc3551662d518ce30))

## [1.67.2](https://github.com/windmill-labs/windmill/compare/v1.67.1...v1.67.2) (2023-02-23)


### Bug Fixes

* **cli:** ignone non wmill looking files ([969e89f](https://github.com/windmill-labs/windmill/commit/969e89f8bbc10f6712920321b70ede35f19ab9ed))

## [1.67.1](https://github.com/windmill-labs/windmill/compare/v1.67.0...v1.67.1) (2023-02-22)


### Bug Fixes

* **cli:** coloring nits ([3fa24ad](https://github.com/windmill-labs/windmill/commit/3fa24adad0a07ba2f469c545b28251b035efdf90))

## [1.67.0](https://github.com/windmill-labs/windmill/compare/v1.66.1...v1.67.0) (2023-02-22)


### Features

* **frontend:** Add app sub grids ([#1208](https://github.com/windmill-labs/windmill/issues/1208)) ([dbc59e9](https://github.com/windmill-labs/windmill/commit/dbc59e952143ee5813780ad13794cef4e036911c))


### Bug Fixes

* **cli:** add --fail-conflicts to ci push ([0085b46](https://github.com/windmill-labs/windmill/commit/0085b46c1e3b8267fcafcb06ce72b4d820e49df5))

## [1.66.1](https://github.com/windmill-labs/windmill/compare/v1.66.0...v1.66.1) (2023-02-22)


### Bug Fixes

* **cli:** delete workspace instead of archiving them ([70dfc8b](https://github.com/windmill-labs/windmill/commit/70dfc8b8d0293d80da7db14caa1b9eb0ed67653d))

## [1.66.0](https://github.com/windmill-labs/windmill/compare/v1.65.0...v1.66.0) (2023-02-22)


### Features

* add delete flows ([e81f7bd](https://github.com/windmill-labs/windmill/commit/e81f7bd7239b73710da2a4ddec0da7805c13da06))
* CLI refactor v1 ([e31d2ae](https://github.com/windmill-labs/windmill/commit/e31d2ae27f886e774ffc429eea80057f4f9f4213))
* **frontend:** Add image app component ([#1213](https://github.com/windmill-labs/windmill/issues/1213)) ([a4b773a](https://github.com/windmill-labs/windmill/commit/a4b773af294554c5787f02ebda363c8d9a3eff1b))

## [1.65.0](https://github.com/windmill-labs/windmill/compare/v1.64.0...v1.65.0) (2023-02-21)


### Features

* **apps:** add asJson for customcss ([71d6dad](https://github.com/windmill-labs/windmill/commit/71d6dad37cc239952ce7799609c02474b0b1fc81))
* **apps:** add custom css for apps ([7f00e1c](https://github.com/windmill-labs/windmill/commit/7f00e1c1a8f2e905b0677d82ba547f55dc23b3e0))
* **backend:** Zip Workspace Export ([#1201](https://github.com/windmill-labs/windmill/issues/1201)) ([5d109b3](https://github.com/windmill-labs/windmill/commit/5d109b3cd4b7749788f9cb9fcbe1949c45eedf1f))
* **frontend:** Add divider app component ([#1209](https://github.com/windmill-labs/windmill/issues/1209)) ([c33e79e](https://github.com/windmill-labs/windmill/commit/c33e79e0b8d5ba1103d87fdd47fcd0e1071e19de))
* **frontend:** Add file input app component ([#1211](https://github.com/windmill-labs/windmill/issues/1211)) ([d4b6d69](https://github.com/windmill-labs/windmill/commit/d4b6d691264bf21e4e2c97548aaad9aa80678a6b))
* **frontend:** Add icon app component ([#1207](https://github.com/windmill-labs/windmill/issues/1207)) ([e4791c2](https://github.com/windmill-labs/windmill/commit/e4791c2b7e3a0e6b90c37bc1200f9cd0ab3b6845))

## [1.64.0](https://github.com/windmill-labs/windmill/compare/v1.63.2...v1.64.0) (2023-02-16)


### Features

* **frontend:** Trigger settings drawer with URL hash ([#1185](https://github.com/windmill-labs/windmill/issues/1185)) ([8445697](https://github.com/windmill-labs/windmill/commit/8445697e31394ac11f3b8aa10af1546cc9c0041c))

## [1.63.2](https://github.com/windmill-labs/windmill/compare/v1.63.1...v1.63.2) (2023-02-15)


### Bug Fixes

* **psql:** update pg client ([a2fbc57](https://github.com/windmill-labs/windmill/commit/a2fbc5702509bb259bae106baa9a6146360ec5dd))

## [1.63.1](https://github.com/windmill-labs/windmill/compare/v1.63.0...v1.63.1) (2023-02-14)


### Bug Fixes

* update hub sync script ([03eb144](https://github.com/windmill-labs/windmill/commit/03eb1444c4a5dfbd170ba8d200784e530ca2f771))

## [1.63.0](https://github.com/windmill-labs/windmill/compare/v1.62.0...v1.63.0) (2023-02-14)


### Features

* add mem peak info ([f584062](https://github.com/windmill-labs/windmill/commit/f584062f13aa7da8e767fd35de1aef7bbb67c3c8))
* **frontend:** Minimal support for custom filenames ([#1190](https://github.com/windmill-labs/windmill/issues/1190)) ([b03b3be](https://github.com/windmill-labs/windmill/commit/b03b3be154efb0984f9623c27acc05617f125bc5))
* **worker:** set oom_adj to 1000 to prioritize killing subprocess ([265fbc5](https://github.com/windmill-labs/windmill/commit/265fbc5835d029d510a794e171392884cb20bdae))


### Bug Fixes

* **python:** return none if argument is missing ([3f2754b](https://github.com/windmill-labs/windmill/commit/3f2754b3305f6cb65373d532ff0db6020bf07e45))
* Update references to the docs ([#1191](https://github.com/windmill-labs/windmill/issues/1191)) ([a574270](https://github.com/windmill-labs/windmill/commit/a574270bc259f423c984259cd7d9a6d91b77815c))

## [1.62.0](https://github.com/windmill-labs/windmill/compare/v1.61.1...v1.62.0) (2023-02-03)


### Features

* add INCLUDE_HEADERS env variable to pass value from request headers ([0921ba0](https://github.com/windmill-labs/windmill/commit/0921ba008535e945f2ec3255728c2e8c1f4c36dc))
* add WHITELIST_WORKSPACES and BLACKLIST_WORKSPACES ([99568ea](https://github.com/windmill-labs/windmill/commit/99568eaa473d57123a7dde4007f8812e0053fb3f))
* Add workspace webhook ([#1158](https://github.com/windmill-labs/windmill/issues/1158)) ([b9ac60f](https://github.com/windmill-labs/windmill/commit/b9ac60f8bb0662e364606c4b7b8a6e3c1e7e4041))
* adding worker_busy ([23007f7](https://github.com/windmill-labs/windmill/commit/23007f7a71630fc2040e1be39db83ba56689e3c4))
* **cli:** 2-Way sync ([#1071](https://github.com/windmill-labs/windmill/issues/1071)) ([cdd1619](https://github.com/windmill-labs/windmill/commit/cdd16195aeaf32e1f1d0648f48e4843954d16d9c))
* **frontend:** App initial loading animations ([#1176](https://github.com/windmill-labs/windmill/issues/1176)) ([3305481](https://github.com/windmill-labs/windmill/commit/3305481d5d4ce598ceb57256cea851869cdaf25e))
* **python:** add ADDITIONAL_PYTHON_PATHS ([14b32be](https://github.com/windmill-labs/windmill/commit/14b32be8b229372c57a167fd74cb958a96f0e8e6))


### Bug Fixes

* **frontend:** Render popups above components in app editor ([#1171](https://github.com/windmill-labs/windmill/issues/1171)) ([bc8d1a3](https://github.com/windmill-labs/windmill/commit/bc8d1a375ec7886357ce0ef5971bb35013c94d61))
* **frontend:** Various fixes and improvements ([#1177](https://github.com/windmill-labs/windmill/issues/1177)) ([9f5500c](https://github.com/windmill-labs/windmill/commit/9f5500c1965ea50796d3bf289c0f9e0c929427f4))
* navigate to new script page before saving script ([f171cd8](https://github.com/windmill-labs/windmill/commit/f171cd8b7c46677173572bac256cbb489a1b8526))

## [1.61.1](https://github.com/windmill-labs/windmill/compare/v1.61.0...v1.61.1) (2023-01-31)


### Bug Fixes

* **backend:** compile issue ([df8cc1f](https://github.com/windmill-labs/windmill/commit/df8cc1f2482b3d8b1530cdaef1361303ff5cadff))

## [1.61.0](https://github.com/windmill-labs/windmill/compare/v1.60.0...v1.61.0) (2023-01-31)


### Features

* add openapi viewer ([#1094](https://github.com/windmill-labs/windmill/issues/1094)) ([1337811](https://github.com/windmill-labs/windmill/commit/1337811438d48e23133f68e9157bd185d5fe4a82))
* add PIP_LOCAL_DEPENDENCIES ([b7db4c7](https://github.com/windmill-labs/windmill/commit/b7db4c78c4629f1fd2dfd7a338f783b16f07b24d))
* add QUEUE_LIMIT_WAIT_RESULT ([51a8810](https://github.com/windmill-labs/windmill/commit/51a8810aa0a9ab7702df459dd270278d42bd3899))
* add resource and resource type from json ([080ecb0](https://github.com/windmill-labs/windmill/commit/080ecb04d7a08d035fe07f179975b52bc0f77297))
* add sql as a valid type in Python ([0172587](https://github.com/windmill-labs/windmill/commit/0172587b129ce54d96dc99336a1f56c66ebdbef5))
* add sync webhook for flows ([f377c84](https://github.com/windmill-labs/windmill/commit/f377c84f5a2148a2bbb7c16e93f13e1d85ceb17e))
* **backend:** add queue_limit + configurable timeout + fix timeout cancel ([eef3bab](https://github.com/windmill-labs/windmill/commit/eef3bab6e4d9f1af1435db868c707a692558ab74))
* **deno:** add support for DENO_AUTH_TOKENS ([832ddab](https://github.com/windmill-labs/windmill/commit/832ddabdf2239521368e5f96df144abce0db31c2))
* **deno:** allow overriding deno sandboxing with DENO_FLAGS' ([7f40373](https://github.com/windmill-labs/windmill/commit/7f40373fd64005d87972854a565c6cf521232982))
* **frontend:** Add app inputs configurations ([#1142](https://github.com/windmill-labs/windmill/issues/1142)) ([3ed16b8](https://github.com/windmill-labs/windmill/commit/3ed16b88a42e4db6e12f8557c5bbaa2d832b1c17))
* **frontend:** Add app preview lock ([#1127](https://github.com/windmill-labs/windmill/issues/1127)) ([6a88e8c](https://github.com/windmill-labs/windmill/commit/6a88e8c4f4d6fa5c393ce27b2040784a74a73b06))
* **frontend:** Add copy button option to app text display component ([#1090](https://github.com/windmill-labs/windmill/issues/1090)) ([bdfc38d](https://github.com/windmill-labs/windmill/commit/bdfc38d954a3c5548fb7f9ee6f80f741eff8cb67))
* **frontend:** Add default codes to app editor ([#1099](https://github.com/windmill-labs/windmill/issues/1099)) ([c50c740](https://github.com/windmill-labs/windmill/commit/c50c7406f267b480af2a01b47e3fcfa1d763db7a))
* **frontend:** Add HTML result rendering ([#1160](https://github.com/windmill-labs/windmill/issues/1160)) ([c01bf70](https://github.com/windmill-labs/windmill/commit/c01bf70f62680a4b77812ac6eb64aca2b15d9a8d))
* **frontend:** Add more integration icons ([#1097](https://github.com/windmill-labs/windmill/issues/1097)) ([2191e85](https://github.com/windmill-labs/windmill/commit/2191e852318f069489f77a4f1c44aadf248c7f53))
* **frontend:** add plotly support ([a4f8f9e](https://github.com/windmill-labs/windmill/commit/a4f8f9e1cf80395d5cd1229c8dd5dda244e2ba7f))
* **frontend:** add selectedRowIndex to the table outputs ([#1145](https://github.com/windmill-labs/windmill/issues/1145)) ([f05f9e4](https://github.com/windmill-labs/windmill/commit/f05f9e4edb928e7a8e3e66a62de9c6487684a14b))
* **frontend:** Add Supabase resource ([#1107](https://github.com/windmill-labs/windmill/issues/1107)) ([12b00a8](https://github.com/windmill-labs/windmill/commit/12b00a808d1f12827a7bc26518cc6f972bdde917))
* **frontend:** add support for background scripts + add FormButtonCo… ([#1124](https://github.com/windmill-labs/windmill/issues/1124)) ([e969af9](https://github.com/windmill-labs/windmill/commit/e969af9e44d1b4409064080e8662552ee3e262e8))
* **frontend:** Add surreal db logo ([#1102](https://github.com/windmill-labs/windmill/issues/1102)) ([d811675](https://github.com/windmill-labs/windmill/commit/d81167588227f2cc433aab64551d96d21a589c5b))
* **frontend:** Add tooltip to app recompute ([#1122](https://github.com/windmill-labs/windmill/issues/1122)) ([4dfdf37](https://github.com/windmill-labs/windmill/commit/4dfdf374af358ef46ee8057373546719c6570067))
* **frontend:** add vega-lite component ([bd79938](https://github.com/windmill-labs/windmill/commit/bd79938bed6da3875a4a2dd72dad14dedbf25ddf))
* **frontend:** Display error as an icon in order to avoid clutter wh… ([#1143](https://github.com/windmill-labs/windmill/issues/1143)) ([22b8fed](https://github.com/windmill-labs/windmill/commit/22b8fed9d904a37aae66f6d957f4987f6ca9955c))
* **frontend:** Open debug runs from component ([#1155](https://github.com/windmill-labs/windmill/issues/1155)) ([73bc13b](https://github.com/windmill-labs/windmill/commit/73bc13bb7d4b1eb25a3a726ac9e6bb80120a495f))
* **frontend:** Update app table component styles ([#1100](https://github.com/windmill-labs/windmill/issues/1100)) ([172b5db](https://github.com/windmill-labs/windmill/commit/172b5dba8f4c3aaf11569c72313ad74845c668a6))
* **python:** add support for extra args in python ([772c768](https://github.com/windmill-labs/windmill/commit/772c768cda094f208a5efb7aab03eee3a8f38f68))


### Bug Fixes

* **frontend:** Add default value for text, number and date input + fix issues with number input + add date input in the settings panel ([#1135](https://github.com/windmill-labs/windmill/issues/1135)) ([8f90602](https://github.com/windmill-labs/windmill/commit/8f906026b3203702c3b6a30bcac9fb2aca985c29))
* **frontend:** Add highlight to selected workspace ([#1159](https://github.com/windmill-labs/windmill/issues/1159)) ([f221a6c](https://github.com/windmill-labs/windmill/commit/f221a6c17f145d0c42f7faf785c37f4037308973))
* **frontend:** add missing condition to properly select first row ([#1128](https://github.com/windmill-labs/windmill/issues/1128)) ([3d873ed](https://github.com/windmill-labs/windmill/commit/3d873ed51c769005981a8d8dfb95faa3ca33bb83))
* **frontend:** App form component display ([#1096](https://github.com/windmill-labs/windmill/issues/1096)) ([339742c](https://github.com/windmill-labs/windmill/commit/339742ca77dd0fda19d5a262617e42c341ef5871))
* **frontend:** App script list panel overflow ([#1101](https://github.com/windmill-labs/windmill/issues/1101)) ([7bc59d9](https://github.com/windmill-labs/windmill/commit/7bc59d9d2650b623a2b481a727ffc495b4216f22))
* **frontend:** App table action button cell ([#1149](https://github.com/windmill-labs/windmill/issues/1149)) ([e989662](https://github.com/windmill-labs/windmill/commit/e98966283dd9b57cc07da34876a90d19210c2927))
* **frontend:** App table header z-index ([#1120](https://github.com/windmill-labs/windmill/issues/1120)) ([59c4cc2](https://github.com/windmill-labs/windmill/commit/59c4cc2058f86deea793b61de59e2936e50e5577))
* **frontend:** Check if hiddenInlineScripts are undefined before iterating over them ([#1134](https://github.com/windmill-labs/windmill/issues/1134)) ([71a443e](https://github.com/windmill-labs/windmill/commit/71a443e3c56d2b8c951de6e3701a411ad1a0ce34))
* **frontend:** fix first row selection ([#1125](https://github.com/windmill-labs/windmill/issues/1125)) ([6c9daf7](https://github.com/windmill-labs/windmill/commit/6c9daf70021859dcd7cef717bc3acdfa88cffd02))
* **frontend:** Fix id generation when a second action ([#1110](https://github.com/windmill-labs/windmill/issues/1110)) ([4f86981](https://github.com/windmill-labs/windmill/commit/4f869811fee73826b2b10965241d2d8dba59dc2a))
* **frontend:** Make sure AppSelect items are an array ([#1144](https://github.com/windmill-labs/windmill/issues/1144)) ([24b1fa0](https://github.com/windmill-labs/windmill/commit/24b1fa0ae327c984841f9ed8b163b3fccc6da258))
* **frontend:** Make sure that old apps are rendering properly ([#1132](https://github.com/windmill-labs/windmill/issues/1132)) ([a78486d](https://github.com/windmill-labs/windmill/commit/a78486d7e08f76e22406063288b35e9030974d7a))
* **frontend:** Playwright ([#1108](https://github.com/windmill-labs/windmill/issues/1108)) ([f0435f5](https://github.com/windmill-labs/windmill/commit/f0435f5f81941c5b49500003aa27956d627daadb))
* **frontend:** Prepare app scripts code for export ([#1123](https://github.com/windmill-labs/windmill/issues/1123)) ([173093a](https://github.com/windmill-labs/windmill/commit/173093a40321f6ad35bf766a5554b21cea388771))
* **frontend:** Prevent modal from hijacking all keypress event ([#1136](https://github.com/windmill-labs/windmill/issues/1136)) ([aa6de3b](https://github.com/windmill-labs/windmill/commit/aa6de3bb5746b9d99c8e3a52e6a9fff10d97bc6a))
* **frontend:** Revert component input panel change ([#1092](https://github.com/windmill-labs/windmill/issues/1092)) ([0419e7e](https://github.com/windmill-labs/windmill/commit/0419e7e1c9239fd3cbc49acf82a73e9c01938153))
* **frontend:** Runnable table overflow ([#1119](https://github.com/windmill-labs/windmill/issues/1119)) ([462adbe](https://github.com/windmill-labs/windmill/commit/462adbe42f823646413a5003fd71f3dd473c0728))
* **frontend:** Select the first row by default, and remove the abilit… ([#1121](https://github.com/windmill-labs/windmill/issues/1121)) ([3c483f5](https://github.com/windmill-labs/windmill/commit/3c483f533759b9b4e589055dbddb31f294bea8fa))
* **frontend:** Show app builder header always on top ([#1118](https://github.com/windmill-labs/windmill/issues/1118)) ([631a3da](https://github.com/windmill-labs/windmill/commit/631a3da17f05a3d29defdf96a50d7e96a9f8baad))
* **frontend:** Update app scripts pane ([#1146](https://github.com/windmill-labs/windmill/issues/1146)) ([18f30c8](https://github.com/windmill-labs/windmill/commit/18f30c8286f8240158643ade8b0ef4607a80fbb0))
* **frontend:** Use absolute path on connect images ([#1095](https://github.com/windmill-labs/windmill/issues/1095)) ([43e069e](https://github.com/windmill-labs/windmill/commit/43e069eb96c0af7d3a1fe1db4f4b69f8e31e7438))
* improvements for error handling as first step of flow ([b77c239](https://github.com/windmill-labs/windmill/commit/b77c239f307a37777acb083b0cdb5c0d214a9dd8))

## [1.60.0](https://github.com/windmill-labs/windmill/compare/v1.59.0...v1.60.0) (2023-01-11)


### Features

* add 'add user to workspace' ([a14623f](https://github.com/windmill-labs/windmill/commit/a14623feaab4a36c01d558b775a42e587a74cdc9))
* **frontend:** Add frost to color palette ([#1084](https://github.com/windmill-labs/windmill/issues/1084)) ([8e72007](https://github.com/windmill-labs/windmill/commit/8e7200736827e8f6e593f900124b1bd1bc0bd5f2))


### Bug Fixes

* **frontend:** Keep pane resizer under open drawer ([#1089](https://github.com/windmill-labs/windmill/issues/1089)) ([cb25f88](https://github.com/windmill-labs/windmill/commit/cb25f883005b99b4ce98e8ae7b8253a8a2fedb5b))

## [1.59.0](https://github.com/windmill-labs/windmill/compare/v1.58.0...v1.59.0) (2023-01-09)


### Features

* add relative imports for python scripts ([#1075](https://github.com/windmill-labs/windmill/issues/1075)) ([5347cd4](https://github.com/windmill-labs/windmill/commit/5347cd46a996b4cf48a96fbb873e4d029ca4f75f))


### Bug Fixes

* **frontend:** Iconed resource height issue ([#1073](https://github.com/windmill-labs/windmill/issues/1073)) ([a84eb9b](https://github.com/windmill-labs/windmill/commit/a84eb9b1f7e1b10c960ee1594ef476e7ba146f5e))

## [1.58.0](https://github.com/windmill-labs/windmill/compare/v1.57.1...v1.58.0) (2023-01-07)


### Features

* add archive/unarchive/delete workspace ([6edf9b9](https://github.com/windmill-labs/windmill/commit/6edf9b9946d613b599cb91688c4986044caaba8d))
* add hub support for apps ([50453ca](https://github.com/windmill-labs/windmill/commit/50453ca690dfd936474ebbf000e36ae1006b188b))
* add min/max constraint to number + slider component ([0bcdcae](https://github.com/windmill-labs/windmill/commit/0bcdcaedcfdf7b7f76f703df3bf50d97dd389995))
* add support for yaml format as a string format ([5204e4a](https://github.com/windmill-labs/windmill/commit/5204e4a75d74e6bb4087dee7087390f7c0388e51))
* **frontend:** Add integration icons ([#1063](https://github.com/windmill-labs/windmill/issues/1063)) ([45acb89](https://github.com/windmill-labs/windmill/commit/45acb89f87ad78c48a1ba6abf1bd1424088b41c4))
* **frontend:** Toggle to hide optional inputs ([#1060](https://github.com/windmill-labs/windmill/issues/1060)) ([4d6a568](https://github.com/windmill-labs/windmill/commit/4d6a568820ceb6c064dc2871085b80412e18c379))
* **frontend:** Update app auto-refresh button ([#1062](https://github.com/windmill-labs/windmill/issues/1062)) ([34e3331](https://github.com/windmill-labs/windmill/commit/34e33319192f6d747d84fc6559853410f5d72ec8))


### Bug Fixes

* **frontend:** Remove popover hover styles ([#1064](https://github.com/windmill-labs/windmill/issues/1064)) ([76a860f](https://github.com/windmill-labs/windmill/commit/76a860fe538dadfc6691074384f92db1a331063d))

## [1.57.1](https://github.com/windmill-labs/windmill/compare/v1.57.0...v1.57.1) (2023-01-02)


### Bug Fixes

* preserver order changes for flows' schema ([2c8e98a](https://github.com/windmill-labs/windmill/commit/2c8e98a9c7fe3fdd48c851c0575fdb1d87c953a9))
* support setting undefined states ([ab0aeb0](https://github.com/windmill-labs/windmill/commit/ab0aeb0df825fb5afefbefae6739179dbbbc5f30))

## [1.57.0](https://github.com/windmill-labs/windmill/compare/v1.56.1...v1.57.0) (2023-01-01)


### Features

* add a All Static Inputs module to the flow editor ([3296deb](https://github.com/windmill-labs/windmill/commit/3296debfe7940fe833d489af0a4b6609c2d53411))
* apps can be published publicly ([be14aab](https://github.com/windmill-labs/windmill/commit/be14aab9b102ef81eccf689e00cd3cd8eae8f503))
* **app:** Update sidebar menu ([#1050](https://github.com/windmill-labs/windmill/issues/1050)) ([faa046a](https://github.com/windmill-labs/windmill/commit/faa046a3fdc326084df93f8e57dd5c573164b91d))
* **app:** Use consistent styles on settings pages ([#1048](https://github.com/windmill-labs/windmill/issues/1048)) ([681e2e8](https://github.com/windmill-labs/windmill/commit/681e2e824a39d9748f1aaa37f20001b5200f82bc))
* **backend:** resume from owner directly in flow status viewer ([#1042](https://github.com/windmill-labs/windmill/issues/1042)) ([40195d4](https://github.com/windmill-labs/windmill/commit/40195d42f661d401cd9ce11ca9739f87c1a27afd))
* **frontend:** Add customization props to radio ([#1056](https://github.com/windmill-labs/windmill/issues/1056)) ([0812f6e](https://github.com/windmill-labs/windmill/commit/0812f6efd8484e86a4f09631b28c71d17cd69627))
* **frontend:** Fix initial component dimensions + Select select + add spinner when buttons are clicked ([#1044](https://github.com/windmill-labs/windmill/issues/1044)) ([70e7a5d](https://github.com/windmill-labs/windmill/commit/70e7a5d07542e1ac936152e434146e056a80bad4))
* **frontend:** Properly support resource ([#1039](https://github.com/windmill-labs/windmill/issues/1039)) ([65f4e86](https://github.com/windmill-labs/windmill/commit/65f4e86a22838bd34373ce808c77a1770eeaf295))
* **frontend:** Update tooltip and home list dropdown ([#1053](https://github.com/windmill-labs/windmill/issues/1053)) ([9d30e5f](https://github.com/windmill-labs/windmill/commit/9d30e5fa57363c4cf715f845f5268856c4aa0fb3))


### Bug Fixes

* **app:** Fix inconsistencies in list items and sidebar menus ([#1051](https://github.com/windmill-labs/windmill/issues/1051)) ([0f1b19c](https://github.com/windmill-labs/windmill/commit/0f1b19c7d3eea4f8106fed3188460678e5035812))
* **frontend:** List item overflowing corners ([#1055](https://github.com/windmill-labs/windmill/issues/1055)) ([2fd730f](https://github.com/windmill-labs/windmill/commit/2fd730f8d2303b57f2da42354cd207dad2a410ce))
* **frontend:** Minor fixes in editor ([#1054](https://github.com/windmill-labs/windmill/issues/1054)) ([adc84f0](https://github.com/windmill-labs/windmill/commit/adc84f06d97275b17bf77cb6c8d264ad28b0f6ce))
* **frontend:** Static inputs overflow ([#1057](https://github.com/windmill-labs/windmill/issues/1057)) ([72aeba1](https://github.com/windmill-labs/windmill/commit/72aeba121cb694e8e96ad189b4acbfc2340bf520))

## [1.56.1](https://github.com/windmill-labs/windmill/compare/v1.56.0...v1.56.1) (2022-12-23)


### Bug Fixes

* **cli:** typo in cli deps ([0614ec4](https://github.com/windmill-labs/windmill/commit/0614ec42baf3e8f1675d62ca0f143b831c2700a1))

## [1.56.0](https://github.com/windmill-labs/windmill/compare/v1.55.0...v1.56.0) (2022-12-23)


### Features

* add move to drawer for script and flows ([f73dbd8](https://github.com/windmill-labs/windmill/commit/f73dbd8039b3c987ca94e5b56f0ecdea93cbd1b8))
* add operator mode ([3485b07](https://github.com/windmill-labs/windmill/commit/3485b07b2548b7ea8fbd2b6b31b91e2d36d072ef))
* auto-invite from same domain ([2bae50f](https://github.com/windmill-labs/windmill/commit/2bae50f3910a99a87efa402a9eef566320fe1f68))
* **backend:** add SUPERADMIN_SECRET as an env set superadmin ([c283112](https://github.com/windmill-labs/windmill/commit/c28311242d58af12a039b81a5e5c90688022ce8c))
* **frontend:** Add an input field to edit inline script name ([#1033](https://github.com/windmill-labs/windmill/issues/1033)) ([95a0b9c](https://github.com/windmill-labs/windmill/commit/95a0b9ceae73e291a0def340e935658b6c2ac3a5))
* **frontend:** Add app number input ([#1010](https://github.com/windmill-labs/windmill/issues/1010)) ([2fe927f](https://github.com/windmill-labs/windmill/commit/2fe927f7fdc1309c7bad8b90fb7e0cc41d364b3f))
* **frontend:** Add form component + fix connection bug ([#1012](https://github.com/windmill-labs/windmill/issues/1012)) ([424c31c](https://github.com/windmill-labs/windmill/commit/424c31c54a2652b89f9b06499a5aaf1cc0f00ad9))
* **frontend:** Add select component to app builder ([#1021](https://github.com/windmill-labs/windmill/issues/1021)) ([08071bb](https://github.com/windmill-labs/windmill/commit/08071bb66b4fc40e3b984ffb459e5d52d5816298))
* **frontend:** Add the ability to lock components so they don't move around ([#1035](https://github.com/windmill-labs/windmill/issues/1035)) ([26a6de2](https://github.com/windmill-labs/windmill/commit/26a6de247c3566bfa524b8fa4f8fc212ca557874))
* **frontend:** Align output panel UI ([#1025](https://github.com/windmill-labs/windmill/issues/1025)) ([0e871ca](https://github.com/windmill-labs/windmill/commit/0e871ca8432d4f0bc68543b4a3f3bf8f8af99669))
* **frontend:** App builder password and date input ([#1022](https://github.com/windmill-labs/windmill/issues/1022)) ([4651c9d](https://github.com/windmill-labs/windmill/commit/4651c9d8cd644e59bfd4f57be0bcecc01962a536))
* **frontend:** AppTable v2 + Inline script panel ([#1023](https://github.com/windmill-labs/windmill/issues/1023)) ([f6df3ae](https://github.com/windmill-labs/windmill/commit/f6df3ae36748a1271625c3f4b50ca66f604d79f7))
* **frontend:** Fix component synchro ([#1038](https://github.com/windmill-labs/windmill/issues/1038)) ([cebbc5f](https://github.com/windmill-labs/windmill/commit/cebbc5fbd1b8b855c9b1bcab535cff5b9de8d778))
* **frontend:** Fix inline script status ([#1034](https://github.com/windmill-labs/windmill/issues/1034)) ([be74311](https://github.com/windmill-labs/windmill/commit/be743117d155afb2a2f0fe33ff610e0f621409f7))
* **frontend:** Fix UI ([#1009](https://github.com/windmill-labs/windmill/issues/1009)) ([0ceb4ab](https://github.com/windmill-labs/windmill/commit/0ceb4ab1a893fecf9e64497612e6040d0e7bc8cd))
* **frontend:** Fork + Fix table ([#1037](https://github.com/windmill-labs/windmill/issues/1037)) ([ab13e8c](https://github.com/windmill-labs/windmill/commit/ab13e8cce44ded7e05a8dda3d4d4d1ac696bf739))
* **frontend:** Small UI fixes ([#1026](https://github.com/windmill-labs/windmill/issues/1026)) ([ebca9f3](https://github.com/windmill-labs/windmill/commit/ebca9f39eab27dda65d0ee5de175a90363bfebae))
* **frontend:** templatable editor with autocompletion ([e228c64](https://github.com/windmill-labs/windmill/commit/e228c6448ead4a7aef433f4abdfe3c466a0f50f4))
* implement usage tracker + quotas ([fd87109](https://github.com/windmill-labs/windmill/commit/fd871093f0ea4b2def351857d7d8d7e4e79f9539))
* introduce folders, deprecate items owned by groups ([4329d25](https://github.com/windmill-labs/windmill/commit/4329d259887da71eb2b2a67f73947b0fbe9f3941))
* introduce folders, deprecate items owned by groups ([c1b0b64](https://github.com/windmill-labs/windmill/commit/c1b0b64e1728007b364d2a0acc58fc459e49e461))
* Superadmins workspace ([#1003](https://github.com/windmill-labs/windmill/issues/1003)) ([4004de0](https://github.com/windmill-labs/windmill/commit/4004de06180868af4570668a2040bd711a461e0d))


### Bug Fixes

* **frontend:** copy-to-clipnoard url with protocol ([#1027](https://github.com/windmill-labs/windmill/issues/1027)) ([f77fe7b](https://github.com/windmill-labs/windmill/commit/f77fe7b6b321c3d00a51a42a4118fd37f7c9d782))
* **frontend:** Fix AppTable frontend search ([#1013](https://github.com/windmill-labs/windmill/issues/1013)) ([f7627b5](https://github.com/windmill-labs/windmill/commit/f7627b5f17a9f5a4528715eebb4d207f33609da2))

## [1.55.0](https://github.com/windmill-labs/windmill/compare/v1.54.0...v1.55.0) (2022-12-09)


### Features

* **frontend:** Add text input to app builder ([#1008](https://github.com/windmill-labs/windmill/issues/1008)) ([6198383](https://github.com/windmill-labs/windmill/commit/6198383138929237c1eb898954a1fd91bdded08a))

## [1.54.0](https://github.com/windmill-labs/windmill/compare/v1.53.0...v1.54.0) (2022-12-08)


### Features

* add lockable version to scripts inside flows ([#972](https://github.com/windmill-labs/windmill/issues/972)) ([799fa92](https://github.com/windmill-labs/windmill/commit/799fa925b39316f6f8232d01959c35c4d6fa9533))
* **frontend:** Add support for object editor + fix wording ([#1004](https://github.com/windmill-labs/windmill/issues/1004)) ([a562dee](https://github.com/windmill-labs/windmill/commit/a562dee3cebfc07f72f0e952cb102c4c86022937))
* implement flow as a flow step ([8c1c508](https://github.com/windmill-labs/windmill/commit/8c1c5083585f4882aac3f05f71ad1a6414772082))

## [1.53.0](https://github.com/windmill-labs/windmill/compare/v1.52.0...v1.53.0) (2022-12-05)


### Features

* add include_header to pass request headers to script ([31c317b](https://github.com/windmill-labs/windmill/commit/31c317b3581e24aa24fa41a708f080c1d1de7e0c))
* **cli:** hub sync ([#975](https://github.com/windmill-labs/windmill/issues/975)) ([2265372](https://github.com/windmill-labs/windmill/commit/22653727a4106fa604796b3958efab94762041c2))
* **frontend:** Add app preview ([#993](https://github.com/windmill-labs/windmill/issues/993)) ([c9ad638](https://github.com/windmill-labs/windmill/commit/c9ad63895891ab3bbaeab43a008573f5bd3681b5))
* **frontend:** clarified UX for connect step ([e4839e2](https://github.com/windmill-labs/windmill/commit/e4839e21ff5d60bec4499245742f2400168c70ad))
* **frontend:** introduce mysql as a script language ([#982](https://github.com/windmill-labs/windmill/issues/982)) ([e089109](https://github.com/windmill-labs/windmill/commit/e089109b50bd014c7a4f0fd7f60c53e8be63fb95))
* refactor favorite menu ([c55fae5](https://github.com/windmill-labs/windmill/commit/c55fae54dd043eb1c01a15c8005e29166a4e992b))


### Bug Fixes

* **cli:** Fix cli pull push ([#985](https://github.com/windmill-labs/windmill/issues/985)) ([1bac237](https://github.com/windmill-labs/windmill/commit/1bac23785cb6af255732b1a2551bf9ffa00e24e7))
* **frontend:** Align hub flow list + fix drawer content everywhere ([#991](https://github.com/windmill-labs/windmill/issues/991)) ([9f59a16](https://github.com/windmill-labs/windmill/commit/9f59a160c39048447ffeefc5070c52e8692c8316))
* **frontend:** Fix app InputValue sync ([#994](https://github.com/windmill-labs/windmill/issues/994)) ([e217fbf](https://github.com/windmill-labs/windmill/commit/e217fbf071fa834c4b4288f602125164bf1d93bf))
* **frontend:** fix app preview ([#979](https://github.com/windmill-labs/windmill/issues/979)) ([129a0ad](https://github.com/windmill-labs/windmill/commit/129a0ad56b58840620fdc77e619928e04c67cd1f))
* **frontend:** fix home ([#981](https://github.com/windmill-labs/windmill/issues/981)) ([fa64e83](https://github.com/windmill-labs/windmill/commit/fa64e83f7ea6bc7786a15db647319d2f2a322b5b))
* **frontend:** fix home header ([#977](https://github.com/windmill-labs/windmill/issues/977)) ([e9fa0ad](https://github.com/windmill-labs/windmill/commit/e9fa0ad0b75d0678167e7a48f8406639e85986a9))
* **frontend:** Fix home margins ([#992](https://github.com/windmill-labs/windmill/issues/992)) ([62d2a33](https://github.com/windmill-labs/windmill/commit/62d2a3343dc27317f33446918404373b7d8285f5))
* **frontend:** Make context clickable ([#984](https://github.com/windmill-labs/windmill/issues/984)) ([9264f4b](https://github.com/windmill-labs/windmill/commit/9264f4b233858537bb344355c5be43be3ec9d8d9))
* **frontend:** variables and resources uses tab navigation ([90ce431](https://github.com/windmill-labs/windmill/commit/90ce4314181d8e5031c08d5fbb75b920c33b7f75))

## [1.52.0](https://github.com/windmill-labs/windmill/compare/v1.51.0...v1.52.0) (2022-12-02)


### Features

* add favorite/star + remove flows/scripts page in favor of unified home page ([#968](https://github.com/windmill-labs/windmill/issues/968)) ([f3f694e](https://github.com/windmill-labs/windmill/commit/f3f694e9251fc62d8e3e10497e8936c588b456ba))
* **cli:** improved setup & allow workspace in base url & refactor workspaces/remotes to unify ([#966](https://github.com/windmill-labs/windmill/issues/966)) ([d3a171c](https://github.com/windmill-labs/windmill/commit/d3a171c28355c5d452e6e9caa0aa741c1ff23875))
* **cli:** Login via Frontend ([#956](https://github.com/windmill-labs/windmill/issues/956)) ([2c31a9c](https://github.com/windmill-labs/windmill/commit/2c31a9cbdf84ff2659313df799cbd79f9c167325))
* **deno-client:** support mysql ([#971](https://github.com/windmill-labs/windmill/issues/971)) ([0e402f6](https://github.com/windmill-labs/windmill/commit/0e402f6a9dfd1b6d00f6d2a951740d7aea0a8b70))
* **frontend:** Add actions to tables ([#951](https://github.com/windmill-labs/windmill/issues/951)) ([1069105](https://github.com/windmill-labs/windmill/commit/10691054510dd955a6f0d36c0186fdab9ce0facc))
* **frontend:** Add Mailchimp resource instructions ([#967](https://github.com/windmill-labs/windmill/issues/967)) ([ba90e8c](https://github.com/windmill-labs/windmill/commit/ba90e8c1b8131e1b1e38322d165c04a53a8622b2))
* **frontend:** flow status viewer include a graph ([02a9c5c](https://github.com/windmill-labs/windmill/commit/02a9c5c4eac557486df6908536a8467d68b92eca))
* **frontend:** rework script detail ([#952](https://github.com/windmill-labs/windmill/issues/952)) ([6c45fe7](https://github.com/windmill-labs/windmill/commit/6c45fe7344858761422916cc497018b35753e0ce))
* **frontend:** Update app component list ([#947](https://github.com/windmill-labs/windmill/issues/947)) ([ec1cebc](https://github.com/windmill-labs/windmill/commit/ec1cebc7920350939e365322f77898b31cafd795))
* overhaul scripts and flows page ([4946093](https://github.com/windmill-labs/windmill/commit/494609364c9d6109c08c7531cf02223793325f88))
* overhaul scripts and flows page ([c26be86](https://github.com/windmill-labs/windmill/commit/c26be86cef9d6cad44ae7cbbb5e0fd5d147c5c52))
* **python:** add support for parsing resource type in python ([63d95cf](https://github.com/windmill-labs/windmill/commit/63d95cfbb31a2b599fa9deaee203e1c4c2f0715e))
* refactor variable + resource linkage + OAuth visibility ([37967a7](https://github.com/windmill-labs/windmill/commit/37967a795006c2eb4e8b218abb3d1b0525c17d5e))
* unify resources under a single connect API ([539d6be](https://github.com/windmill-labs/windmill/commit/539d6be9088ccb2d18b0d16ca020b23bffaa79b9))


### Bug Fixes

* **backend:** support PIP_INDEX_URL ([12f9677](https://github.com/windmill-labs/windmill/commit/12f967726b96cc04e5024134216727ddfcd5fe82))
* **backend:** support PIP_INDEX_URL ([afcb44a](https://github.com/windmill-labs/windmill/commit/afcb44a12707dc3b0839182479438d2b010362ca))
* **frontend:** Fix pie animation + actions wrap ([#953](https://github.com/windmill-labs/windmill/issues/953)) ([ed7838d](https://github.com/windmill-labs/windmill/commit/ed7838d6bcf538525f6b3e4257bffe6d51318c8a))
* **frontend:** psql demo expects integers as a key ([#958](https://github.com/windmill-labs/windmill/issues/958)) ([4d8a5c4](https://github.com/windmill-labs/windmill/commit/4d8a5c4fd927e421825a9d9d2dc5dcfaf8b3949a))
* **frontend:** Refactor apps to support multiple breakpoints ([#957](https://github.com/windmill-labs/windmill/issues/957)) ([96666af](https://github.com/windmill-labs/windmill/commit/96666af3d9d6f68e4e5bb0f7a748614c9916f394))

## [1.51.0](https://github.com/windmill-labs/windmill/compare/v1.50.0...v1.51.0) (2022-11-26)


### Features

* Add notification on app save ([#943](https://github.com/windmill-labs/windmill/issues/943)) ([79cec36](https://github.com/windmill-labs/windmill/commit/79cec368ba643a88a554a88e4bc0500701e2fcc8))
* **backend:** add configurable custom client ([975a1db](https://github.com/windmill-labs/windmill/commit/975a1db10ea592038cef0c2677e66a8b6d6b8ee5))
* **cli:** Run flows & scripts ([#940](https://github.com/windmill-labs/windmill/issues/940)) ([cdd3e2c](https://github.com/windmill-labs/windmill/commit/cdd3e2cfc11cd003246643528b950cd0aafe1140))
* **frontend:** Add guard against script overwrite ([#944](https://github.com/windmill-labs/windmill/issues/944)) ([dd75b37](https://github.com/windmill-labs/windmill/commit/dd75b370afd3d7e6a112e0ec9a6444a82b5620e3))
* **frontend:** Add inline script picker to apps ([#945](https://github.com/windmill-labs/windmill/issues/945)) ([ddab2df](https://github.com/windmill-labs/windmill/commit/ddab2dffd5459a3e35a368e09a64ebcbceefc87a))
* **frontend:** flow UX overhaul II + go + python support for trigger scripts ([#928](https://github.com/windmill-labs/windmill/issues/928)) ([802abe7](https://github.com/windmill-labs/windmill/commit/802abe7f901fc93bee1be401a3166fa22b63d00c))
* **frontend:** login page makeup ([5028d86](https://github.com/windmill-labs/windmill/commit/5028d8603d08f13f4c9ae061b5aa9c6b4b5ea4f4))
* **frontend:** login page makeup ([ced2678](https://github.com/windmill-labs/windmill/commit/ced2678a21e2078973cfbe506586061f806c2dfe))
* Update apps button component with colors ([#936](https://github.com/windmill-labs/windmill/issues/936)) ([4b2b346](https://github.com/windmill-labs/windmill/commit/4b2b3467d2bbb204acd5330c4c100d63acb4e40a))


### Bug Fixes

* **backend:** bash flow lock & add flow lock tests ([#933](https://github.com/windmill-labs/windmill/issues/933)) ([4ddb3ec](https://github.com/windmill-labs/windmill/commit/4ddb3ec276ef9140e15a8604d796c3a2e6210311))
* **deno-client:** pg 0.16.1 -&gt; 0.17.0 ([ac6454b](https://github.com/windmill-labs/windmill/commit/ac6454b3835562f70694ce2b935e4b229f9118c6))
* **frontend:** add checkbox component + fix alignment ([#941](https://github.com/windmill-labs/windmill/issues/941)) ([43a1d7e](https://github.com/windmill-labs/windmill/commit/43a1d7ef2a1c9167262ea7d19cc0fb10d0493eed))
* **frontend:** Cleanup dead code ([#935](https://github.com/windmill-labs/windmill/issues/935)) ([fa4840a](https://github.com/windmill-labs/windmill/commit/fa4840ad656b2cb592c644193f617b49e53211aa))
* **frontend:** Fix context panel + delete component ([#937](https://github.com/windmill-labs/windmill/issues/937)) ([ab481b3](https://github.com/windmill-labs/windmill/commit/ab481b3096ae6390e0d08b23a6b18f0f988cf1bd))
* **frontend:** prevent runnable to run if the script is not defined ([#938](https://github.com/windmill-labs/windmill/issues/938)) ([e64195e](https://github.com/windmill-labs/windmill/commit/e64195e42b940e552d9b89b040dff4a4d0f8be37))
* **frontend:** properly refresh context panel + Adjust style in the flow editor ([#934](https://github.com/windmill-labs/windmill/issues/934)) ([b59a1de](https://github.com/windmill-labs/windmill/commit/b59a1de93baade3ad576300c07143fbd3f074054))

## [1.50.0](https://github.com/windmill-labs/windmill/compare/v1.49.1...v1.50.0) (2022-11-21)


### Features

* **deno,python:** get/set_shared_state ([c8266fb](https://github.com/windmill-labs/windmill/commit/c8266fb8b3262d9e9ec5698f824b2e9df716a228))
* **frontend:** overhaul the whole flow UX ([d23e218](https://github.com/windmill-labs/windmill/commit/d23e218e1fd9b200aaa3fff12182f18e251da796))


### Bug Fixes

* **caching:** preserve permissions ([a352975](https://github.com/windmill-labs/windmill/commit/a3529759ad34db5c8234a7886aba1c3d07a644cf))

## [1.49.1](https://github.com/windmill-labs/windmill/compare/v1.49.0...v1.49.1) (2022-11-20)


### Bug Fixes

* **caching:** add a second caching mechanism by tarring the entire cache for fast startup ([7af345e](https://github.com/windmill-labs/windmill/commit/7af345e5e57c6fbc35db9069782432664232851a))

## [1.49.0](https://github.com/windmill-labs/windmill/compare/v1.48.2...v1.49.0) (2022-11-20)


### Features

* **go:** improve cold start of 200ms by building outside of nsjail ([838a92a](https://github.com/windmill-labs/windmill/commit/838a92a0dbb75f4e7e32a7541800cbda4808cea7))
* **python-client:** remove unecessary imports in wmill to speed-up imports ([46fe9ad](https://github.com/windmill-labs/windmill/commit/46fe9ad52594d3a45b7917b91b37a83bc779bb1b))

## [1.48.2](https://github.com/windmill-labs/windmill/compare/v1.48.1...v1.48.2) (2022-11-19)


### Bug Fixes

* **go-client:** support setVariable, setResource, setState, getState ([e33bd1e](https://github.com/windmill-labs/windmill/commit/e33bd1e6b25bb9e3a3fe6f2c93d8c686c200b253))

## [1.48.1](https://github.com/windmill-labs/windmill/compare/v1.48.0...v1.48.1) (2022-11-19)


### Bug Fixes

* **python-client:** get_state on empty state return None ([968675d](https://github.com/windmill-labs/windmill/commit/968675d8d068b19413a8bca7d4cb80179646c114))

## [1.48.0](https://github.com/windmill-labs/windmill/compare/v1.47.3...v1.48.0) (2022-11-18)


### Features

* add slack_bot token on connecting workspace to slack ([b3178d1](https://github.com/windmill-labs/windmill/commit/b3178d1b8aacfa90b8a68554a186f3b26f3190ba))
* **backend:** sync cache features on all workers [enterprise] ([#907](https://github.com/windmill-labs/windmill/issues/907)) ([bd09884](https://github.com/windmill-labs/windmill/commit/bd09884955bbe04f41fbcce9b978a070145f23a3))
* **python:** add Resource[resource_type] as a parsed parameter ([9d17abb](https://github.com/windmill-labs/windmill/commit/9d17abbb12463c81de325eef875161cf86449b25))
* supercache extended to all version ([8846ca5](https://github.com/windmill-labs/windmill/commit/8846ca585699c2ec7b18b4479e895b296774ee95))


### Bug Fixes

* **backend:** saving bash script does not require dep job ([381b036](https://github.com/windmill-labs/windmill/commit/381b0368d72ad42501082c91a7c62964593ba3ad))
* **frontend:** app editor v1 ([#908](https://github.com/windmill-labs/windmill/issues/908)) ([53a8c5e](https://github.com/windmill-labs/windmill/commit/53a8c5e04cc4f407c137b0d621003dbab1bfdc67))
* **frontend:** Reduce the size of the separator + fix Auto scroll ([#895](https://github.com/windmill-labs/windmill/issues/895)) ([3f8295b](https://github.com/windmill-labs/windmill/commit/3f8295bb0c7d9e9c831e8dbcb7f1e8b944e45c66))
* support flows to be triggered by slack commands ([199a11a](https://github.com/windmill-labs/windmill/commit/199a11a8cf92691a3ac5aa7ebdc3157d10677139))

## [1.47.3](https://github.com/windmill-labs/windmill/compare/v1.47.2...v1.47.3) (2022-11-15)


### Bug Fixes

* **python-client:** fix transform_leaves ([a649f77](https://github.com/windmill-labs/windmill/commit/a649f772a564eaffb5f6192a510f7112ed618300))

## [1.47.2](https://github.com/windmill-labs/windmill/compare/v1.47.1...v1.47.2) (2022-11-15)


### Bug Fixes

* **python-client:** fix get_state ([b4fd470](https://github.com/windmill-labs/windmill/commit/b4fd4700251892116b0dff2940d98b7e473c79bf))

## [1.47.1](https://github.com/windmill-labs/windmill/compare/v1.47.0...v1.47.1) (2022-11-15)


### Bug Fixes

* **python-client:** fix set_resource ([a6a5ada](https://github.com/windmill-labs/windmill/commit/a6a5adadf45f6334eaf17f59985c0e7870f25167))

## [1.47.0](https://github.com/windmill-labs/windmill/compare/v1.46.2...v1.47.0) (2022-11-15)


### Features

* **backend:** Flow lock ([#868](https://github.com/windmill-labs/windmill/issues/868)) ([47c9ff1](https://github.com/windmill-labs/windmill/commit/47c9ff1edc28b63a1a16ffce08d3751a4f8f5422))
* **backend:** remove go.sum from go lockfile ([#891](https://github.com/windmill-labs/windmill/issues/891)) ([3357cff](https://github.com/windmill-labs/windmill/commit/3357cffb043254d8712a2afe2729533d5884d56f))
* **clients:** rename internal state as state + setters for resources/variables in python ([32bca1f](https://github.com/windmill-labs/windmill/commit/32bca1fd4cd0714a9f18a508b0e0782f63ee25a8))


### Bug Fixes

* **backend:** go use windmill cache dir even if nsjail disabled ([a9abd28](https://github.com/windmill-labs/windmill/commit/a9abd288822731add05d00e3d3fc43d29e11c7cb))
* **frontend:** add size prop to tabs ([#894](https://github.com/windmill-labs/windmill/issues/894)) ([e8d3a0e](https://github.com/windmill-labs/windmill/commit/e8d3a0efb1e23ae66d755489f96f09932544be9c))
* **frontend:** App Editor v0 ([#886](https://github.com/windmill-labs/windmill/issues/886)) ([cc5f629](https://github.com/windmill-labs/windmill/commit/cc5f629a7b142a2bd0ce7ca8950e24f6cb5473ff))
* **frontend:** Set settings as header and error handler as footer ([#893](https://github.com/windmill-labs/windmill/issues/893)) ([4dc05b9](https://github.com/windmill-labs/windmill/commit/4dc05b913e4d98dd37b032639831d20aa662e4e9))

## [1.46.2](https://github.com/windmill-labs/windmill/compare/v1.46.1...v1.46.2) (2022-11-12)


### Bug Fixes

* **ci:** sqlx offline data ([76a6768](https://github.com/windmill-labs/windmill/commit/76a6768ed9ab223363f47c62cfcd8c51dd624b62))

## [1.46.1](https://github.com/windmill-labs/windmill/compare/v1.46.0...v1.46.1) (2022-11-12)


### Bug Fixes

* **backend:** apps backend v0 ([#888](https://github.com/windmill-labs/windmill/issues/888)) ([2d9e990](https://github.com/windmill-labs/windmill/commit/2d9e9909da5b82eda39eb99c870f073b869b6ff5))

## [1.46.0](https://github.com/windmill-labs/windmill/compare/v1.45.0...v1.46.0) (2022-11-12)


### Features

* **cli:** Relax push folder layout to accept one layer of organizational structure ([#882](https://github.com/windmill-labs/windmill/issues/882)) ([a658308](https://github.com/windmill-labs/windmill/commit/a658308b59d7ef51d1aa6cda7598947ed0ce7548))
* **cli:** Tarball pull ([#867](https://github.com/windmill-labs/windmill/issues/867)) ([d375836](https://github.com/windmill-labs/windmill/commit/d375836989fd730acbb4a04218d143b9fef63e0d))
* deprecate previous_result in favor of results per id ([40183ce](https://github.com/windmill-labs/windmill/commit/40183ce4e42f648d9eb6e2765fb141e16eba908e))
* **frontend:** Flow graph ([#827](https://github.com/windmill-labs/windmill/issues/827)) ([9bf0f6e](https://github.com/windmill-labs/windmill/commit/9bf0f6e70d7501737a61e4d62d116d44b1f136df))
* publish arm64 image ([#885](https://github.com/windmill-labs/windmill/issues/885)) ([c3b2bab](https://github.com/windmill-labs/windmill/commit/c3b2bab5d1a7eee49c517c2c8c5e9108c3f32333))

## [1.45.0](https://github.com/windmill-labs/windmill/compare/v1.44.0...v1.45.0) (2022-11-06)


### Features

* **backend:** add global delete user endpoint ([23a0c10](https://github.com/windmill-labs/windmill/commit/23a0c10b77a430b274e7023078f1a7a963e490d2))
* **backend:** flow duration is now computed as the sum of every child ([badc601](https://github.com/windmill-labs/windmill/commit/badc60193c2480f93056eee5be6548bcf49fc1fc))
* **backend:** use result_by_id in branchone ([#857](https://github.com/windmill-labs/windmill/issues/857)) ([0170188](https://github.com/windmill-labs/windmill/commit/01701882dc168862219ac4e3cf53621e1937b013))
* **frontend:** fill schema and test args from payload ([cc65bf5](https://github.com/windmill-labs/windmill/commit/cc65bf5f48447cd52547a50a714ece38f5c445f7))
* **frontend:** show runs using a time chart ([b31c5c4](https://github.com/windmill-labs/windmill/commit/b31c5c435e9aa8268e5c4f5771bb444182f76a01))
* support bash as 4th language ([#865](https://github.com/windmill-labs/windmill/issues/865)) ([3c09275](https://github.com/windmill-labs/windmill/commit/3c0927596078eb68a9066663fb5a3bd5202c1850))


### Bug Fixes

* **backend:** improve csp ([#861](https://github.com/windmill-labs/windmill/issues/861)) ([3ba1870](https://github.com/windmill-labs/windmill/commit/3ba18700dea282837d1bb27f24ed50ad1c417063))
* **backend:** tighten http security headers ([#860](https://github.com/windmill-labs/windmill/issues/860)) ([7040bbe](https://github.com/windmill-labs/windmill/commit/7040bbe4c92c522d0815bc93c36604accd321bd5))
* **backend:** tighten security around cookies to avoid csrf ([#859](https://github.com/windmill-labs/windmill/issues/859)) ([cddec64](https://github.com/windmill-labs/windmill/commit/cddec6469e7f3a082504f181de3785a2759b0a16))
* **frontend:** dispose monaco models onDestroy ([83c79a4](https://github.com/windmill-labs/windmill/commit/83c79a47eefe63aee3ecb9e009323d561b8b662f))
* **frontend:** fix remaining openModal bugs ([49bebe2](https://github.com/windmill-labs/windmill/commit/49bebe20cc87b5ce078d04f7fad9003d2e26bbf6))
* **frontend:** go editor nits ([971988d](https://github.com/windmill-labs/windmill/commit/971988dfe222ebee4fa2a8b796f50f57f0a291a0))
* **frontend:** reload websocket on lsp go import install ([5b4c9d9](https://github.com/windmill-labs/windmill/commit/5b4c9d9eb044a68a278c069fd1932a0b8c19b5d1))
* **frontend:** reset rows default to 1 ([175a188](https://github.com/windmill-labs/windmill/commit/175a188f61f344c830d937e854cd4f4d77069fcb))

## [1.44.0](https://github.com/windmill-labs/windmill/compare/v1.43.2...v1.44.0) (2022-11-03)


### Features

* **backend:** Deno lock files ([#851](https://github.com/windmill-labs/windmill/issues/851)) ([5bbfb40](https://github.com/windmill-labs/windmill/commit/5bbfb40ee1114d83bf0a277fa991aa70d5be8a62))
* implement allowed domains for self-hosted ([513924b](https://github.com/windmill-labs/windmill/commit/513924b0437a1d80720ac5bd1f38c33f97839d28))


### Bug Fixes

* **backend:** capture up all lockfile issues ([35868ef](https://github.com/windmill-labs/windmill/commit/35868ef9bf1eac650cbb735807aebc5a604dd5d6))
* implement require admin differently than unauthorized ([14c296d](https://github.com/windmill-labs/windmill/commit/14c296dbb85131c355980cd416c26a88c4823978))
* **python-client:** fix get_resource ([20bc904](https://github.com/windmill-labs/windmill/commit/20bc904e5fa3b97192d9cf7b2b70bdbde0408913))

## [1.43.2](https://github.com/windmill-labs/windmill/compare/v1.43.1...v1.43.2) (2022-11-02)


### Bug Fixes

* **go-client:** use stable oapi codegen version ([4707d1e](https://github.com/windmill-labs/windmill/commit/4707d1ecaafa10b9cf8737e18ab432b3855c0c7f))

## [1.43.1](https://github.com/windmill-labs/windmill/compare/v1.43.0...v1.43.1) (2022-11-02)


### Bug Fixes

* **backend:** extend default scope set for slack resource ([#848](https://github.com/windmill-labs/windmill/issues/848)) ([ffaf7ca](https://github.com/windmill-labs/windmill/commit/ffaf7cad4a76e1c520071877579485b4c757c65e))
* **go-client:** fix openapi generation ([1329493](https://github.com/windmill-labs/windmill/commit/1329493873fb18b373c879f3f153fdf2a5036405))

## [1.43.0](https://github.com/windmill-labs/windmill/compare/v1.42.1...v1.43.0) (2022-11-01)


### Features

* **backend:** add parallel option for forloop and branchall ([#840](https://github.com/windmill-labs/windmill/issues/840)) ([39937e6](https://github.com/windmill-labs/windmill/commit/39937e6a83c3b7ec9dd889b40c10004abb8938a7))
* new wmill CLI [#831](https://github.com/windmill-labs/windmill/issues/831) ([f5ea13a](https://github.com/windmill-labs/windmill/commit/f5ea13ab2b2f7f8735504099d0267c32ac8ca6f2))

## [1.42.1](https://github.com/windmill-labs/windmill/compare/v1.42.0...v1.42.1) (2022-10-30)


### Bug Fixes

* **deno-client:** add missing approver encoding to hmac api request ([#829](https://github.com/windmill-labs/windmill/issues/829)) ([eef7c7f](https://github.com/windmill-labs/windmill/commit/eef7c7ff9442b818a87f63439726efc89395cb07))

## [1.42.0](https://github.com/windmill-labs/windmill/compare/v1.41.0...v1.42.0) (2022-10-30)


### Features

* **frontend:** Flow editor branches ([#727](https://github.com/windmill-labs/windmill/issues/727)) ([054c142](https://github.com/windmill-labs/windmill/commit/054c142882d4dc7b097fb04def0595e79ab81b75))
* **frontend:** result by id ([6fcf984](https://github.com/windmill-labs/windmill/commit/6fcf984ea344331ee96fcb7b42b5ac7a91a6e00e))
* **frontend:** Update progress bar ([#770](https://github.com/windmill-labs/windmill/issues/770)) ([17e766a](https://github.com/windmill-labs/windmill/commit/17e766aa6e252419e4395cca9c56e707fe9247b3))
* payload capture of json to initialize flow input ([#655](https://github.com/windmill-labs/windmill/issues/655)) ([9a67607](https://github.com/windmill-labs/windmill/commit/9a67607b20896b2efa65863604d8cb791c9943b5))
* **python:** type is automatically inferred from default parameters ([84a3fbe](https://github.com/windmill-labs/windmill/commit/84a3fbe46b4efb321b3b676258b1fc59cd67b186))


### Bug Fixes

* **backend:** fix error handler progress update ([4bd74ad](https://github.com/windmill-labs/windmill/commit/4bd74ad7232755a3c2d911d5284282bb1fb4f430))
* **deno-client:** automatically encode approver param + refactor: use URL class to format urls ([#809](https://github.com/windmill-labs/windmill/issues/809)) ([10e1de8](https://github.com/windmill-labs/windmill/commit/10e1de84760b6b7eec92397117c44a938b0bc358))
* **frontend:** Add summary to the script editor ([#825](https://github.com/windmill-labs/windmill/issues/825)) ([79e8b1f](https://github.com/windmill-labs/windmill/commit/79e8b1ff75b76d6a5c2f80079255124014a2c813))
* **frontend:** Fix input transforms ([#813](https://github.com/windmill-labs/windmill/issues/813)) ([53eede4](https://github.com/windmill-labs/windmill/commit/53eede4f02c01c9dce0c10e4439a3cc2687010ac))
* **frontend:** Fix legacy input transforms ([#814](https://github.com/windmill-labs/windmill/issues/814)) ([b078bde](https://github.com/windmill-labs/windmill/commit/b078bde30528dbbadf41cfacaf46223317795a2e))
* **frontend:** Fix overlay map indicator ([#816](https://github.com/windmill-labs/windmill/issues/816)) ([a65c4c3](https://github.com/windmill-labs/windmill/commit/a65c4c35709e199943499304d4b04ce4fbbd1a98))

## [1.41.0](https://github.com/windmill-labs/windmill/compare/v1.40.1...v1.41.0) (2022-10-24)


### Features

* add approver to approval step ([a0b2c9e](https://github.com/windmill-labs/windmill/commit/a0b2c9e77dd77e5727b2921890b1298cbac780f9))


### Bug Fixes

* approval pages now require no auth ([3c91e42](https://github.com/windmill-labs/windmill/commit/3c91e42b9ec185d7ae17c76f82511f6caa4837de))
* **deno-client:** add approver ([17d9f38](https://github.com/windmill-labs/windmill/commit/17d9f38d307c6a8554e20b60aabe675e43df10fd))

## [1.40.1](https://github.com/windmill-labs/windmill/compare/v1.40.0...v1.40.1) (2022-10-22)


### Bug Fixes

* **deno-client:** fix build.sh to have reproducible builds ([#793](https://github.com/windmill-labs/windmill/issues/793)) ([a5dfd86](https://github.com/windmill-labs/windmill/commit/a5dfd865c3912bb8528c0048519ad4c134eceab2))

## [1.40.0](https://github.com/windmill-labs/windmill/compare/v1.39.0...v1.40.0) (2022-10-22)


### Features

* **backend:** propagate cancel instantly to all flow jobs if any ([cb5ed9b](https://github.com/windmill-labs/windmill/commit/cb5ed9b9a1fdcaf5609ce20c59aeca2356ae1883))
* **deno-client:** improve docs by extending function signatures ([#791](https://github.com/windmill-labs/windmill/issues/791)) ([4ab547b](https://github.com/windmill-labs/windmill/commit/4ab547bdf4e93793306b7f98bf0e237849aa391a))
* support running and publishing go, python scripts to the hub ([#779](https://github.com/windmill-labs/windmill/issues/779)) ([8ec33c5](https://github.com/windmill-labs/windmill/commit/8ec33c5e165316e2f8f804575ea3369b8beefdbd))


### Bug Fixes

* **backend:** avoid mem leak on interval [#786](https://github.com/windmill-labs/windmill/issues/786) ([ac84b76](https://github.com/windmill-labs/windmill/commit/ac84b76909e0d6dfa170cb58608344b1b6d2627f))
* **frontend:** rework te new script page ([6c68f26](https://github.com/windmill-labs/windmill/commit/6c68f264cbcf18a872775b37be40b4f09dee8e2b))
* improve approval flow with approval page ([884edd7](https://github.com/windmill-labs/windmill/commit/884edd77153100a26a72c28c52b76c9619bd7642))
* only create a schedule after flow change if schedule is enabled ([4ce3e07](https://github.com/windmill-labs/windmill/commit/4ce3e0795c000aeff6f729ed515091fb93f7ceb2))

## [1.39.0](https://github.com/windmill-labs/windmill/compare/v1.38.5...v1.39.0) (2022-10-20)


### Features

* add ids to modules + input_transform lowered to flowmodulevalue ([#768](https://github.com/windmill-labs/windmill/issues/768)) ([af9e1f4](https://github.com/windmill-labs/windmill/commit/af9e1f4479604df53c1bdc2488867a0033abdc70))
* add result by id to fetch result from any node ([#769](https://github.com/windmill-labs/windmill/issues/769)) ([57600ab](https://github.com/windmill-labs/windmill/commit/57600ab873a78435c5b930465ac466f69711e540))
* **backend:** add branch all ([#751](https://github.com/windmill-labs/windmill/issues/751)) ([a5aad94](https://github.com/windmill-labs/windmill/commit/a5aad947e6402a174b0d4703e227e2370618292f))
* **backend:** atomic moving queue -> complete and delete ([#771](https://github.com/windmill-labs/windmill/issues/771)) ([45a6976](https://github.com/windmill-labs/windmill/commit/45a6976d52829f181805281d78a741653e41b25c))
* **backend:** rework forloop flow job arg passing + reimplement branchone using flows ([b180569](https://github.com/windmill-labs/windmill/commit/b1805699c9af759375b96969f1f9a0fd71ca6508))
* **benchmark:** Initial Benchmarking Tool ([#731](https://github.com/windmill-labs/windmill/issues/731)) ([846462c](https://github.com/windmill-labs/windmill/commit/846462c68bf1a57523582c5e821e58a1f8b3886e))
* **frontend:** publish script of any lang to hub ([1a93593](https://github.com/windmill-labs/windmill/commit/1a935935291bcb01bb8b7cc037949fb6b36afff0))
* **frontend:** Update split panes ([#741](https://github.com/windmill-labs/windmill/issues/741)) ([8a774e0](https://github.com/windmill-labs/windmill/commit/8a774e0d042ed9a05b45cd8a85ba67c78eacc630))
* **frontend:** Update workspace selector ([#754](https://github.com/windmill-labs/windmill/issues/754)) ([582fc9a](https://github.com/windmill-labs/windmill/commit/582fc9a2eda1e618a5a834bc79263e91a14ba26b))
* InProgress forloop_jobs -> flow_jobs to unify with branchAll ([9e0c2d7](https://github.com/windmill-labs/windmill/commit/9e0c2d759b6db2061905677172a6d46f0bde684e))


### Bug Fixes

* **backend:** reschedule flow at first step end ([#746](https://github.com/windmill-labs/windmill/issues/746)) ([955cc41](https://github.com/windmill-labs/windmill/commit/955cc4104ae229544f83cf4d6ae9f3bda5df0e8a))
* **deno-client:** error handling for getInternalState ([5117430](https://github.com/windmill-labs/windmill/commit/5117430b16c2f741b09702058a26d52aaafdaebe))
* **frontend:** Fix text styling ([#753](https://github.com/windmill-labs/windmill/issues/753)) ([99e60b1](https://github.com/windmill-labs/windmill/commit/99e60b1b7423787f4cf48f66bc77d949c4687667))
* **frontend:** Style fix ([#755](https://github.com/windmill-labs/windmill/issues/755)) ([9edb8a8](https://github.com/windmill-labs/windmill/commit/9edb8a8e1ce5fbe58bb89c4cd810e1c1e2f4303b))

## [1.38.5](https://github.com/windmill-labs/windmill/compare/v1.38.4...v1.38.5) (2022-10-15)


### Bug Fixes

* **deno-client:** use proper base url ([bb1750f](https://github.com/windmill-labs/windmill/commit/bb1750fd6dddaa1235deafe0a68467f3a631a8e9))

## [1.38.4](https://github.com/windmill-labs/windmill/compare/v1.38.3...v1.38.4) (2022-10-15)


### Bug Fixes

* refactor deno client to use another openapi generator [#743](https://github.com/windmill-labs/windmill/issues/743) ([350d31f](https://github.com/windmill-labs/windmill/commit/350d31fe068260820978b8a629a74da80384f037))

## [1.38.3](https://github.com/windmill-labs/windmill/compare/v1.38.2...v1.38.3) (2022-10-15)


### Bug Fixes

* **go-client:** go-client README ([8d37e40](https://github.com/windmill-labs/windmill/commit/8d37e40fced961c15fc6cd2198c4e696952f392c))

## [1.38.2](https://github.com/windmill-labs/windmill/compare/v1.38.1...v1.38.2) (2022-10-15)


### Bug Fixes

* **go-client:** improve go-client error handling ([467ff10](https://github.com/windmill-labs/windmill/commit/467ff105db34c7e2bd028d35dff18a08df599a4c))
* **go-client:** improve go-client variable and resource handling ([fffcb5e](https://github.com/windmill-labs/windmill/commit/fffcb5ec2a47efcb9ba8db6211314d67f38f5b24))
* **go-client:** return error ([1f7ef30](https://github.com/windmill-labs/windmill/commit/1f7ef3006f551a324b8b8f5e7d260d69287eb4cf))
* **python-client:** provide backwards compatibility down to python3.7 ([#738](https://github.com/windmill-labs/windmill/issues/738)) ([#739](https://github.com/windmill-labs/windmill/issues/739)) ([e4cd931](https://github.com/windmill-labs/windmill/commit/e4cd931ab5d212e5bd8ed32f5fa1a33b431d16a4))

## [1.38.1](https://github.com/windmill-labs/windmill/compare/v1.38.0...v1.38.1) (2022-10-14)


### Bug Fixes

* **go-client:** pass bearer token to requests ([9d38d66](https://github.com/windmill-labs/windmill/commit/9d38d66d2b6571d9ae7cbdb71d105790273155ca))

## [1.38.0](https://github.com/windmill-labs/windmill/compare/v1.37.0...v1.38.0) (2022-10-14)


### Features

* **backend:** implement new OpenFlow module Branches ([#692](https://github.com/windmill-labs/windmill/issues/692)) ([cc07a6b](https://github.com/windmill-labs/windmill/commit/cc07a6b7e4572f239b11ff566d616bcf66952a1b))
* **backend:** supercache for python heavy dependencies in alpha ([7e35d99](https://github.com/windmill-labs/windmill/commit/7e35d9989aab74cd91f676c679b36e98033f1176))
* **frontend:** Loading placeholder ([#707](https://github.com/windmill-labs/windmill/issues/707)) ([9acee22](https://github.com/windmill-labs/windmill/commit/9acee22b1fc0b4eb82a1b47bc62598fe5af076e1))
* **frontend:** Typography update ([#725](https://github.com/windmill-labs/windmill/issues/725)) ([2c1cd7e](https://github.com/windmill-labs/windmill/commit/2c1cd7eea8250f02588bc151bab8faf07ee7133d))
* secure suspended resume event + configurable timeout ([#721](https://github.com/windmill-labs/windmill/issues/721)) ([ff7fb0f](https://github.com/windmill-labs/windmill/commit/ff7fb0f6f361322fbd3a1024c1604907d71aa4c9))
* support struct in Go as script parameters [#705](https://github.com/windmill-labs/windmill/issues/705) ([7bdbfec](https://github.com/windmill-labs/windmill/commit/7bdbfec71a9a02ebbf4117c0e16e7249a0e028e6))


### Bug Fixes

* **deno:** approval endpoints generator ([#728](https://github.com/windmill-labs/windmill/issues/728)) ([af8a421](https://github.com/windmill-labs/windmill/commit/af8a4216f8c3960e8ae5f930d4303bda7eee5c2b))
* **frontend:** Apply small text size to hljs ([#706](https://github.com/windmill-labs/windmill/issues/706)) ([8be31d6](https://github.com/windmill-labs/windmill/commit/8be31d608b928a0ba8d8c53cbfb87c4915e41c20))
* **frontend:** do not alert on non internal nav for unconfirmed saves ([e5fdbff](https://github.com/windmill-labs/windmill/commit/e5fdbff8ec42ba1f581b0b94ef4ace0380a91d8a))
* **frontend:** do not alert on non internal nav for unconfirmed saves ([24a2932](https://github.com/windmill-labs/windmill/commit/24a2932a7bddc13bddde760655bff44202e96d01))
* **frontend:** fix viewscript for go ([e840522](https://github.com/windmill-labs/windmill/commit/e840522822c905be8fcfdeadde23ce76293d7755))
* **frontend:** go websockets ([154796c](https://github.com/windmill-labs/windmill/commit/154796cdb692cf068afec53dc080c838df273ae6))
* **frontend:** remove flowbite svelte dependency from shared Badge ([#722](https://github.com/windmill-labs/windmill/issues/722)) ([ca991d0](https://github.com/windmill-labs/windmill/commit/ca991d0fa10d2f8778512f67b1230b5922bbb980))
* **frontend:** Update skeleton animation timings ([#730](https://github.com/windmill-labs/windmill/issues/730)) ([2e21fb4](https://github.com/windmill-labs/windmill/commit/2e21fb43d5edbf4f8e271bff8a6d6fa3736a79f7))

## [1.37.0](https://github.com/windmill-labs/windmill/compare/v1.36.0...v1.37.0) (2022-10-08)


### Features

* add go LSP ([#699](https://github.com/windmill-labs/windmill/issues/699)) ([6cb3fbc](https://github.com/windmill-labs/windmill/commit/6cb3fbc8b71f5c30aa860d60be4b327a3f658d54))
* **backend:** add WM_BASE_URL ([612f727](https://github.com/windmill-labs/windmill/commit/612f7272a9cf19ed8b738da90b0234a349b32354))
* **backend:** separate properly logs from result ([6ebedfc](https://github.com/windmill-labs/windmill/commit/6ebedfc5fb8637919b2e409d14f4f06bde83fc58))
* **frontend:** Add action bar to run details ([#684](https://github.com/windmill-labs/windmill/issues/684)) ([4e472f5](https://github.com/windmill-labs/windmill/commit/4e472f5a3950d4dc5959c1c6ec21345b4d6e4a7d))
* **frontend:** add input transforms for flow loop ([b1b418a](https://github.com/windmill-labs/windmill/commit/b1b418a36265f91cad4072dc66a8edfec6994465))
* **frontend:** add prop picker to iterator ([0c25d80](https://github.com/windmill-labs/windmill/commit/0c25d80578449458d5a481f206f8b6fdb675c04e))
* **frontend:** add prop picker to iterator ([ee15bd9](https://github.com/windmill-labs/windmill/commit/ee15bd9a9df9047105e5e86ca9f6c7f489782efd))
* **frontend:** add variables and resources to the prop picker ([84a6441](https://github.com/windmill-labs/windmill/commit/84a6441b9a9b8fc753006b71cde6595d76e5e2b6))
* **frontend:** Button with popup ([#639](https://github.com/windmill-labs/windmill/issues/639)) ([fcb1c39](https://github.com/windmill-labs/windmill/commit/fcb1c39d96792e60b30e64fcd4b425df74494b13))
* **frontend:** Discard changes confirmation modal ([#653](https://github.com/windmill-labs/windmill/issues/653)) ([0e23d2d](https://github.com/windmill-labs/windmill/commit/0e23d2d60479e1b2d5654cdb7cdf8dd3b345052b))
* **frontend:** prop picker for stop condition ([e772f03](https://github.com/windmill-labs/windmill/commit/e772f0377e1c85baf3657a3cbe4e5bc423bb210c))
* **frontend:** remove step 2 for flows ([ad0ffb5](https://github.com/windmill-labs/windmill/commit/ad0ffb5eb60b3d6a119209c048123a027fb969ae))
* implement same_worker openflow attribute for running flow all in one go + sharing folder `/shared` ([#689](https://github.com/windmill-labs/windmill/issues/689)) ([f4caa4f](https://github.com/windmill-labs/windmill/commit/f4caa4ffa666de68538d7fa218e4c25315307501))
* individual retry + flow UX refactor ([c207745](https://github.com/windmill-labs/windmill/commit/c207745fa7031c6106ef7796879252ef508f552a))
* sleep for arbitrary number of seconds statically or with a javascript expression ([#691](https://github.com/windmill-labs/windmill/issues/691)) ([a084366](https://github.com/windmill-labs/windmill/commit/a08436622b1a6460fab71ee2c6acc42c0e96fd29))


### Bug Fixes

* add step to running badge in flow viewer ([895fe10](https://github.com/windmill-labs/windmill/commit/895fe106f8f1995acbdb48e24ac2c6592c7c7e12))
* **backend:** go lock dependency with no requirements ([22c4a3b](https://github.com/windmill-labs/windmill/commit/22c4a3b37574b7dfab7dde0420dd40235acec350))
* **backend:** same_worker uses the same folder even within loops ([2c5b32b](https://github.com/windmill-labs/windmill/commit/2c5b32bdb796e40b8f6ddcdb1b8b6479a5d188b5))
* change command behavior for monacos ([0a67d3f](https://github.com/windmill-labs/windmill/commit/0a67d3fb87c7270b6bbf6cd065e4ccc5a7db9dcc))
* **frontend:** Align Settings button + add missing suspend shortcut ([#694](https://github.com/windmill-labs/windmill/issues/694)) ([b59d1f8](https://github.com/windmill-labs/windmill/commit/b59d1f8717bbbd45a910204c1756bc229bd51f58))
* **frontend:** clear interval on job run ([065dcc9](https://github.com/windmill-labs/windmill/commit/065dcc9196e9bb59e8fd1fe1a31c91003083cf1b))
* **frontend:** Remove legacy tabs ([#695](https://github.com/windmill-labs/windmill/issues/695)) ([e424b6b](https://github.com/windmill-labs/windmill/commit/e424b6b9b9229588478cb8a580334a7191269d29))
* **frontend:** split early stop + fix highlight code ([5d46496](https://github.com/windmill-labs/windmill/commit/5d464963429700b87399e9d46cdb540a131a7352))
* **frontend:** split early stop + fix highlight code ([e8f2d38](https://github.com/windmill-labs/windmill/commit/e8f2d38f471d5b2daf704352ee9ae10989a2da29))
* get info about kill reason ([8accb59](https://github.com/windmill-labs/windmill/commit/8accb59a8c82e1eb8e038d38c8c8831dfe865791))
* get info about kill reason ([b31e72a](https://github.com/windmill-labs/windmill/commit/b31e72a620d00390e1373b618fe2aae4f81e9d00))
* only display error handler span if toggled on ([ce0a410](https://github.com/windmill-labs/windmill/commit/ce0a4108236e06036d06e18ece0a227f4471d9b3))

## [1.36.0](https://github.com/windmill-labs/windmill/compare/v1.35.0...v1.36.0) (2022-10-02)


### Features

* add iterator expression tooltip ([#638](https://github.com/windmill-labs/windmill/issues/638)) ([a494975](https://github.com/windmill-labs/windmill/commit/a494975e69da983aba795432da668644e13dc809))
* add private registries pip ([#636](https://github.com/windmill-labs/windmill/issues/636)) ([ae3f86d](https://github.com/windmill-labs/windmill/commit/ae3f86db112407f7684209463e1201ccc3d2349d))
* **backend:** add WM_FLOW_JOB_ID ([d863b1e](https://github.com/windmill-labs/windmill/commit/d863b1ed909dfd3006a62085de957f4385e6e0a4))
* **backend:** flow suspend resume ([#522](https://github.com/windmill-labs/windmill/issues/522)) ([126dd24](https://github.com/windmill-labs/windmill/commit/126dd24c710e3f5d261e6a3bb9e29d476e9d51eb))
* **dev:** setup devcontainer ([#549](https://github.com/windmill-labs/windmill/issues/549)) ([b78f2d1](https://github.com/windmill-labs/windmill/commit/b78f2d1a91968e840e8fd75562b49f9d2a5ba1b6))
* **front:** Add a confirmation modal ([#634](https://github.com/windmill-labs/windmill/issues/634)) ([876dc60](https://github.com/windmill-labs/windmill/commit/876dc6061007c751ce7facf2e31c6d74c54a9e31))
* **front:** Confirmation modal when deleting a resource or a variable ([#648](https://github.com/windmill-labs/windmill/issues/648)) ([bbaba14](https://github.com/windmill-labs/windmill/commit/bbaba142ac1e49028d509103ecd42626d9a25477))
* **frontend:** Add a split panel in the test tab ([#619](https://github.com/windmill-labs/windmill/issues/619)) ([5146c37](https://github.com/windmill-labs/windmill/commit/5146c37baf9be6406acd6efc0d00fcda48a8d082))
* **frontend:** Add contextual actions to insert variables or resources ([#629](https://github.com/windmill-labs/windmill/issues/629)) ([13cfed6](https://github.com/windmill-labs/windmill/commit/13cfed6d895d6e3595bdfd89f54bf80da780c01f))
* **frontend:** Add support for failure modules ([#612](https://github.com/windmill-labs/windmill/issues/612)) ([025d31f](https://github.com/windmill-labs/windmill/commit/025d31f843bbf80f38e0540f16b245bff555464b))
* **frontend:** Add support for retries for flows ([#607](https://github.com/windmill-labs/windmill/issues/607)) ([0f33c26](https://github.com/windmill-labs/windmill/commit/0f33c26d54d23571d9d6bfab525be8145c221823))
* **frontend:** Badge component and script page ([#617](https://github.com/windmill-labs/windmill/issues/617)) ([f4c8636](https://github.com/windmill-labs/windmill/commit/f4c8636209ecf4d26e2b107393160313990d9cbb))
* **frontend:** Button component ([#616](https://github.com/windmill-labs/windmill/issues/616)) ([e8e4199](https://github.com/windmill-labs/windmill/commit/e8e4199c5ced73fc4532c48d1c68200e0efd4f1f))
* **frontend:** Extract publish to hub button ([#620](https://github.com/windmill-labs/windmill/issues/620)) ([2d02558](https://github.com/windmill-labs/windmill/commit/2d0255824c23fb61936cd50ff5ea1d6c852aeabb))
* **frontend:** Flow UX entire rework ([#552](https://github.com/windmill-labs/windmill/issues/552)) ([9fa4d01](https://github.com/windmill-labs/windmill/commit/9fa4d01e3b506e4ac2497f1b6897927204e05e95))
* **frontend:** Landing rework ([#630](https://github.com/windmill-labs/windmill/issues/630)) ([941fe71](https://github.com/windmill-labs/windmill/commit/941fe7146e53434ab2b5e89bbdafa6a1dccb22fc))
* **frontend:** merge logs and result tab in script editor ([#622](https://github.com/windmill-labs/windmill/issues/622)) ([bcb1136](https://github.com/windmill-labs/windmill/commit/bcb113682f5ef68475875706aef63af83a3f3f70))
* **frontend:** Prop picker panel ([#605](https://github.com/windmill-labs/windmill/issues/605)) ([9ef6663](https://github.com/windmill-labs/windmill/commit/9ef6663dc528ab5b0e7bc54e5eafb3249080248a))
* **frontend:** rich renderer improvements ([2e101a0](https://github.com/windmill-labs/windmill/commit/2e101a0c3b1d3c25e33a7aed27fccf9f56ab60c2))
* **frontend:** Script page action row ([#626](https://github.com/windmill-labs/windmill/issues/626)) ([b10b1cc](https://github.com/windmill-labs/windmill/commit/b10b1cc90a8ebc94b55138467e72007f585f8e89))
* **front:** Rework how summaries are edited in the flow editor ([#632](https://github.com/windmill-labs/windmill/issues/632)) ([b0ac674](https://github.com/windmill-labs/windmill/commit/b0ac674f46303068a7c45a2fb3cd811f499e2fbd))
* implement go support ([#571](https://github.com/windmill-labs/windmill/issues/571)) ([39918a9](https://github.com/windmill-labs/windmill/commit/39918a9bb149dcf64e26018622a2a4214aa9faf1))
* is_trigger is just a type tag, soon to include failure and command ([#523](https://github.com/windmill-labs/windmill/issues/523)) ([e9abcff](https://github.com/windmill-labs/windmill/commit/e9abcffdd1e4087069dda3550ec29d8efbfda772))
* **job:** run job by hash ([#551](https://github.com/windmill-labs/windmill/issues/551)) ([6f09405](https://github.com/windmill-labs/windmill/commit/6f09405c2daabca8418389d99582ef602f00ab72))


### Bug Fixes

* **backend:** allow for now payload on resume GET ([6fe5b8d](https://github.com/windmill-labs/windmill/commit/6fe5b8d6b7f674b0ff70dbc828f89f26a7f91335))
* change string default input behavior for input arg ([5406a70](https://github.com/windmill-labs/windmill/commit/5406a704079dce286c3c797bef3acb3d7a073b6c))
* **frontend:** do only one request if job is completed [related to [#649](https://github.com/windmill-labs/windmill/issues/649)] ([#651](https://github.com/windmill-labs/windmill/issues/651)) ([6b6f1b4](https://github.com/windmill-labs/windmill/commit/6b6f1b407fff38959ec5d93254b547ec99b8f9f9))
* **frontend:** don't loop for completed jobs [[#649](https://github.com/windmill-labs/windmill/issues/649)] ([#650](https://github.com/windmill-labs/windmill/issues/650)) ([9592c92](https://github.com/windmill-labs/windmill/commit/9592c92f70ce9b94e141031c663ccb0cf01ef7d7))
* **frontend:** Fix buttons spacings ([#627](https://github.com/windmill-labs/windmill/issues/627)) ([d2e5168](https://github.com/windmill-labs/windmill/commit/d2e516822277948005fb5fd6596c7b9b9119ec7a))
* **frontend:** Fix flow preview inputs display to avoid hiding results ([#581](https://github.com/windmill-labs/windmill/issues/581)) ([e2924d5](https://github.com/windmill-labs/windmill/commit/e2924d581e595906cc0cda5e86c0782289dbfe23))
* **frontend:** Hide the editor panel when we are editing a PathScript ([#631](https://github.com/windmill-labs/windmill/issues/631)) ([deb0b47](https://github.com/windmill-labs/windmill/commit/deb0b47a5f0f7b450b65ebd7003a2bdf9f81c798))
* **frontend:** increase the default size of the log and result panel for the script editor ([08edcb2](https://github.com/windmill-labs/windmill/commit/08edcb24cac2fb0a0f09f16e26943b0d8eb69c2c))
* **frontend:** loading flows with for loops + flowStatusViewer treat single jobs properly ([40160c0](https://github.com/windmill-labs/windmill/commit/40160c03f17d0f8a8e56dfaa4ef2d73315718418))
* **frontend:** rework the error handler script picker ([eee7067](https://github.com/windmill-labs/windmill/commit/eee7067074e8560c2fd883e574e314b4fd87c637))
* **frontend:** Support of suspend & stop expression + restore import/export menu ([#580](https://github.com/windmill-labs/windmill/issues/580)) ([a85302c](https://github.com/windmill-labs/windmill/commit/a85302c1c37eba9c8eb3de9cab18826dc60228cb))
* **frontend:** variable editor now acceps including 3000 chars + show length ([b9518d7](https://github.com/windmill-labs/windmill/commit/b9518d748e127e67e83aa3bdc962e8b2a36860a8))
* **frontend:** various small fixes ([e8e2efd](https://github.com/windmill-labs/windmill/commit/e8e2efd9bc0f4b3c3237020f0c2ef96d7918cfa2))
* **frontend:** various small fixes ([cb5db64](https://github.com/windmill-labs/windmill/commit/cb5db64320d76f0284a2e03c05bc887ad0063af4))
* **frontend:** various small fixes ([d394edf](https://github.com/windmill-labs/windmill/commit/d394edf44f2aeffd2468afa8f24e00bae3e17a7c))
* **frontend:** workers as the last menu link ([c0a55bf](https://github.com/windmill-labs/windmill/commit/c0a55bfdd4e287d0b736ea2a6c19b6ccfba19fa1))
* **front:** Fix wording issues ([#633](https://github.com/windmill-labs/windmill/issues/633)) ([77ef514](https://github.com/windmill-labs/windmill/commit/77ef514029841eb967376b6472c78d33a2cca55c))
* **go:** inner_main is in a separate file rather than wrapped ([eabd835](https://github.com/windmill-labs/windmill/commit/eabd83580758121149b629285d8f4cb228c9a7ea))
* **go:** make lines align with appended code ([945a750](https://github.com/windmill-labs/windmill/commit/945a750c6b4a2d8d01793ba50e67a4a666041c96))
* iterator input transform is made more generic ([#524](https://github.com/windmill-labs/windmill/issues/524)) ([110a25f](https://github.com/windmill-labs/windmill/commit/110a25f6f860f83bfcf32121fc80488bc6c05d60))
* last ping is set when the job is started avoiding erronous restart ([1bc1217](https://github.com/windmill-labs/windmill/commit/1bc12179c7a8c3f56016716e45320ceaf2e338e6))
* prop picker values correspond to test values ([#628](https://github.com/windmill-labs/windmill/issues/628)) ([4e791b0](https://github.com/windmill-labs/windmill/commit/4e791b039d4f8752af8d40870a6922306be03207))

## [1.35.0](https://github.com/windmill-labs/windmill/compare/v1.34.0...v1.35.0) (2022-09-02)


### Features

* clean openflow spec v1 ([#491](https://github.com/windmill-labs/windmill/issues/491)) ([cf7209b](https://github.com/windmill-labs/windmill/commit/cf7209bdb92bc4f029224640ccdc5213e2c3cb98))
* **frontend:** Add runs to landing page + fix responsive issues ([#487](https://github.com/windmill-labs/windmill/issues/487)) ([9b8f263](https://github.com/windmill-labs/windmill/commit/9b8f263319599b00d7af6350127dabceaccad37e))
* **frontend:** App landing page ([#486](https://github.com/windmill-labs/windmill/issues/486)) ([5954789](https://github.com/windmill-labs/windmill/commit/5954789abb2749488bf0055e98d2b77d0b885056))
* **frontend:** Menu + Tab components ([#517](https://github.com/windmill-labs/windmill/issues/517)) ([6bb80b8](https://github.com/windmill-labs/windmill/commit/6bb80b803d0fa43d40d9add30c12ec5d11cd8230))
* **frontend:** Script editor ([#518](https://github.com/windmill-labs/windmill/issues/518)) ([a2265f7](https://github.com/windmill-labs/windmill/commit/a2265f7f41bb82be7e98c216ad5b73ced29959b2))
* pass bearerToken as queryArg ([3527716](https://github.com/windmill-labs/windmill/commit/35277160a6a5ff400e3a91a98fe97978a6007146))


### Bug Fixes

* **front:** Display all the logs ([#478](https://github.com/windmill-labs/windmill/issues/478)) ([ab994e6](https://github.com/windmill-labs/windmill/commit/ab994e6d42e3bd24307f4c536862f86e966995db))
* **front:** Display all the logs ([#479](https://github.com/windmill-labs/windmill/issues/479)) ([8a585c0](https://github.com/windmill-labs/windmill/commit/8a585c084a9c2bf49c39db848075e62a047f4a81))
* **frontend:** Make sure the schema is infered when the component is mounted ([#520](https://github.com/windmill-labs/windmill/issues/520)) ([0deb31e](https://github.com/windmill-labs/windmill/commit/0deb31e6b6c6b72e73f97654bbdcd40f1a708878))
* **front:** Fix display ([#481](https://github.com/windmill-labs/windmill/issues/481)) ([538dc8f](https://github.com/windmill-labs/windmill/commit/538dc8f4c2aa4b58f0e26ba3d62744bfd77e188a))
* **front:** Fix inline preview ([#476](https://github.com/windmill-labs/windmill/issues/476)) ([cbe9676](https://github.com/windmill-labs/windmill/commit/cbe9676a1f8682b9b22337b54b42b03eff0e313d))
* **front:** Fix not found error + add timeout ([d8bb9dc](https://github.com/windmill-labs/windmill/commit/d8bb9dccffabe63836abe512041804ea827290e4))
* **front:** Fix not found error + add timeout ([#480](https://github.com/windmill-labs/windmill/issues/480)) ([96e42dd](https://github.com/windmill-labs/windmill/commit/96e42dd0fd1b69e48c356dc67dd5b73625a9d0b5))
* **front:** Fix scroll ([#475](https://github.com/windmill-labs/windmill/issues/475)) ([34dd4be](https://github.com/windmill-labs/windmill/commit/34dd4bef12a7094adc4c9163dd02f74ac02c3f17))
* **front:** Set run button state to done when all jobs are loaded ([#482](https://github.com/windmill-labs/windmill/issues/482)) ([4c1cb1d](https://github.com/windmill-labs/windmill/commit/4c1cb1d379819ec3c571e8e5ca6b4a6df7c399e4))
* **front:** Simplfiy how the job's results are read ([#483](https://github.com/windmill-labs/windmill/issues/483)) ([0ec77f2](https://github.com/windmill-labs/windmill/commit/0ec77f2e6f469c1daefa16b24dfeaec1b45a8389))
* remove duplicate path ([#473](https://github.com/windmill-labs/windmill/issues/473)) ([bd98cad](https://github.com/windmill-labs/windmill/commit/bd98cad5c708eb0bed16c666c538275984863e12))

## [1.34.0](https://github.com/windmill-labs/windmill/compare/v1.33.0...v1.34.0) (2022-08-21)


### Features

* implicit types infered from default parameters ([b9dfbfa](https://github.com/windmill-labs/windmill/commit/b9dfbfa2d8d86f0313d4f8b1829c27a1b1c1c380))

## [1.33.0](https://github.com/windmill-labs/windmill/compare/v1.32.0...v1.33.0) (2022-08-21)


### Features

* PostgreSQL parametrized statement handled as typescript template ([1aa28c5](https://github.com/windmill-labs/windmill/commit/1aa28c55990b27901c698eea6812a51eaafc97bb))

## [1.32.0](https://github.com/windmill-labs/windmill/compare/v1.31.0...v1.32.0) (2022-08-21)


### Features

* **backend:** failure_module ([#452](https://github.com/windmill-labs/windmill/issues/452)) ([32d067f](https://github.com/windmill-labs/windmill/commit/32d067f8c078fd7940c2c4bab8dbb01de876503e))
* **frontend:** Open/Close UI ([#445](https://github.com/windmill-labs/windmill/issues/445)) ([7e4aac9](https://github.com/windmill-labs/windmill/commit/7e4aac997175bf2ba479021742e5aa8abab4ff41))
* private imports ([a5343fa](https://github.com/windmill-labs/windmill/commit/a5343fa959a237120fc22d6a3c06da3b29a3f990))
* rely on PG time rather than worker time ([0057266](https://github.com/windmill-labs/windmill/commit/00572668f16183f7508b9966213cbcc9c106da51))


### Bug Fixes

* **backend:** clear_schedule only clear non running jobs ([0cd814c](https://github.com/windmill-labs/windmill/commit/0cd814cfec3ab088f7646b6b9f6970e48961e710))
* **backend:** fixes forloop with 257 items only iterates once ([#446](https://github.com/windmill-labs/windmill/issues/446)) ([bae8573](https://github.com/windmill-labs/windmill/commit/bae85732ff7c70796c2defcd0430d64dedeb36f7))
* **backend:** started_at info for completed_job is no more completed_at ([77a6851](https://github.com/windmill-labs/windmill/commit/77a685144ddc65c8e5205688ce7e411a14f7915b))
* cancel a flow now does the expected behavior ([c0e9cd0](https://github.com/windmill-labs/windmill/commit/c0e9cd05641d28336cc26eee5167a397149d61f2))
* **deno-client:** pg module now supports prepared statements ([5900a03](https://github.com/windmill-labs/windmill/commit/5900a03c045861732bbf6f7bff1280f3c94b86ce))
* **deno-client:** wrap the deno-postgres client and not the query statement ([68aaf32](https://github.com/windmill-labs/windmill/commit/68aaf3267ce183e366696ebadc644580976ed7ce))
* **frontend:** Fix loops pickable properties ([#441](https://github.com/windmill-labs/windmill/issues/441)) ([0681472](https://github.com/windmill-labs/windmill/commit/068147251c831d3ab8564ccb909ad72ef2e32e74))
* **frontend:** input checks refresh when schema change ([15f7cad](https://github.com/windmill-labs/windmill/commit/15f7cadc3d179993b70e1f7584d532528aaabb52))
* **frontend:** link to schedule in runs discriminate isFlows ([7d76e69](https://github.com/windmill-labs/windmill/commit/7d76e69be9753cc572ce7c085d0191a31471d9e9))
* **frontend:** simplify flow preview  logic([#450](https://github.com/windmill-labs/windmill/issues/450)) ([bc5a568](https://github.com/windmill-labs/windmill/commit/bc5a5688ce9c351ad745be225c11a977c1ad2afb))
* handle 0 length for-loops in the backend ([#440](https://github.com/windmill-labs/windmill/issues/440)) ([561e13e](https://github.com/windmill-labs/windmill/commit/561e13e51ee7ffcf20bc524c22d756ea582d546e))
* restart zombie jobs was restarting all jobs ([da77d04](https://github.com/windmill-labs/windmill/commit/da77d040942c01b0011e76546dddd6aaa7786b8f))

## [1.31.0](https://github.com/windmill-labs/windmill/compare/v1.30.0...v1.31.0) (2022-08-17)


### Features

* allow to configure port via envar ([#407](https://github.com/windmill-labs/windmill/issues/407)) ([34be056](https://github.com/windmill-labs/windmill/commit/34be0564f89f942478c25e77fd77a515367a6afd))
* db users: admin -> windmill_admin, app -> windmill_user ([#404](https://github.com/windmill-labs/windmill/issues/404)) ([1c40f01](https://github.com/windmill-labs/windmill/commit/1c40f01e5d8e3d854de4c30d9f5e4f731c220ce2))
* **frontend:** Redesign of the Flow Editor + Arbitrary forloop ([127b0b4](https://github.com/windmill-labs/windmill/commit/127b0b4e5e6a96f91d7e8234cc52d887afb637b0))


### Bug Fixes

* **backend:** collecting result when for loop is not the last step [#422](https://github.com/windmill-labs/windmill/issues/422)  ([e606118](https://github.com/windmill-labs/windmill/commit/e6061189438fb3a7e630d2e390075fc3eded984c))
* **self-hosting:** add lsp and caddy to docke-compose ([#432](https://github.com/windmill-labs/windmill/issues/432)) ([1004518](https://github.com/windmill-labs/windmill/commit/100451878c26d2fa324c6195838accae959a5310))
* set secure only for https ([1275f5f](https://github.com/windmill-labs/windmill/commit/1275f5f7fb65e32a17d7d397d43d0b49ecd5cd0e))
* users privileges ([2bdb617](https://github.com/windmill-labs/windmill/commit/2bdb617b1f80104bd3314656603dccb0021e05cb))

## [1.30.0](https://github.com/windmill-labs/windmill/compare/v1.29.0...v1.30.0) (2022-08-13)


### Features

* add literal object type support ([#401](https://github.com/windmill-labs/windmill/issues/401)) ([845de82](https://github.com/windmill-labs/windmill/commit/845de8206214ed265aef895f0d13636e6e0e26ce))
* support union type will null | undefined ([#400](https://github.com/windmill-labs/windmill/issues/400)) ([0384727](https://github.com/windmill-labs/windmill/commit/0384727a56347aa01a5fee06c82bd49eab2522fa))
* support union types ([#398](https://github.com/windmill-labs/windmill/issues/398)) ([e68ea1b](https://github.com/windmill-labs/windmill/commit/e68ea1b8fc4f88e587121387ecac6858d04ebae2))

## [1.29.0](https://github.com/windmill-labs/windmill/compare/v1.28.1...v1.29.0) (2022-08-10)


### Features

* _value, _index => iter.value, iter.index ([07f4a21](https://github.com/windmill-labs/windmill/commit/07f4a217d0c6b46fd3defaa0242d229a60c69463))
* remove res1 wrapping ([e76a981](https://github.com/windmill-labs/windmill/commit/e76a9816ee09e59d5c38bf0c19231bac8347148c))


### Bug Fixes

* do not skip undefined values ([8b68a87](https://github.com/windmill-labs/windmill/commit/8b68a87c523fe13a9f45786ee0fbb57b10efda13))
* **python:** not filled field with default <function_call> now call the default function ([33962c4](https://github.com/windmill-labs/windmill/commit/33962c44660fd20173a0ae14b00a66a985dd4fc7))
* surface new _iterator value ([13b1904](https://github.com/windmill-labs/windmill/commit/13b1904a7ab5a6e7a7c82d2a2806648441759756))
* update logs even if last new log was < 500ms ([c69621f](https://github.com/windmill-labs/windmill/commit/c69621fa7a9a18223e7854f0824bd6fbcabdfe10))

## [1.28.1](https://github.com/windmill-labs/windmill/compare/v1.28.0...v1.28.1) (2022-08-05)


### Bug Fixes

* **frontend:** add toggl connect ([#341](https://github.com/windmill-labs/windmill/issues/341)) ([b94895f](https://github.com/windmill-labs/windmill/commit/b94895f24eb4ba1b67f499a98c6e6e8d9d006b14))
* **frontend:** schedule args in flow ([#343](https://github.com/windmill-labs/windmill/issues/343)) ([350a25c](https://github.com/windmill-labs/windmill/commit/350a25c837b1367fa5568dd1de0196202d632bd0))
* improve flow viewer with retrieving hub script ([80e28db](https://github.com/windmill-labs/windmill/commit/80e28dbba3e77154c0017bd8e74d144e6aae13fb))

## [1.28.0](https://github.com/windmill-labs/windmill/compare/v1.27.2...v1.28.0) (2022-08-04)


### Features

* **frontend:** global flow preview ([#329](https://github.com/windmill-labs/windmill/issues/329)) ([615f69e](https://github.com/windmill-labs/windmill/commit/615f69e935e9c9c0b60edfb6dc2e82aebba623b9))


### Bug Fixes

* **api:** add discord webhook manual instructions ([a9a4b9b](https://github.com/windmill-labs/windmill/commit/a9a4b9b21d7b68a3e46c28ce13986d7a9ebd2cac))
* **backend:** generalize oauth clients to take in extra params ([6332910](https://github.com/windmill-labs/windmill/commit/6332910dd27f78d555f0ab040545e98dedbea89d))
* **backend:** handle better some flow edge-cases ([3bcd542](https://github.com/windmill-labs/windmill/commit/3bcd542130bc0cb45dfb1fa7681dd4b7beb95c7e))
* **backend:** handle better some flow edge-cases ([9885361](https://github.com/windmill-labs/windmill/commit/988536128bd04dab94cc686bc2db547e57894587))
* **backend:** handle better some flow edge-cases ([70de6e3](https://github.com/windmill-labs/windmill/commit/70de6e3972af81aec68b706dca93e16182a584bb))
* **backend:** prometheus histogram for worker job timer ([#312](https://github.com/windmill-labs/windmill/issues/312)) ([4055586](https://github.com/windmill-labs/windmill/commit/40555868e6221620beca85ebafad2da67e56ec08))
* **frontend:** add jpeg support ([0e8552b](https://github.com/windmill-labs/windmill/commit/0e8552ba800f13add6b25a83a765dace8d4369e7))
* **frontend:** loading template pick the language as well ([82c7ddc](https://github.com/windmill-labs/windmill/commit/82c7ddc00e79a1cc5336a0a219f46d705c2c8d88))
* **frontend:** Use the bracket notation when an identifier is not a valid JS expression ([#327](https://github.com/windmill-labs/windmill/issues/327)) ([05324bd](https://github.com/windmill-labs/windmill/commit/05324bd3562f6066cdc12d74c87033325d1c7ef1))
* **oauth2:** remove discord oauth integration ([986e76d](https://github.com/windmill-labs/windmill/commit/986e76dc8729a53d09cd83531d474f9b5fe88f35))

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
