# @aws-sdk/signature-v4-multi-region

[![NPM version](https://img.shields.io/npm/v/@aws-sdk/signature-v4-multi-region/latest.svg)](https://www.npmjs.com/package/@aws-sdk/signature-v4-multi-region)
[![NPM downloads](https://img.shields.io/npm/dm/@aws-sdk/signature-v4-multi-region.svg)](https://www.npmjs.com/package/@aws-sdk/signature-v4-multi-region)

See also https://github.com/aws/aws-sdk-js-v3/tree/main#functionality-requiring-aws-common-runtime-crt.
## Usage

This package contains optional dependency [`@aws-sdk/signature-v4-crt`](https://www.npmjs.com/package/@aws-sdk/signature-v4).
You need to install this package explicitly to sign an un-regional request using SigV4a algorithm. The package contains
Node.js native implementation which requires building at installation. The installed package MAY NOT work if the
instance building the package runs a different operating system than the instance running the application.

The `@aws-sdk/signature-v4-crt` is only supported in Node.js currently because it depends on a native dependency.

Please refer to [this issue](https://github.com/aws/aws-sdk-js-v3/issues/2822) for more information.

## Description

This package provides a SigV4-compatible request signer that wraps a pure-JS SigV4 signer
([`@aws-sdk/signature-v4`](https://www.npmjs.com/package/@aws-sdk/signature-v4)) for regional requests, and attempts to
call a native implementation of SigV4a signer([`@aws-sdk/signature-v4-crt`](https://www.npmjs.com/package/@aws-sdk/signature-v4))
it the request is not regional.

An un-regional request is identified by the `signingRegion` parameter. A region is un-regional if the `signingRegion`
parameter is set to `*`.
