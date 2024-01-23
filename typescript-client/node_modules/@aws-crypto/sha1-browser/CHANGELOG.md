# Change Log

All notable changes to this project will be documented in this file.
See [Conventional Commits](https://conventionalcommits.org) for commit guidelines.

# [3.0.0](https://github.com/aws/aws-sdk-js-crypto-helpers/compare/v2.0.2...v3.0.0) (2023-01-12)

- feat!: replace Hash implementations with Checksum interface (#492) ([da43dc0](https://github.com/aws/aws-sdk-js-crypto-helpers/commit/da43dc0fdf669d9ebb5bfb1b1f7c79e46c4aaae1)), closes [#492](https://github.com/aws/aws-sdk-js-crypto-helpers/issues/492)

### BREAKING CHANGES

- All classes that implemented `Hash` now implement `Checksum`.

## [2.0.2](https://github.com/aws/aws-sdk-js-crypto-helpers/compare/v2.0.1...v2.0.2) (2022-09-07)

### Bug Fixes

- **#337:** update @aws-sdk/types ([#373](https://github.com/aws/aws-sdk-js-crypto-helpers/issues/373)) ([b26a811](https://github.com/aws/aws-sdk-js-crypto-helpers/commit/b26a811a392f5209c7ec7e57251500d4d78f97ff)), closes [#337](https://github.com/aws/aws-sdk-js-crypto-helpers/issues/337)

# [2.0.0](https://github.com/aws/aws-sdk-js-crypto-helpers/compare/v1.2.2...v2.0.0) (2021-10-25)

**Note:** Version bump only for package @aws-crypto/sha1-browser

# [1.2.0](https://github.com/aws/aws-sdk-js-crypto-helpers/compare/v1.1.1...v1.2.0) (2021-09-17)

### Bug Fixes

- Adding ie11-detection dependency to sha1-browser ([#213](https://github.com/aws/aws-sdk-js-crypto-helpers/issues/213)) ([138750d](https://github.com/aws/aws-sdk-js-crypto-helpers/commit/138750d96385b8cc479b6f54c500ee1b5380648c))

### Features

- Add SHA1 ([#208](https://github.com/aws/aws-sdk-js-crypto-helpers/issues/208)) ([45c50ff](https://github.com/aws/aws-sdk-js-crypto-helpers/commit/45c50ffa3acc9e3bf4039ab59a0102e4d40455ec))
