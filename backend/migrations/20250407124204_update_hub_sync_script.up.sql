-- Add up migration script here
-- Add up migration script here
UPDATE script SET content = 'import * as wmill from "windmill-cli@1.481.0"

export async function main() {
  await wmill.hubPull({ workspace: "admins", token: process.env["WM_TOKEN"], baseUrl: globalThis.process.env["BASE_URL"] });
}
', language = 'bun',
lock = '{
  "dependencies": {
    "windmill-cli": "1.481.0"
  }
}
//bun.lock
{
  "lockfileVersion": 1,
  "workspaces": {
    "": {
      "dependencies": {
        "windmill-cli": "1.481.0",
      },
    },
  },
  "packages": {
    "@ayonli/jsext": ["@ayonli/jsext@1.6.0", "", { "dependencies": { "iconv-lite": "^0.6.3", "sudo-prompt": "^9.2.1", "ws": "^8.17.0", "zod": "^3.23.8" } }, "sha512-dMQuZJIVadEgQ6xp1Q5hRv2JfwANDwnElH6kZWuSjcnAjhvtCoQQ02CdOipfCd4cyrFpaI+8yBboiT9OkRaVyg=="],

    "@deno/shim-deno": ["@deno/shim-deno@0.18.2", "", { "dependencies": { "@deno/shim-deno-test": "^0.5.0", "which": "^4.0.0" } }, "sha512-oQ0CVmOio63wlhwQF75zA4ioolPvOwAoK0yuzcS5bDC1JUvH3y1GS8xPh8EOpcoDQRU4FTG8OQfxhpR+c6DrzA=="],

    "@deno/shim-deno-test": ["@deno/shim-deno-test@0.5.0", "", {}, "sha512-4nMhecpGlPi0cSzT67L+Tm+GOJqvuk8gqHBziqcUQOarnuIax1z96/gJHCSIz2Z0zhxE6Rzwb3IZXPtFh51j+w=="],

    "@esbuild/aix-ppc64": ["@esbuild/aix-ppc64@0.25.2", "", { "os": "aix", "cpu": "ppc64" }, "sha512-wCIboOL2yXZym2cgm6mlA742s9QeJ8DjGVaL39dLN4rRwrOgOyYSnOaFPhKZGLb2ngj4EyfAFjsNJwPXZvseag=="],

    "@esbuild/android-arm": ["@esbuild/android-arm@0.25.2", "", { "os": "android", "cpu": "arm" }, "sha512-NQhH7jFstVY5x8CKbcfa166GoV0EFkaPkCKBQkdPJFvo5u+nGXLEH/ooniLb3QI8Fk58YAx7nsPLozUWfCBOJA=="],

    "@esbuild/android-arm64": ["@esbuild/android-arm64@0.25.2", "", { "os": "android", "cpu": "arm64" }, "sha512-5ZAX5xOmTligeBaeNEPnPaeEuah53Id2tX4c2CVP3JaROTH+j4fnfHCkr1PjXMd78hMst+TlkfKcW/DlTq0i4w=="],

    "@esbuild/android-x64": ["@esbuild/android-x64@0.25.2", "", { "os": "android", "cpu": "x64" }, "sha512-Ffcx+nnma8Sge4jzddPHCZVRvIfQ0kMsUsCMcJRHkGJ1cDmhe4SsrYIjLUKn1xpHZybmOqCWwB0zQvsjdEHtkg=="],

    "@esbuild/darwin-arm64": ["@esbuild/darwin-arm64@0.25.2", "", { "os": "darwin", "cpu": "arm64" }, "sha512-MpM6LUVTXAzOvN4KbjzU/q5smzryuoNjlriAIx+06RpecwCkL9JpenNzpKd2YMzLJFOdPqBpuub6eVRP5IgiSA=="],

    "@esbuild/darwin-x64": ["@esbuild/darwin-x64@0.25.2", "", { "os": "darwin", "cpu": "x64" }, "sha512-5eRPrTX7wFyuWe8FqEFPG2cU0+butQQVNcT4sVipqjLYQjjh8a8+vUTfgBKM88ObB85ahsnTwF7PSIt6PG+QkA=="],

    "@esbuild/freebsd-arm64": ["@esbuild/freebsd-arm64@0.25.2", "", { "os": "freebsd", "cpu": "arm64" }, "sha512-mLwm4vXKiQ2UTSX4+ImyiPdiHjiZhIaE9QvC7sw0tZ6HoNMjYAqQpGyui5VRIi5sGd+uWq940gdCbY3VLvsO1w=="],

    "@esbuild/freebsd-x64": ["@esbuild/freebsd-x64@0.25.2", "", { "os": "freebsd", "cpu": "x64" }, "sha512-6qyyn6TjayJSwGpm8J9QYYGQcRgc90nmfdUb0O7pp1s4lTY+9D0H9O02v5JqGApUyiHOtkz6+1hZNvNtEhbwRQ=="],

    "@esbuild/linux-arm": ["@esbuild/linux-arm@0.25.2", "", { "os": "linux", "cpu": "arm" }, "sha512-UHBRgJcmjJv5oeQF8EpTRZs/1knq6loLxTsjc3nxO9eXAPDLcWW55flrMVc97qFPbmZP31ta1AZVUKQzKTzb0g=="],

    "@esbuild/linux-arm64": ["@esbuild/linux-arm64@0.25.2", "", { "os": "linux", "cpu": "arm64" }, "sha512-gq/sjLsOyMT19I8obBISvhoYiZIAaGF8JpeXu1u8yPv8BE5HlWYobmlsfijFIZ9hIVGYkbdFhEqC0NvM4kNO0g=="],

    "@esbuild/linux-ia32": ["@esbuild/linux-ia32@0.25.2", "", { "os": "linux", "cpu": "ia32" }, "sha512-bBYCv9obgW2cBP+2ZWfjYTU+f5cxRoGGQ5SeDbYdFCAZpYWrfjjfYwvUpP8MlKbP0nwZ5gyOU/0aUzZ5HWPuvQ=="],

    "@esbuild/linux-loong64": ["@esbuild/linux-loong64@0.25.2", "", { "os": "linux", "cpu": "none" }, "sha512-SHNGiKtvnU2dBlM5D8CXRFdd+6etgZ9dXfaPCeJtz+37PIUlixvlIhI23L5khKXs3DIzAn9V8v+qb1TRKrgT5w=="],

    "@esbuild/linux-mips64el": ["@esbuild/linux-mips64el@0.25.2", "", { "os": "linux", "cpu": "none" }, "sha512-hDDRlzE6rPeoj+5fsADqdUZl1OzqDYow4TB4Y/3PlKBD0ph1e6uPHzIQcv2Z65u2K0kpeByIyAjCmjn1hJgG0Q=="],

    "@esbuild/linux-ppc64": ["@esbuild/linux-ppc64@0.25.2", "", { "os": "linux", "cpu": "ppc64" }, "sha512-tsHu2RRSWzipmUi9UBDEzc0nLc4HtpZEI5Ba+Omms5456x5WaNuiG3u7xh5AO6sipnJ9r4cRWQB2tUjPyIkc6g=="],

    "@esbuild/linux-riscv64": ["@esbuild/linux-riscv64@0.25.2", "", { "os": "linux", "cpu": "none" }, "sha512-k4LtpgV7NJQOml/10uPU0s4SAXGnowi5qBSjaLWMojNCUICNu7TshqHLAEbkBdAszL5TabfvQ48kK84hyFzjnw=="],

    "@esbuild/linux-s390x": ["@esbuild/linux-s390x@0.25.2", "", { "os": "linux", "cpu": "s390x" }, "sha512-GRa4IshOdvKY7M/rDpRR3gkiTNp34M0eLTaC1a08gNrh4u488aPhuZOCpkF6+2wl3zAN7L7XIpOFBhnaE3/Q8Q=="],

    "@esbuild/linux-x64": ["@esbuild/linux-x64@0.25.2", "", { "os": "linux", "cpu": "x64" }, "sha512-QInHERlqpTTZ4FRB0fROQWXcYRD64lAoiegezDunLpalZMjcUcld3YzZmVJ2H/Cp0wJRZ8Xtjtj0cEHhYc/uUg=="],

    "@esbuild/netbsd-arm64": ["@esbuild/netbsd-arm64@0.25.2", "", { "os": "none", "cpu": "arm64" }, "sha512-talAIBoY5M8vHc6EeI2WW9d/CkiO9MQJ0IOWX8hrLhxGbro/vBXJvaQXefW2cP0z0nQVTdQ/eNyGFV1GSKrxfw=="],

    "@esbuild/netbsd-x64": ["@esbuild/netbsd-x64@0.25.2", "", { "os": "none", "cpu": "x64" }, "sha512-voZT9Z+tpOxrvfKFyfDYPc4DO4rk06qamv1a/fkuzHpiVBMOhpjK+vBmWM8J1eiB3OLSMFYNaOaBNLXGChf5tg=="],

    "@esbuild/openbsd-arm64": ["@esbuild/openbsd-arm64@0.25.2", "", { "os": "openbsd", "cpu": "arm64" }, "sha512-dcXYOC6NXOqcykeDlwId9kB6OkPUxOEqU+rkrYVqJbK2hagWOMrsTGsMr8+rW02M+d5Op5NNlgMmjzecaRf7Tg=="],

    "@esbuild/openbsd-x64": ["@esbuild/openbsd-x64@0.25.2", "", { "os": "openbsd", "cpu": "x64" }, "sha512-t/TkWwahkH0Tsgoq1Ju7QfgGhArkGLkF1uYz8nQS/PPFlXbP5YgRpqQR3ARRiC2iXoLTWFxc6DJMSK10dVXluw=="],

    "@esbuild/sunos-x64": ["@esbuild/sunos-x64@0.25.2", "", { "os": "sunos", "cpu": "x64" }, "sha512-cfZH1co2+imVdWCjd+D1gf9NjkchVhhdpgb1q5y6Hcv9TP6Zi9ZG/beI3ig8TvwT9lH9dlxLq5MQBBgwuj4xvA=="],

    "@esbuild/win32-arm64": ["@esbuild/win32-arm64@0.25.2", "", { "os": "win32", "cpu": "arm64" }, "sha512-7Loyjh+D/Nx/sOTzV8vfbB3GJuHdOQyrOryFdZvPHLf42Tk9ivBU5Aedi7iyX+x6rbn2Mh68T4qq1SDqJBQO5Q=="],

    "@esbuild/win32-ia32": ["@esbuild/win32-ia32@0.25.2", "", { "os": "win32", "cpu": "ia32" }, "sha512-WRJgsz9un0nqZJ4MfhabxaD9Ft8KioqU3JMinOTvobbX6MOSUigSBlogP8QB3uxpJDsFS6yN+3FDBdqE5lg9kg=="],

    "@esbuild/win32-x64": ["@esbuild/win32-x64@0.25.2", "", { "os": "win32", "cpu": "x64" }, "sha512-kM3HKb16VIXZyIeVrM1ygYmZBKybX8N4p754bw390wGO3Tf2j4L2/WYL+4suWujpgf6GBYs3jv7TyUivdd05JA=="],

    "accepts": ["accepts@2.0.0", "", { "dependencies": { "mime-types": "^3.0.0", "negotiator": "^1.0.0" } }, "sha512-5cvg6CtKwfgdmVqY1WIiXKc3Q1bkRqGLi+2W/6ao+6Y7gu/RCwRuAhGEzh5B4KlszSuTLgZYuqFqo5bImjNKng=="],

    "balanced-match": ["balanced-match@1.0.2", "", {}, "sha512-3oSeUO0TMV67hN1AmbXsK4yaqU7tjiHlbxRDZOpH0KW9+CeX4bRAaX0Anxt0tx2MrpRpWwQaPwIlISEJhYU5Pw=="],

    "body-parser": ["body-parser@2.2.0", "", { "dependencies": { "bytes": "^3.1.2", "content-type": "^1.0.5", "debug": "^4.4.0", "http-errors": "^2.0.0", "iconv-lite": "^0.6.3", "on-finished": "^2.4.1", "qs": "^6.14.0", "raw-body": "^3.0.0", "type-is": "^2.0.0" } }, "sha512-02qvAaxv8tp7fBa/mw1ga98OGm+eCbqzJOKoRt70sLmfEEi+jyBYVTDGfCL/k06/4EMk/z01gCe7HoCH/f2LTg=="],

    "brace-expansion": ["brace-expansion@2.0.1", "", { "dependencies": { "balanced-match": "^1.0.0" } }, "sha512-XnAIvQ8eM+kC6aULx6wuQiwVsnzsi9d3WxzV3FpWTGA19F621kwdbsAcFKXgKUHZWsy+mY6iL1sHTxWEFCytDA=="],

    "bundle-name": ["bundle-name@4.1.0", "", { "dependencies": { "run-applescript": "^7.0.0" } }, "sha512-tjwM5exMg6BGRI+kNmTntNsvdZS1X8BFYS6tnJ2hdH0kVxM6/eVZ2xy+FqStSWvYmtfFMDLIxurorHwDKfDz5Q=="],

    "bytes": ["bytes@3.1.2", "", {}, "sha512-/Nf7TyzTx6S3yRJObOAV7956r8cr2+Oj8AC5dt8wSP3BQAoeX58NoHyCU8P8zGkNXStjTSi6fzO6F0pBdcYbEg=="],

    "call-bind-apply-helpers": ["call-bind-apply-helpers@1.0.2", "", { "dependencies": { "es-errors": "^1.3.0", "function-bind": "^1.1.2" } }, "sha512-Sp1ablJ0ivDkSzjcaJdxEunN5/XvksFJ2sMBFfq6x0ryhQV/2b/KwFe21cMpmHtPOSij8K99/wSfoEuTObmuMQ=="],

    "call-bound": ["call-bound@1.0.4", "", { "dependencies": { "call-bind-apply-helpers": "^1.0.2", "get-intrinsic": "^1.3.0" } }, "sha512-+ys997U96po4Kx/ABpBCqhA9EuxJaQWDQg7295H4hBphv3IZg0boBKuwYpt4YXp6MZ5AmZQnU/tyMTlRpaSejg=="],

    "content-disposition": ["content-disposition@1.0.0", "", { "dependencies": { "safe-buffer": "5.2.1" } }, "sha512-Au9nRL8VNUut/XSzbQA38+M78dzP4D+eqg3gfJHMIHHYa3bg067xj1KxMUWj+VULbiZMowKngFFbKczUrNJ1mg=="],

    "content-type": ["content-type@1.0.5", "", {}, "sha512-nTjqfcBFEipKdXCv4YDQWCfmcLZKm81ldF0pAopTvyrFGVbcR6P/VAAd5G7N+0tTr8QqiU0tFadD6FK4NtJwOA=="],

    "cookie": ["cookie@0.7.2", "", {}, "sha512-yki5XnKuf750l50uGTllt6kKILY4nQ1eNIQatoXEByZ5dWgnKqbnqmTrBE5B4N7lrMJKQ2ytWMiTO2o0v6Ew/w=="],

    "cookie-signature": ["cookie-signature@1.2.2", "", {}, "sha512-D76uU73ulSXrD1UXF4KE2TMxVVwhsnCgfAyTg9k8P6KGZjlXKrOLe4dJQKI3Bxi5wjesZoFXJWElNWBjPZMbhg=="],

    "core-util-is": ["core-util-is@1.0.3", "", {}, "sha512-ZQBvi1DcpJ4GDqanjucZ2Hj3wEO5pZDS89BWbkcrvdxksJorwUDDZamX9ldFkp9aw2lmBDLgkObEA4DWNJ9FYQ=="],

    "debug": ["debug@4.4.0", "", { "dependencies": { "ms": "^2.1.3" } }, "sha512-6WTZ/IxCY/T6BALoZHaE4ctp9xm+Z5kY/pzYaCHRFeyVhojxlrm+46y68HA6hr0TcwEssoxNiDEUJQjfPZ/RYA=="],

    "default-browser": ["default-browser@5.2.1", "", { "dependencies": { "bundle-name": "^4.1.0", "default-browser-id": "^5.0.0" } }, "sha512-WY/3TUME0x3KPYdRRxEJJvXRHV4PyPoUsxtZa78lwItwRQRHhd2U9xOscaT/YTf8uCXIAjeJOFBVEh/7FtD8Xg=="],

    "default-browser-id": ["default-browser-id@5.0.0", "", {}, "sha512-A6p/pu/6fyBcA1TRz/GqWYPViplrftcW2gZC9q79ngNCKAeR/X3gcEdXQHl4KNXV+3wgIJ1CPkJQ3IHM6lcsyA=="],

    "define-lazy-prop": ["define-lazy-prop@3.0.0", "", {}, "sha512-N+MeXYoqr3pOgn8xfyRPREN7gHakLYjhsHhWGT3fWAiL4IkAt0iDw14QiiEm2bE30c5XX5q0FtAA3CK5f9/BUg=="],

    "depd": ["depd@2.0.0", "", {}, "sha512-g7nH6P6dyDioJogAAGprGpCtVImJhpPk/roCzdb3fIh61/s/nPsfR6onyMwkCAR/OlC3yBC0lESvUoQEAssIrw=="],

    "diff": ["diff@7.0.0", "", {}, "sha512-PJWHUb1RFevKCwaFA9RlG5tCd+FO5iRh9A8HEtkmBH2Li03iJriB6m6JIN4rGz3K3JLawI7/veA1xzRKP6ISBw=="],

    "dunder-proto": ["dunder-proto@1.0.1", "", { "dependencies": { "call-bind-apply-helpers": "^1.0.1", "es-errors": "^1.3.0", "gopd": "^1.2.0" } }, "sha512-KIN/nDJBQRcXw0MLVhZE9iQHmG68qAVIBg9CqmUYjmQIhgij9U5MFvrqkUL5FbtyyzZuOeOt0zdeRe4UY7ct+A=="],

    "ee-first": ["ee-first@1.1.1", "", {}, "sha512-WMwm9LhRUo+WUaRN+vRuETqG89IgZphVSNkdFgeb6sS/E4OrDIN7t48CAewSHXc6C8lefD8KKfr5vY61brQlow=="],

    "encodeurl": ["encodeurl@2.0.0", "", {}, "sha512-Q0n9HRi4m6JuGIV1eFlmvJB7ZEVxu93IrMyiMsGC0lrMJMWzRgx6WGquyfQgZVb31vhGgXnfmPNNXmxnOkRBrg=="],

    "es-define-property": ["es-define-property@1.0.1", "", {}, "sha512-e3nRfgfUZ4rNGL232gUgX06QNyyez04KdjFrF+LTRoOXmrOgFKDg4BCdsjW8EnT69eqdYGmRpJwiPVYNrCaW3g=="],

    "es-errors": ["es-errors@1.3.0", "", {}, "sha512-Zf5H2Kxt2xjTvbJvP2ZWLEICxA6j+hAmMzIlypy4xcBg1vKVnx89Wy0GbS+kf5cwCVFFzdCFh2XSCFNULS6csw=="],

    "es-main": ["es-main@1.3.0", "", {}, "sha512-AzORKdz1Zt97TzbYQnIrI3ZiibWpRXUfpo/w0xOJ20GpNYd2bd3MU9m31zS/aJ1TJl6JfLTok83Y8HjNunYT0A=="],

    "es-object-atoms": ["es-object-atoms@1.1.1", "", { "dependencies": { "es-errors": "^1.3.0" } }, "sha512-FGgH2h8zKNim9ljj7dankFPcICIK9Cp5bm+c2gQSYePhpaG5+esrLODihIorn+Pe6FGJzWhXQotPv73jTaldXA=="],

    "esbuild": ["esbuild@0.25.2", "", { "optionalDependencies": { "@esbuild/aix-ppc64": "0.25.2", "@esbuild/android-arm": "0.25.2", "@esbuild/android-arm64": "0.25.2", "@esbuild/android-x64": "0.25.2", "@esbuild/darwin-arm64": "0.25.2", "@esbuild/darwin-x64": "0.25.2", "@esbuild/freebsd-arm64": "0.25.2", "@esbuild/freebsd-x64": "0.25.2", "@esbuild/linux-arm": "0.25.2", "@esbuild/linux-arm64": "0.25.2", "@esbuild/linux-ia32": "0.25.2", "@esbuild/linux-loong64": "0.25.2", "@esbuild/linux-mips64el": "0.25.2", "@esbuild/linux-ppc64": "0.25.2", "@esbuild/linux-riscv64": "0.25.2", "@esbuild/linux-s390x": "0.25.2", "@esbuild/linux-x64": "0.25.2", "@esbuild/netbsd-arm64": "0.25.2", "@esbuild/netbsd-x64": "0.25.2", "@esbuild/openbsd-arm64": "0.25.2", "@esbuild/openbsd-x64": "0.25.2", "@esbuild/sunos-x64": "0.25.2", "@esbuild/win32-arm64": "0.25.2", "@esbuild/win32-ia32": "0.25.2", "@esbuild/win32-x64": "0.25.2" }, "bin": { "esbuild": "bin/esbuild" } }, "sha512-16854zccKPnC+toMywC+uKNeYSv+/eXkevRAfwRD/G9Cleq66m8XFIrigkbvauLLlCfDL45Q2cWegSg53gGBnQ=="],

    "escape-html": ["escape-html@1.0.3", "", {}, "sha512-NiSupZ4OeuGwr68lGIeym/ksIZMJodUGOSCZ/FSnTxcrekbvqrgdUxlJOMpijaKZVjAJrWrGs/6Jy8OMuyj9ow=="],

    "etag": ["etag@1.8.1", "", {}, "sha512-aIL5Fx7mawVa300al2BnEE4iNvo1qETxLrPI/o05L7z6go7fCw1J6EQmbK4FmJ2AS7kgVF/KEZWufBfdClMcPg=="],

    "express": ["express@5.1.0", "", { "dependencies": { "accepts": "^2.0.0", "body-parser": "^2.2.0", "content-disposition": "^1.0.0", "content-type": "^1.0.5", "cookie": "^0.7.1", "cookie-signature": "^1.2.1", "debug": "^4.4.0", "encodeurl": "^2.0.0", "escape-html": "^1.0.3", "etag": "^1.8.1", "finalhandler": "^2.1.0", "fresh": "^2.0.0", "http-errors": "^2.0.0", "merge-descriptors": "^2.0.0", "mime-types": "^3.0.0", "on-finished": "^2.4.1", "once": "^1.4.0", "parseurl": "^1.3.3", "proxy-addr": "^2.0.7", "qs": "^6.14.0", "range-parser": "^1.2.1", "router": "^2.2.0", "send": "^1.1.0", "serve-static": "^2.2.0", "statuses": "^2.0.1", "type-is": "^2.0.1", "vary": "^1.1.2" } }, "sha512-DT9ck5YIRU+8GYzzU5kT3eHGA5iL+1Zd0EutOmTE9Dtk+Tvuzd23VBU+ec7HPNSTxXYO55gPV/hq4pSBJDjFpA=="],

    "finalhandler": ["finalhandler@2.1.0", "", { "dependencies": { "debug": "^4.4.0", "encodeurl": "^2.0.0", "escape-html": "^1.0.3", "on-finished": "^2.4.1", "parseurl": "^1.3.3", "statuses": "^2.0.1" } }, "sha512-/t88Ty3d5JWQbWYgaOGCCYfXRwV1+be02WqYYlL6h0lEiUAMPM8o8qKGO01YIkOHzka2up08wvgYD0mDiI+q3Q=="],

    "forwarded": ["forwarded@0.2.0", "", {}, "sha512-buRG0fpBtRHSTCOASe6hD258tEubFoRLb4ZNA6NxMVHNw2gOcwHo9wyablzMzOA5z9xA9L1KNjk/Nt6MT9aYow=="],

    "fresh": ["fresh@2.0.0", "", {}, "sha512-Rx/WycZ60HOaqLKAi6cHRKKI7zxWbJ31MhntmtwMoaTeF7XFH9hhBp8vITaMidfljRQ6eYWCKkaTK+ykVJHP2A=="],

    "function-bind": ["function-bind@1.1.2", "", {}, "sha512-7XHNxH7qX9xG5mIwxkhumTox/MIRNcOgDrxWsMt2pAr23WHp6MrRlN7FBSFpCpr+oVO0F744iUgR82nJMfG2SA=="],

    "get-intrinsic": ["get-intrinsic@1.3.0", "", { "dependencies": { "call-bind-apply-helpers": "^1.0.2", "es-define-property": "^1.0.1", "es-errors": "^1.3.0", "es-object-atoms": "^1.1.1", "function-bind": "^1.1.2", "get-proto": "^1.0.1", "gopd": "^1.2.0", "has-symbols": "^1.1.0", "hasown": "^2.0.2", "math-intrinsics": "^1.1.0" } }, "sha512-9fSjSaos/fRIVIp+xSJlE6lfwhES7LNtKaCBIamHsjr2na1BiABJPo0mOjjz8GJDURarmCPGqaiVg5mfjb98CQ=="],

    "get-port": ["get-port@7.1.0", "", {}, "sha512-QB9NKEeDg3xxVwCCwJQ9+xycaz6pBB6iQ76wiWMl1927n0Kir6alPiP+yuiICLLU4jpMe08dXfpebuQppFA2zw=="],

    "get-proto": ["get-proto@1.0.1", "", { "dependencies": { "dunder-proto": "^1.0.1", "es-object-atoms": "^1.0.0" } }, "sha512-sTSfBjoXBp89JvIKIefqw7U2CCebsc74kiY6awiGogKtoSGbgjYE/G/+l9sF3MWFPNc9IcoOC4ODfKHfxFmp0g=="],

    "gopd": ["gopd@1.2.0", "", {}, "sha512-ZUKRh6/kUFoAiTAtTYPZJ3hw9wNxx+BIBOijnlG9PnrJsCcSjs1wyyD6vJpaYtgnzDrKYRSqf3OO6Rfa93xsRg=="],

    "has-symbols": ["has-symbols@1.1.0", "", {}, "sha512-1cDNdwJ2Jaohmb3sg4OmKaMBwuC48sYni5HUw2DvsC8LjGTLK9h+eb1X6RyuOHe4hT0ULCW68iomhjUoKUqlPQ=="],

    "hasown": ["hasown@2.0.2", "", { "dependencies": { "function-bind": "^1.1.2" } }, "sha512-0hJU9SCPvmMzIBdZFqNPXWa6dqh7WdH0cII9y+CyS8rG3nL48Bclra9HmKhVVUHyPWNH5Y7xDwAB7bfgSjkUMQ=="],

    "http-errors": ["http-errors@2.0.0", "", { "dependencies": { "depd": "2.0.0", "inherits": "2.0.4", "setprototypeof": "1.2.0", "statuses": "2.0.1", "toidentifier": "1.0.1" } }, "sha512-FtwrG/euBzaEjYeRqOgly7G0qviiXoJWnvEH2Z1plBdXgbyjv34pHTSb9zoeHMyDy33+DWy5Wt9Wo+TURtOYSQ=="],

    "iconv-lite": ["iconv-lite@0.6.3", "", { "dependencies": { "safer-buffer": ">= 2.1.2 < 3.0.0" } }, "sha512-4fCk79wshMdzMp2rH06qWrJE4iolqLhCUH+OiuIgU++RB0+94NlDL81atO7GX55uUKueo0txHNtvEyI6D7WdMw=="],

    "immediate": ["immediate@3.0.6", "", {}, "sha512-XXOFtyqDjNDAQxVfYxuF7g9Il/IbWmmlQg2MYKOH8ExIT1qg6xc4zyS3HaEEATgs1btfzxq15ciUiY7gjSXRGQ=="],

    "inherits": ["inherits@2.0.4", "", {}, "sha512-k/vGaX4/Yla3WzyMCvTQOXYeIHvqOKtnqBduzTHpzpQZzAskKMhZ2K+EnBiSM9zGSoIFeMpXKxa4dYeZIQqewQ=="],

    "ipaddr.js": ["ipaddr.js@1.9.1", "", {}, "sha512-0KI/607xoxSToH7GjN1FfSbLoU0+btTicjsQSWQlh/hZykN8KpmMf7uYwPW3R+akZ6R/w18ZlXSHBYXiYUPO3g=="],

    "is-docker": ["is-docker@3.0.0", "", { "bin": { "is-docker": "cli.js" } }, "sha512-eljcgEDlEns/7AXFosB5K/2nCM4P7FQPkGc/DWLy5rmFEWvZayGrik1d9/QIY5nJ4f9YsVvBkA6kJpHn9rISdQ=="],

    "is-inside-container": ["is-inside-container@1.0.0", "", { "dependencies": { "is-docker": "^3.0.0" }, "bin": { "is-inside-container": "cli.js" } }, "sha512-KIYLCCJghfHZxqjYBE7rEy0OBuTd5xCHS7tHVgvCLkx7StIoaxwNW3hCALgEUjFfeRk+MG/Qxmp/vtETEF3tRA=="],

    "is-promise": ["is-promise@4.0.0", "", {}, "sha512-hvpoI6korhJMnej285dSg6nu1+e6uxs7zG3BYAm5byqDsgJNWwxzM6z6iZiAgQR4TJ30JmBTOwqZUw3WlyH3AQ=="],

    "is-wsl": ["is-wsl@3.1.0", "", { "dependencies": { "is-inside-container": "^1.0.0" } }, "sha512-UcVfVfaK4Sc4m7X3dUSoHoozQGBEFeDC+zVo06t98xe8CzHSZZBekNXH+tu0NalHolcJ/QAGqS46Hef7QXBIMw=="],

    "isarray": ["isarray@1.0.0", "", {}, "sha512-VLghIWNM6ELQzo7zwmcg0NmTVyWKYjvIeM83yjp0wRDTmUnrM678fQbcKBo6n2CJEF0szoG//ytg+TKla89ALQ=="],

    "isexe": ["isexe@3.1.1", "", {}, "sha512-LpB/54B+/2J5hqQ7imZHfdU31OlgQqx7ZicVlkm9kzg9/w8GKLEcFfJl/t7DCEDueOyBAD6zCCwTO6Fzs0NoEQ=="],

    "jszip": ["jszip@3.7.1", "", { "dependencies": { "lie": "~3.3.0", "pako": "~1.0.2", "readable-stream": "~2.3.6", "set-immediate-shim": "~1.0.1" } }, "sha512-ghL0tz1XG9ZEmRMcEN2vt7xabrDdqHHeykgARpmZ0BiIctWxM47Vt63ZO2dnp4QYt/xJVLLy5Zv1l/xRdh2byg=="],

    "lie": ["lie@3.3.0", "", { "dependencies": { "immediate": "~3.0.5" } }, "sha512-UaiMJzeWRlEujzAuw5LokY1L5ecNQYZKfmyZ9L7wDHb/p5etKaxXhohBcrw0EYby+G/NA52vRSN4N39dxHAIwQ=="],

    "math-intrinsics": ["math-intrinsics@1.1.0", "", {}, "sha512-/IXtbwEk5HTPyEwyKX6hGkYXxM9nbj64B+ilVJnC/R6B0pH5G4V3b0pVbL7DBj4tkhBAppbQUlf6F6Xl9LHu1g=="],

    "media-typer": ["media-typer@1.1.0", "", {}, "sha512-aisnrDP4GNe06UcKFnV5bfMNPBUw4jsLGaWwWfnH3v02GnBuXX2MCVn5RbrWo0j3pczUilYblq7fQ7Nw2t5XKw=="],

    "merge-descriptors": ["merge-descriptors@2.0.0", "", {}, "sha512-Snk314V5ayFLhp3fkUREub6WtjBfPdCPY1Ln8/8munuLuiYhsABgBVWsozAG+MWMbVEvcdcpbi9R7ww22l9Q3g=="],

    "mime-db": ["mime-db@1.54.0", "", {}, "sha512-aU5EJuIN2WDemCcAp2vFBfp/m4EAhWJnUNSSw0ixs7/kXbd6Pg64EmwJkNdFhB8aWt1sH2CTXrLxo/iAGV3oPQ=="],

    "mime-types": ["mime-types@3.0.1", "", { "dependencies": { "mime-db": "^1.54.0" } }, "sha512-xRc4oEhT6eaBpU1XF7AjpOFD+xQmXNB5OVKwp4tqCuBpHLS/ZbBDrc07mYTDqVMg6PfxUjjNp85O6Cd2Z/5HWA=="],

    "minimatch": ["minimatch@10.0.1", "", { "dependencies": { "brace-expansion": "^2.0.1" } }, "sha512-ethXTt3SGGR+95gudmqJ1eNhRO7eGEGIgYA9vnPatK4/etz2MEVDno5GMCibdMTuBMyElzIlgxMna3K94XDIDQ=="],

    "ms": ["ms@2.1.3", "", {}, "sha512-6FlzubTLZG3J2a/NVCAleEhjzq5oxgHyaCU9yYXvcLsvoVaHJq/s5xXI6/XXP6tz7R9xAOtHnSO/tXtF3WRTlA=="],

    "negotiator": ["negotiator@1.0.0", "", {}, "sha512-8Ofs/AUQh8MaEcrlq5xOX0CQ9ypTF5dl78mjlMNfOK08fzpgTHQRQPBxcPlEtIw0yRpws+Zo/3r+5WRby7u3Gg=="],

    "object-inspect": ["object-inspect@1.13.4", "", {}, "sha512-W67iLl4J2EXEGTbfeHCffrjDfitvLANg0UlX3wFUUSTx92KXRFegMHUVgSqE+wvhAbi4WqjGg9czysTV2Epbew=="],

    "on-finished": ["on-finished@2.4.1", "", { "dependencies": { "ee-first": "1.1.1" } }, "sha512-oVlzkg3ENAhCk2zdv7IJwd/QUD4z2RxRwpkcGY8psCVcCYZNq4wYnVWALHM+brtuJjePWiYF/ClmuDr8Ch5+kg=="],

    "once": ["once@1.4.0", "", { "dependencies": { "wrappy": "1" } }, "sha512-lNaJgI+2Q5URQBkccEKHTQOPaXdUxnZZElQTZY0MFUAuaEqe1E+Nyvgdz/aIyNi6Z9MzO5dv1H8n58/GELp3+w=="],

    "open": ["open@10.1.0", "", { "dependencies": { "default-browser": "^5.2.1", "define-lazy-prop": "^3.0.0", "is-inside-container": "^1.0.0", "is-wsl": "^3.1.0" } }, "sha512-mnkeQ1qP5Ue2wd+aivTD3NHd/lZ96Lu0jgf0pwktLPtx6cTZiH7tyeGRRHs0zX0rbrahXPnXlUnbeXyaBBuIaw=="],

    "pako": ["pako@1.0.11", "", {}, "sha512-4hLB8Py4zZce5s4yd9XzopqwVv/yGNhV1Bl8NTmCq1763HeK2+EwVTv+leGeL13Dnh2wfbqowVPXCIO0z4taYw=="],

    "parseurl": ["parseurl@1.3.3", "", {}, "sha512-CiyeOxFT/JZyN5m0z9PfXw4SCBJ6Sygz1Dpl0wqjlhDEGGBP1GnsUVEL0p63hoG1fcj3fHynXi9NYO4nWOL+qQ=="],

    "path-to-regexp": ["path-to-regexp@8.2.0", "", {}, "sha512-TdrF7fW9Rphjq4RjrW0Kp2AW0Ahwu9sRGTkS6bvDi0SCwZlEZYmcfDbEsTz8RVk0EHIS/Vd1bv3JhG+1xZuAyQ=="],

    "process-nextick-args": ["process-nextick-args@2.0.1", "", {}, "sha512-3ouUOpQhtgrbOa17J7+uxOTpITYWaGP7/AhoR3+A+/1e9skrzelGi/dXzEYyvbxubEF6Wn2ypscTKiKJFFn1ag=="],

    "proxy-addr": ["proxy-addr@2.0.7", "", { "dependencies": { "forwarded": "0.2.0", "ipaddr.js": "1.9.1" } }, "sha512-llQsMLSUDUPT44jdrU/O37qlnifitDP+ZwrmmZcoSKyLKvtZxpyV0n2/bD/N4tBAAZ/gJEdZU7KMraoK1+XYAg=="],

    "qs": ["qs@6.14.0", "", { "dependencies": { "side-channel": "^1.1.0" } }, "sha512-YWWTjgABSKcvs/nWBi9PycY/JiPJqOD4JA6o9Sej2AtvSGarXxKC3OQSk4pAarbdQlKAh5D4FCQkJNkW+GAn3w=="],

    "range-parser": ["range-parser@1.2.1", "", {}, "sha512-Hrgsx+orqoygnmhFbKaHE6c296J+HTAQXoxEF6gNupROmmGJRoyzfG3ccAveqCBrwr/2yxQ5BVd/GTl5agOwSg=="],

    "raw-body": ["raw-body@3.0.0", "", { "dependencies": { "bytes": "3.1.2", "http-errors": "2.0.0", "iconv-lite": "0.6.3", "unpipe": "1.0.0" } }, "sha512-RmkhL8CAyCRPXCE28MMH0z2PNWQBNk2Q09ZdxM9IOOXwxwZbN+qbWaatPkdkWIKL2ZVDImrN/pK5HTRz2PcS4g=="],

    "readable-stream": ["readable-stream@2.3.8", "", { "dependencies": { "core-util-is": "~1.0.0", "inherits": "~2.0.3", "isarray": "~1.0.0", "process-nextick-args": "~2.0.0", "safe-buffer": "~5.1.1", "string_decoder": "~1.1.1", "util-deprecate": "~1.0.1" } }, "sha512-8p0AUk4XODgIewSi0l8Epjs+EVnWiK7NoDIEGU0HhE7+ZyY8D1IMY7odu5lRrFXGg71L15KG8QrPmum45RTtdA=="],

    "router": ["router@2.2.0", "", { "dependencies": { "debug": "^4.4.0", "depd": "^2.0.0", "is-promise": "^4.0.0", "parseurl": "^1.3.3", "path-to-regexp": "^8.0.0" } }, "sha512-nLTrUKm2UyiL7rlhapu/Zl45FwNgkZGaCpZbIHajDYgwlJCOzLSk+cIPAnsEqV955GjILJnKbdQC1nVPz+gAYQ=="],

    "run-applescript": ["run-applescript@7.0.0", "", {}, "sha512-9by4Ij99JUr/MCFBUkDKLWK3G9HVXmabKz9U5MlIAIuvuzkiOicRYs8XJLxX+xahD+mLiiCYDqF9dKAgtzKP1A=="],

    "safe-buffer": ["safe-buffer@5.2.1", "", {}, "sha512-rp3So07KcdmmKbGvgaNxQSJr7bGVSVk5S9Eq1F+ppbRo70+YeaDxkw5Dd8NPN+GD6bjnYm2VuPuCXmpuYvmCXQ=="],

    "safer-buffer": ["safer-buffer@2.1.2", "", {}, "sha512-YZo3K82SD7Riyi0E1EQPojLz7kpepnSQI9IyPbHHg1XXXevb5dJI7tpyN2ADxGcQbHG7vcyRHk0cbwqcQriUtg=="],

    "send": ["send@1.2.0", "", { "dependencies": { "debug": "^4.3.5", "encodeurl": "^2.0.0", "escape-html": "^1.0.3", "etag": "^1.8.1", "fresh": "^2.0.0", "http-errors": "^2.0.0", "mime-types": "^3.0.1", "ms": "^2.1.3", "on-finished": "^2.4.1", "range-parser": "^1.2.1", "statuses": "^2.0.1" } }, "sha512-uaW0WwXKpL9blXE2o0bRhoL2EGXIrZxQ2ZQ4mgcfoBxdFmQold+qWsD2jLrfZ0trjKL6vOw0j//eAwcALFjKSw=="],

    "serve-static": ["serve-static@2.2.0", "", { "dependencies": { "encodeurl": "^2.0.0", "escape-html": "^1.0.3", "parseurl": "^1.3.3", "send": "^1.2.0" } }, "sha512-61g9pCh0Vnh7IutZjtLGGpTA355+OPn2TyDv/6ivP2h/AdAVX9azsoxmg2/M6nZeQZNYBEwIcsne1mJd9oQItQ=="],

    "set-immediate-shim": ["set-immediate-shim@1.0.1", "", {}, "sha512-Li5AOqrZWCVA2n5kryzEmqai6bKSIvpz5oUJHPVj6+dsbD3X1ixtsY5tEnsaNpH3pFAHmG8eIHUrtEtohrg+UQ=="],

    "setprototypeof": ["setprototypeof@1.2.0", "", {}, "sha512-E5LDX7Wrp85Kil5bhZv46j8jOeboKq5JMmYM3gVGdGH8xFpPWXUMsNrlODCrkoxMEeNi/XZIwuRvY4XNwYMJpw=="],

    "side-channel": ["side-channel@1.1.0", "", { "dependencies": { "es-errors": "^1.3.0", "object-inspect": "^1.13.3", "side-channel-list": "^1.0.0", "side-channel-map": "^1.0.1", "side-channel-weakmap": "^1.0.2" } }, "sha512-ZX99e6tRweoUXqR+VBrslhda51Nh5MTQwou5tnUDgbtyM0dBgmhEDtWGP/xbKn6hqfPRHujUNwz5fy/wbbhnpw=="],

    "side-channel-list": ["side-channel-list@1.0.0", "", { "dependencies": { "es-errors": "^1.3.0", "object-inspect": "^1.13.3" } }, "sha512-FCLHtRD/gnpCiCHEiJLOwdmFP+wzCmDEkc9y7NsYxeF4u7Btsn1ZuwgwJGxImImHicJArLP4R0yX4c2KCrMrTA=="],

    "side-channel-map": ["side-channel-map@1.0.1", "", { "dependencies": { "call-bound": "^1.0.2", "es-errors": "^1.3.0", "get-intrinsic": "^1.2.5", "object-inspect": "^1.13.3" } }, "sha512-VCjCNfgMsby3tTdo02nbjtM/ewra6jPHmpThenkTYh8pG9ucZ/1P8So4u4FGBek/BjpOVsDCMoLA/iuBKIFXRA=="],

    "side-channel-weakmap": ["side-channel-weakmap@1.0.2", "", { "dependencies": { "call-bound": "^1.0.2", "es-errors": "^1.3.0", "get-intrinsic": "^1.2.5", "object-inspect": "^1.13.3", "side-channel-map": "^1.0.1" } }, "sha512-WPS/HvHQTYnHisLo9McqBHOJk2FkHO/tlpvldyrnem4aeQp4hai3gythswg6p01oSoTl58rcpiFAjF2br2Ak2A=="],

    "statuses": ["statuses@2.0.1", "", {}, "sha512-RwNA9Z/7PrK06rYLIzFMlaF+l73iwpzsqRIFgbMLbTcLD6cOao82TaWefPXQvB2fOC4AjuYSEndS7N/mTCbkdQ=="],

    "string_decoder": ["string_decoder@1.1.1", "", { "dependencies": { "safe-buffer": "~5.1.0" } }, "sha512-n/ShnvDi6FHbbVfviro+WojiFzv+s8MPMHBczVePfUpDJLwoLT0ht1l4YwBCbi8pJAveEEdnkHyPyTP/mzRfwg=="],

    "sudo-prompt": ["sudo-prompt@9.2.1", "", {}, "sha512-Mu7R0g4ig9TUuGSxJavny5Rv0egCEtpZRNMrZaYS1vxkiIxGiGUwoezU3LazIQ+KE04hTrTfNPgxU5gzi7F5Pw=="],

    "toidentifier": ["toidentifier@1.0.1", "", {}, "sha512-o5sSPKEkg/DIQNmH43V0/uerLrpzVedkUh8tGNvaeXpfpuwjKenlSox/2O/BTlZUtEe+JG7s5YhEz608PlAHRA=="],

    "type-is": ["type-is@2.0.1", "", { "dependencies": { "content-type": "^1.0.5", "media-typer": "^1.1.0", "mime-types": "^3.0.0" } }, "sha512-OZs6gsjF4vMp32qrCbiVSkrFmXtG/AZhY3t0iAMrMBiAZyV9oALtXO8hsrHbMXF9x6L3grlFuwW2oAz7cav+Gw=="],

    "unpipe": ["unpipe@1.0.0", "", {}, "sha512-pjy2bYhSsufwWlKwPc+l3cN7+wuJlK6uz0YdJEOlQDbl6jo/YlPi4mb8agUkVC8BF7V8NuzeyPNqRksA3hztKQ=="],

    "util-deprecate": ["util-deprecate@1.0.2", "", {}, "sha512-EPD5q1uXyFxJpCrLnCc1nHnq3gOa6DZBocAIiI2TaSCA7VCJ1UJDMagCzIkXNsUYfD1daK//LTEQ8xiIbrHtcw=="],

    "vary": ["vary@1.1.2", "", {}, "sha512-BNGbWLfd0eUPabhkXUVm0j8uuvREyTh5ovRa/dyow/BqAbZJyC+5fU+IzQOzmAKzYqYRAISoRhdQr3eIZ/PXqg=="],

    "which": ["which@4.0.0", "", { "dependencies": { "isexe": "^3.1.1" }, "bin": { "node-which": "bin/which.js" } }, "sha512-GlaYyEb07DPxYCKhKzplCWBJtvxZcZMrL+4UkrTSJHHPyZU4mYYTv3qaOe77H7EODLSSopAUFAc6W8U4yqvscg=="],

    "windmill-cli": ["windmill-cli@1.481.0", "", { "dependencies": { "@ayonli/jsext": "*", "@deno/shim-deno": "~0.18.0", "diff": "*", "es-main": "*", "esbuild": "*", "express": "*", "get-port": "7.1.0", "jszip": "3.7.1", "minimatch": "*", "open": "*", "ws": "*" }, "bin": { "wmill": "esm/main.js" } }, "sha512-nIIrt+/+TqeyHlgcDnPMTBH3CZX4TMVwxy2UFD+5lOI5OY9JOtCpWk5UPn6BOs6fw1DG0MIBJG7AhCuPelfSiQ=="],

    "wrappy": ["wrappy@1.0.2", "", {}, "sha512-l4Sp/DRseor9wL6EvV2+TuQn63dMkPjZ/sp9XkghTEbV9KlPS1xUsZ3u7/IQO4wxtcFB4bgpQPRcR3QCvezPcQ=="],

    "ws": ["ws@8.18.1", "", { "peerDependencies": { "bufferutil": "^4.0.1", "utf-8-validate": ">=5.0.2" }, "optionalPeers": ["bufferutil", "utf-8-validate"] }, "sha512-RKW2aJZMXeMxVpnZ6bck+RswznaxmzdULiBr6KY7XkTnW8uvt0iT9H5DkHUChXrc+uurzwa0rVI16n/Xzjdz1w=="],

    "zod": ["zod@3.24.2", "", {}, "sha512-lY7CDW43ECgW9u1TcT3IoXHflywfVqDYze4waEz812jR/bZ8FHDsl7pFQoSZTz5N+2NqRXs8GBwnAwo3ZNxqhQ=="],

    "readable-stream/safe-buffer": ["safe-buffer@5.1.2", "", {}, "sha512-Gd2UZBJDkXlY7GbJxfsE8/nvKkUEU1G38c1siN6QP6a9PT9MmHB8GnpscSmMJSoF8LOIrt8ud/wPtojys4G6+g=="],

    "string_decoder/safe-buffer": ["safe-buffer@5.1.2", "", {}, "sha512-Gd2UZBJDkXlY7GbJxfsE8/nvKkUEU1G38c1siN6QP6a9PT9MmHB8GnpscSmMJSoF8LOIrt8ud/wPtojys4G6+g=="],
  }
}'
WHERE hash = -28028598712388162 AND workspace_id = 'admins';