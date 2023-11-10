# Changelog


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
