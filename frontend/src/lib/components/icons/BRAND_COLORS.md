# Icon brand colours

Where every icon's colours come from, and whether they survive both app surfaces.
Compiled from the components themselves during the audit — colours read from the fills and
the Tailwind pair classes, sources from each component's provenance comment, contrast
computed from those hexes against `surface-primary` in each theme. Maintained by hand from
here on: change an icon's colour or source and change its row.

Surfaces: light `#fbfbfd`, dark `#2e3441`. Ratios are WCAG non-text contrast; **bold** marks a
mark that is effectively invisible on that surface. WCAG exempts logotypes from the 3:1
floor, so a low ratio is a signal the colour may be wrong, not automatically a defect.

`pair` = brand publishes a per-theme variant. Usually applied as `text-[#light] dark:text-[#dark]`; `AnsibleIcon` inverts instead (`dark:invert`), and `DatadogIcon`, `DenoIcon`, `DeepLIcon` and `TogglIcon` swap between two SVGs (`dark:hidden` / `hidden dark:block`) because their two marks are different artwork, not the same shape recoloured.
`fixed` = full-colour mark, same in both themes. `inherits` = brand publishes no colour,
so the mark takes the surrounding text colour. `mixed` = the root carries a
`fill="currentColor"` that hardcoded path fills override, so it is inert — these are
candidates for cleanup, not theme-aware icons.

The Light/Dark columns show the colour that carries the mark; white and black knockout
details are omitted. Ratios are the best contrast any part of the mark achieves.

**Do not change a colour here without a first-party source.** Several of these look like
mistakes and are not: Cal.com is deliberately greyscale, Google Cloud may not be recoloured,
Stripe is blurple rather than black. Third-party icon sets go stale and have been wrong
repeatedly — check the brand's own page.

| Icon | Resource types | Mode | Light | Dark | ☀ | 🌙 | Source |
|---|---|---|---|---|---|---|---|
| `AblyIcon` | `ably` | fixed | #FF5416 | #FF5416 | 3.87 | 3.88 | brand.ably.com/logo |
| `AbstractApiIcon` | `abstractapi` | fixed | #20E492 | #20E492 | **1.62** | 12.47 | abstractapi.com's own logo SVG (6538df34291c9fa4ed28d6f7_Logo.svg) |
| `AcceloIcon` | `accelo` | fixed | #4C49CB | #4C49CB | 6.51 | 8.15 | Accelo_Logo-Primary.svg on accelo.com |
| `ActiveCampaignIcon` | `activecampaign` | pair | #004CFF | #FFFFFF | 5.84 | 12.47 | activecampaign.com/brand logo pack (ActiveCampaign-Glyph-Blue.svg / ActiveCampaign-Glyph-White.svg) |
| `ActivitypubIcon` | `activitypub` | fixed | #F1007E | #F1007E | 5.01 | 2.99 | activitypub.rocks/static/images/ActivityPub-logo.svg |
| `AcumbamailIcon` | `acumbamail` | fixed | #E62F71 | #E62F71 | 8.83 | 8.86 | Acumbamail's own isotype SVG, /static/favico/Acumbamail/favicon-32.svg on acumbamail.com |
| `AdhookIcon` | `adhook` | fixed | #00ACC6 | #00ACC6 | 2.63 | 4.58 | adhook's own logo (https://adhook.io/fr/images/logo.svg, `.cls-1{fill:#00acc6}`) |
| `AdobeAcrobatSignIcon` | `adobe_acrobat_sign` | fixed | #584CCC | #584CCC | 6.12 | 12.47 | Adobe's own Acrobat Sign product icon (adobe.com/cc-shared/assets/img/product-icons/svg/acrobat-sign.svg); same value in the live app favicon |
| `Ai21Icon` | `ai21` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | ai21.com (ai21-logo-black.svg / ai21-logo-white.svg) |
| `AirtableIcon` | `airtable` | mixed | #FCB400 | #FCB400 | 3.88 | 6.93 | airtable.com/favicon.ico (fixed full-colour mark: #18BFFF and #F82B60 panels) |
| `AlgoliaIcon` | `algolia` | pair | #003DFF | #FFFFFF | 6.53 | 12.47 | algolia.com logo pack (Algolia-mark-blue.svg / Algolia-mark-white.svg) |
| `AmqpIcon` | `amqp` | fixed | — | — | — | — | — |
| `AnsibleIcon` | `ansible` | pair | #1A1918 | #E5E6E7 | 16.99 | 9.98 | ansible/logos community-marks (Black and White variants, CC BY-SA 4.0) |
| `AnthropicIcon` | `anthropic` | pair | #141413 | #FAF9F5 | 17.84 | 11.84 | anthropics/skills |
| `ApifyIcon` | `apify` | fixed | #246DFF | #246DFF | 4.32 | 12.47 | apify.com/resources/brand |
| `ApolloIcon` | `apollo` | pair | #1F1F1E | #F8FF2C | 15.96 | 11.48 | apollo.io |
| `AppwriteIcon` | `appwrite` | mixed | #FD366E | #FD366E | 3.88 | 5.71 | https://appwrite.io/assets |
| `ArcGisIcon` | `arcgis_account` | fixed | #006FDE | #006FDE | 4.69 | 2.57 | Esri's ArcGIS Pro product logo (esri.com/content/dam/esrisites/en-us/common/icons/product-logos/arcgis-pro-64.svg) |
| `AsanaIcon` | `asana` | fixed | #FF584A | #FF584A | 3.01 | 4.01 | asana.com/brand |
| `AssemblyAiIcon` | `assemblyai` | pair | #1D1B16 | #C7C3B2 | 16.65 | 12.47 | assemblyai.com (assemblyai-logo-full-primary.svg / assemblyai-logo-full-secondary.svg) |
| `AttioIcon` | `attio` | pair | #1C1D1F | #FFFFFF | 16.32 | 12.47 | the attio.com header logo (--color-black-100 / --color-white-100) |
| `Auth0Icon` | `auth0` | pair | #232220 | #FFFFFF | 15.38 | 12.47 | auth0.com docs logo light.svg / dark.svg |
| `AutheliaIcon` | `authelia` | fixed | #3F51B4 | #3F51B4 | 6.67 | 1.81 | authelia.com/images/branding/logo-cropped.svg (light stop of the official #3F51B4→#113155 gradient, flattened) |
| `AuthentikIcon` | `authentik` | pair | #FD4B2D | #FFFFFF | 3.27 | 12.47 | goauthentik.io/press |
| `AwsEcrIcon` | `aws_ecr` | fixed | #ED7100 | #ED7100 | 2.92 | 12.47 | the AWS Architecture Icons package (Icon-package_07312026, Arch_Containers/Arch_Amazon-Elastic-Container-Registry) |
| `AwsIcon` | `aws`, `redshift` | pair | #252F3E | #FF9900 | 13.07 | 5.83 | AWS's own logo files (d0.awsstatic.com/logos/powered-by-aws{,-white}.png) |
| `AzureIcon` | `azure` | fixed | — | — | — | — | Microsoft's own logo_azure.svg (learn.microsoft.com/media/logos/logo_azure.svg), whose outer wedges add the #114A8B->#0669BC and #3CCBF4->#2892DF gradients |
| `BambooHrIcon` | `bamboo_hr` | pair | #599D15 | #FFFFFF | 3.25 | 12.47 | bamboohr.com (Encore --brandColor; bamboohr-logo-white.png is the published reversed variant) |
| `BaremetricsIcon` | `baremetrics` | fixed | #5386FF | #5386FF | 3.27 | 3.70 | the mark in baremetrics.com's header logo (baremetrics-logo.svg), the asset this path is taken from |
| `BaserowIcon` | `baserow`, `baserow_table` | fixed | #2BC3F1 | #2BC3F1 | 4.96 | 6.05 | the baserow.io favicon and horizontal logo |
| `BasisTheoryIcon` | `basis_theory` | pair | #1D2032 | #EBEDFF | 15.57 | 10.74 | developers.basistheory.com/img/bt-logo-light.svg and bt-logo-dark.svg, which ship the same mark geometry in the two theme colours |
| `BeamerIcon` | `beamer` | pair | #1C1E21 | #FFFFFF | 16.16 | 12.47 | the getbeamer.com header logo (g#isotype) and their webclip app icon, which sets the same mark in white on #1C1E21 |
| `BigQueryIcon` | `bigquery` | fixed | #34A853 | #34A853 | 3.80 | 7.30 | Google Cloud's official icon library (cloud.google.com/icons, core-products-icons.zip) |
| `BitbucketIcon` | `bitbucket` | pair | #1868DB | #FFFFFF | 5.03 | 12.47 | atlassian.design/foundations/logos (Bitbucket mark, brand and inverse) |
| `BitlyIcon` | `bitly` | fixed | #F36600 | #F36600 | 3.03 | 3.99 | bitly.com/pages/bitly-logo-usage-guidelines-for-media (Bitly-MediaKit glyph_bitly_orange_RGB.svg) |
| `BloggerIcon` | `blogger` | fixed | #F57C00 | #F57C00 | 2.62 | 12.47 | Google's Blogger product logo (gstatic.com/images/branding/productlogos/blogger/v5/192px.svg) |
| `BlueskyIcon` | `bluesky` | pair | #0560FF | #FFFFFF | 4.93 | 12.47 | bsky.social/about/support/branding |
| `BotifyIcon` | `botify` | fixed | #A973FF | #A973FF | 3.09 | 3.91 | botify.com design tokens (--color--surface--purple-05) |
| `BoxIcon` | `box` | pair | #0061D5 | #FFFFFF | 5.54 | 12.47 | box.com (.box-logo-svg fill:#0061d5, reversed to #fff over the dark masthead) |
| `BrevoIcon` | `brevo`, `sendinblue` | fixed | #0B996E | #0B996E | 3.51 | 12.47 | brevo.com's favicon.svg |
| `BrexIcon` | `brex` | pair | #15191E | #FFFFFF | 17.08 | 12.47 | brex.com |
| `BrowserlessIcon` | `browserless` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | browserless.io/favicon.svg |
| `BubbleIcon` | `bubble` | mixed | #0000FF | #0000FF | 8.31 | 5.71 | the logo SVG served on bubble.io/brand; the B is #262626 there, kept as currentColor so the monochrome part follows the app theme |
| `BuildkiteIcon` | `buildkite` | fixed | #30F2A2 | #30F2A2 | 2.04 | 8.52 | buildkite.com/about/brand-assets |
| `BunIcon` | — | fixed | #FBF0DF | #FBF0DF | 6.48 | 12.47 | https://bun.com/logo.svg |
| `ButtondownIcon` | `buttondown` | fixed | #0069FF | #0069FF | 4.55 | 2.65 | https://buttondown.com/brand |
| `CSharpIcon` | — | fixed | #927BE5 | #927BE5 | 7.68 | 12.47 | dotnet/brand logo/language-icons/csharp-72.svg (CC0) |
| `CalcomIcon` | `calcom` | pair | #292929 | #FAFAFA | 14.08 | 11.95 | design.cal.com |
| `CalendlyIcon` | `calendly` | pair | #006BFF | #FFFFFF | 4.47 | 12.47 | Calendly's 2024 External Brand Guidelines and calendly_brand mark_white.svg (media kit on calendly.com/newsroom) |
| `CampaynIcon` | `campayn` | fixed | #008AFF | #008AFF | 3.34 | 12.47 | app.campayn.com/images/campayn/favicons/safari-pinned-tab.svg (colours sampled from android-chrome-512x512.png in the same directory) |
| `CertopusIcon` | `certopus` | fixed | #FF6E30 | #FF6E30 | 12.07 | 12.47 | https://certopus.com/images/logo/logo_circle.svg |
| `ChromaIcon` | `chromadb` | fixed | #FFDE2D | #FFDE2D | 3.65 | 9.35 | Chroma's own logo SVG served by trychroma.com (chroma-wordmark.svg) |
| `CircleCiIcon` | `circleci` | pair | #161616 | #FFFFFF | 17.51 | 12.47 | brand.circleci.com |
| `CiscoIcon` | `cisco` | pair | #00BCEB | #FFFFFF | 2.16 | 12.47 | cisco.com logo SVG and newsroom.cisco.com/logos |
| `ClaudeIcon` | — | fixed | #D97757 | #D97757 | 3.02 | 12.47 | https://claude.ai/favicon.svg (Anthropic's own asset) |
| `ClearbitIcon` | `clearbit` | fixed | #4DB1FD | #4DB1FD | 20.32 | 10.83 | clearbit.com/logo.svg |
| `ClerkIcon` | `clerk` | fixed | #BAB1FF | #BAB1FF | 5.10 | 6.43 | clerk.com/brand-assets (symbol-primary.svg) |
| `ClickhouseIcon` | `clickhouse` | pair | #161616 | #FFFFFF | 17.51 | 12.47 | clickhouse.design/brand/logo-usage (logomark, on-light / on-dark) |
| `ClickupIcon` | `clickup` | fixed | #6647F0 | #6647F0 | 5.46 | 4.12 | clickup.com/brand (v4 Logomark-gradient.svg); the gradient mark is the same on light and dark, and the guidelines say "don't change the color" |
| `CloseIcon` | `close` | fixed | #4EC375 | #4EC375 | 4.77 | 7.39 | close.com/brand (close-logo-2024 mark.svg) |
| `CloudflareIcon` | `cloudflare` | fixed | #FF5F08 | #FF5F08 | 2.95 | 5.83 | the logomark shipped on cloudflare.com, blog.cloudflare.com and workers.cloudflare.com |
| `CloudinaryIcon` | `cloudinary` | pair | #3448C5 | #FFFFFF | 7.06 | 12.47 | cloudinary_logo_for_white_bg.svg and cloudinary_logo_for_black_bg.svg on cloudinary-res.cloudinary.com |
| `CockroachDbIcon` | `cockroachdb` | pair | #6933FF | #FFFFFF | 5.78 | 12.47 | cockroachlabs.com (electric-purple-500, also the CockroachDB docs primaryColor) and the docs light/dark logo pair |
| `CodaIcon` | `coda` | fixed | #F46A54 | #F46A54 | 2.89 | 4.18 | Coda's own app icon, https://cdn.coda.io/icons/png/color/coda-192.png (single-colour mark, no dark variant published) |
| `CodatIcon` | `codat` | fixed | #D1E100 | #D1E100 | 17.31 | 8.60 | codat.io (logo-white.svg glyph outlines, colours from the site palette); framing matches their 300x300 favicon exactly |
| `CohereIcon` | `cohere` | fixed | #355146 | #355146 | 8.41 | 12.47 | https://cohere.com/logo.svg |
| `CoinMarketCapIcon` | `coinmarketcap` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | coinmarketcap.com |
| `CoinbaseIcon` | `coinbase` | pair | #0052FF | — | 5.57 | — | Coinbase's own light/dark logo files (mintcdn.com/coinbase-prod/.../logos/wordmark-light.svg and wordmark-dark.svg, served by docs.cdp.coinbase.com) |
| `ComapeoIcon` | `comapeo_server` | pair | #022199 | #0066FF | 12.09 | 2.58 | the CoMapeo Cloud mark shipped as public/favicon.svg in digidem/comapeo-cloud-app (the server this resource connects to) |
| `ConfluenceIcon` | `confluence` | fixed | #1868DB | #1868DB | 5.03 | 12.47 | Atlassian's @atlaskit/logo (atlassian.design logo library) |
| `ContentfulIcon` | `contentful` | fixed | #1773EB | #1773EB | 4.33 | 9.08 | Contentful's Forma 36 design system (ContentfulLogoIcon) |
| `ContiguityIcon` | `contiguity` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | contiguity.com/assets/icon-white.png and icon-black.png (docs.contiguity.com likewise ships logo/black.svg for light and logo/white.svg for dark) |
| `ConvertKitIcon` | `convertkit` | pair | #1E1E1E | #F2EFE9 | 16.13 | 10.87 | kit.com/brand |
| `CoupaIcon` | `coupa` | pair | #1565C0 | #FFFFFF | 5.56 | 12.47 | the Coupa logo kit linked from coupa.com/company/press-kit, which ships the mark in blue and a white reversed variant |
| `CssIcon` | — | fixed | #663399 | #663399 | 8.13 | 12.47 | github.com/CSS-Next/logo.css (CC0), the official CSS logo endorsed by the W3C CSS WG |
| `CurrencyApiIcon` | `currencyapi` | fixed | #2994FF | #2994FF | 9.13 | 4.67 | currencyapi.com/img/currencyapi_logo_color.svg |
| `DatabricksIcon` | `databricks` | fixed | #FF3621 | #FF3621 | 3.50 | 3.45 | Databricks' own logo asset (databricks.com/sites/default/files/2023-08/databricks-default.png) |
| `DatadogIcon` | `datadog` | pair | #632CA6 | #FFFFFF | 8.32 | 12.47 | datadoghq.com press kit |
| `DatoCmsIcon` | `datocms` | fixed | #FF7751 | #FF7751 | 2.54 | 4.76 | datocms.com/company/brand-assets |
| `DbtIcon` | `dbt_profile` | fixed | #FE6703 | #FE6703 | 2.84 | 4.25 | the dbt Labs brand assets (getdbt.com/brand-guidelines) |
| `DeelIcon` | `deel` | pair | #1B1B1B | #FFFFFF | 16.67 | 12.47 | deel.com's own logo_revamp.svg / logo_revamp_white.svg |
| `DeepInfraIcon` | `deep_infra` | pair | #2A3275 | #4C9CEC | 11.22 | 12.47 | the DeepInfra press-kit logo pack (deepinfra.com/media-center → DEEPINFRA_LOGO_COLOR / DEEPINFRA_LOGO_WHITE) |
| `DeepLIcon` | `deepl` | pair | #0F2B46 | #FFFFFF | 13.97 | 12.47 | DeepL's official logo pack on deepl.com/en/press ("Logo Deep Blue" RGB #0F2B46 and the published "Logo White" reversed variant) |
| `DeepSeekIcon` | `deepseek` | pair | #4D6BFE | #6799FE | 4.19 | 4.49 | deepseek.com design tokens (--ds-color-brand under :root / [data-theme=dark]) |
| `DenoIcon` | — | pair | #000000 | #FFFFFF | 20.32 | 12.47 | the "Deno Logo Guidelines 2024" asset pack on deno.com/brand |
| `DigitalOceanIcon` | `digitalocean` | fixed | #0080FF | #0080FF | 3.67 | 3.29 | DigitalOcean's official logo kit (DO_Logo_icon_blue.svg, linked from digitalocean.com/press) |
| `DiscordIcon` | `discord`, `discord_webhook` | mixed | #5865F2 | #5865F2 | 4.46 | 5.71 | https://discord.com/branding |
| `DiscourseIcon` | `discourse` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | discourse.org/brand (discourse-icon.svg / discourse-icon-dark.svg) |
| `DocSpringIcon` | `docspring` | fixed | #3C8EE0 | #3C8EE0 | 3.31 | 12.47 | DocSpring's own logo SVG, docspring.com/assets/logo-text-*.svg |
| `DockerIcon` | — | fixed | #2560FF | #2560FF | 4.84 | 2.49 | Docker's official logo kit (docker.com/company/newsroom/media-resources) |
| `DocusignIcon` | `docusign` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | brand.docusign.com/logo: only the Nexus overlap flips per background, Cobalt #4C00FF and Poppy #FF5252 must not be recoloured |
| `DropboxIcon` | `dropbox` | fixed | #0061FE | #0061FE | 4.91 | 2.46 | brand.dropbox.com/logo and the DIG token dig-color__primary__base |
| `DuckDbIcon` | `duckdb` | pair | #1A1A1A | #FFF100 | 16.84 | 10.59 | duckdb.org/design logo package (DuckDB_icon-lightmode.svg / DuckDB_icon-darkmode.svg) |
| `DucklakeIcon` | — | pair | #1A1A1A | #2EAFFF | 16.84 | 5.16 | duckdb.org |
| `DustIcon` | `dust` | fixed | #FE9C1A | #FE9C1A | 4.04 | 10.64 | dust.tt/home/brand-resources (Dust_LogoSquare.svg from their brand kit) |
| `DynatraceIcon` | `dynatrace` | fixed | #1496FF | #1496FF | 10.01 | 7.83 | Dynatrace brand guidelines (live.standards.site/dynatrace, Dynatrace_mark_color.svg) |
| `EdgeDbIcon` | `edgedb` | fixed | #8FAF24 | #8FAF24 | 2.44 | 4.94 | geldata.com (favicon/apple-touch-icon glyph and its <link rel="mask-icon" color>) |
| `EnodeIcon` | `enode` | pair | #5D770D | #E8E8E1 | 4.94 | 10.13 | enode.com/static/favicon.svg |
| `EventbriteIcon` | `eventbrite` | fixed | #FF5E30 | #FF5E30 | 2.95 | 4.10 | the 2025 Eventbrite press kit logos; the brand publishes no reversed variant |
| `ExaIcon` | `exa` | pair | #0143D9 | #FFFFFF | 7.24 | 12.47 | exa.ai/brand (Exa Brand Assets kit, Logomark Blue/White) |
| `FaunadbIcon` | `faunadb` | pair | #3F00A5 | #604BE9 | 11.58 | 2.19 | Fauna's own VS Code extension icons (fauna/fauna-vscode: icons/fauna.svg for light themes, icons/fauna-light.svg for dark) |
| `FigmaIcon` | `figma` | fixed | #24CB71 | #24CB71 | 4.42 | 5.85 | static.figma.com/app/icon/2/favicon.svg (2025 brand refresh) |
| `FirebaseIcon` | `firebase` | fixed | #FF9100 | #FF9100 | 4.58 | 7.81 | firebase.google.com/brand-guidelines (Logomark_Full Color.svg in firebase-brand-assets.zip) |
| `FlyIcon` | `fly` | pair | #24175B | #FFFFFF | 15.07 | 12.47 | fly.io |
| `FormstackIcon` | `formstack` | fixed | #21B573 | #21B573 | 2.56 | 4.70 | the brand guide at formstack.com/press-kit |
| `FoxentryIcon` | `foxentry` | fixed | #E74600 | #E74600 | 5.09 | 4.14 | foxentry.com/assets/img/logo-foxentry-symbol.svg |
| `FreshdeskIcon` | `freshdesk` | fixed | #20A849 | #20A849 | 3.01 | 12.47 | Freshworks' own product-logo asset (freshdesk-dew.svg, used on freshworks.com/apps) |
| `FrontAppIcon` | `frontapp` | fixed | #A857F1 | #A857F1 | 3.83 | 3.15 | the logo mark front.com ships inline on its own pages; the mark keeps this purple on both light and dark backgrounds |
| `FunkwhaleIcon` | `funkwhale` | mixed | #009FE3 | #009FE3 | 10.69 | 5.71 | www.funkwhale.audio/logos (theme/images/icon.svg) |
| `GSheetsIcon` | `gsheets` | fixed | #009954 | #009954 | 3.57 | 12.47 | Google product logo sheets_2026q3 (gstatic productlogos, used on workspace.google.com/products/sheets) |
| `GcalIcon` | `gcal` | fixed | #BBE2FF | #BBE2FF | 3.40 | 12.47 | Google's own Calendar 2026 product logo, https://www.gstatic.com/images/branding/productlogos/calendar_2026/v2/web/192px.svg (paths verbatim) |
| `GdocsIcon` | `gdocs` | inherits | #718096 | #A9B0BA | 3.88 | 5.71 | Google's own Docs product icon (gstatic.com/images/branding/productlogos/docs_2026/v2/web/192px.svg, served on workspace.google.com/products/docs) |
| `GdriveIcon` | `gdrive` | fixed | #B43333 | #B43333 | 5.87 | 10.05 | https://www.gstatic.com/images/branding/productlogos/drive_2026/v2/web/192px.svg, Google's own product-logo CDN; paths and gradient stops are verbatim |
| `GhostCmsIcon` | `ghostcms` | pair | #15171A | #FFFFFF | 17.38 | 12.47 | docs.ghost.org |
| `GiphyIcon` | `giphy` | fixed | #FFF35C | #FFF35C | 4.76 | 10.83 | GIPHY's own app icon (giphy.com/static/img/icons/apple-touch-icon-180px.png) |
| `GitBookIcon` | `gitbook` | pair | #181C1F | #F2F7F7 | 16.59 | 11.54 | the GitBook-icon-dark / GitBook-icon-light downloads on gitbook.gitbook.io/brand-assets, matching the live gitbook.com favicon |
| `GitIcon` | `git_repository`, `git` | fixed | #F03C2E | #F03C2E | 3.77 | 3.20 | git-scm.com/community/logos (Git-Icon-1788C.svg, logo by Jason Long, CC BY 3.0) |
| `GithubIcon` | `github` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | brand.github.com/foundations/logo |
| `GitlabIcon` | `gitlab` | mixed | #FC6D26 | #FC6D26 | 4.01 | 6.18 | https://design.gitlab.com/brand-design/color (Orange 03p/02p/01p, "colors from our core logo") |
| `GmailIcon` | `gmail` | mixed | #4285F4 | #4285F4 | 5.61 | 7.30 | gstatic.com/images/branding/product/2x/gmail_2020q4_48dp.png |
| `GoogleAiIcon` | `googleai` | fixed | #217BFE | #217BFE | 3.81 | 5.44 | Google's standard Gemini product icon (gstatic.com/images/branding/productlogos/gemini/v1/192px.svg) |
| `GoogleCalendarIcon` | — | fixed | #BBE2FF | #BBE2FF | 3.40 | 12.47 | the Google Calendar 2026 product icon, taken verbatim from https://www.gstatic.com/images/branding/productlogos/calendar_2026/v2/web/192px.svg |
| `GoogleCloudIcon` | `gcloud`, `gcp_service_account` | fixed | #EA4335 | #EA4335 | 3.80 | 7.30 | Google's own product logo asset https://www.gstatic.com/images/branding/product/2x/google_cloud_64dp.png |
| `GoogleDriveIcon` | — | fixed | #B43333 | #B43333 | 5.87 | 10.05 | https://www.gstatic.com/images/branding/productlogos/drive_2026/v2/web/192px.svg (Drive 2026 mark, copied verbatim) |
| `GoogleFormsIcon` | `gforms` | fixed | #969DFF | #969DFF | 5.99 | 12.47 | Google's own Forms product icon at www.gstatic.com/images/branding/productlogos/forms_2026/v2/web/192px.svg |
| `GoogleIcon` | `google`, `gworkspace` | mixed | #4285F4 | #4285F4 | 3.88 | 7.30 | the G mark Google serves in accounts.google.com/gsi/client |
| `GorgiasIcon` | `gorgias` | pair | #000000 | #FFF9F4 | 20.32 | 11.94 | gorgias.com/about-us/style, which ships the symbol as a "Dark"/"Light" pair |
| `GraphqlIcon` | `graphql` | pair | #E10098 | #FFFFFF | 4.37 | 12.47 | graphql.org |
| `GreipIcon` | `greip` | pair | #141C27 | #FFFFFF | 16.59 | 12.47 | docs.greip.io |
| `GristIcon` | `grist` | fixed | #16B378 | #16B378 | 2.62 | 8.25 | getgrist.com/trademark/assets/ |
| `GroqIcon` | `groqai`, `groq` | fixed | #F43E01 | #F43E01 | 3.67 | 12.47 | https://groq.com/favicon.svg |
| `HackernewsIcon` | `hackernews` | fixed | #FF6600 | #FF6600 | 2.84 | 12.47 | news.ycombinator.com/y18.svg |
| `HoldedIcon` | `holded` | fixed | #FD454D | #FD454D | 3.31 | 3.64 | cdn.holded.com/assets/img/brand/holded-logo.svg |
| `HoneybadgerIcon` | `honeybadger` | fixed | #EA5937 | #EA5937 | 3.40 | 3.55 | honeybadger.io/favicon.svg |
| `HtmlIcon` | — | fixed | #E44D26 | #E44D26 | 3.77 | 12.47 | the W3C HTML5 logo, w3.org/html/logo (downloads/HTML5_Logo.svg) |
| `HubspotIcon` | `hubspot` | pair | #FF2F00 | #FFFFFF | 3.59 | 12.47 | hubspot.com |
| `IfsIcon` | `ifs_cloud_oidc` | fixed | #72C9F8 | #72C9F8 | 6.03 | 6.79 | the IFS symbol on ifs.com |
| `IftttIcon` | `ifttt` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | ifttt.com |
| `InkeepIcon` | `inkeep` | fixed | #D5E5FF | #D5E5FF | 2.45 | 9.79 | Inkeep's brand page "Icon Core" (https://inkeep.com/brand) |
| `IntercomIcon` | `intercom` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | intercom.com |
| `IpinfoIcon` | `ipinfo` | pair | #3091CF | #FFFFFF | 3.34 | 12.47 | ipinfo.io logo-positive.svg and logo-negative.svg |
| `JavaIcon` | — | fixed | #007396 | #007396 | 5.22 | 4.93 | Oracle's Java Branding and Licensing Guidelines v21 (oracle.com/a/ocom/docs/java-licensing-logo-guidelines-1908204.pdf) |
| `JavaScriptIcon` | — | fixed | #F7DF1E | #F7DF1E | 20.32 | 9.22 | js.svg in github.com/voodootikigod/logo.js, the origin of the JavaScript logo |
| `JiraIcon` | `jira` | fixed | #1868DB | #1868DB | 5.03 | 12.47 | Atlassian's official Jira logo pack (atlassian.design/foundations/logos) |
| `JoomlaIcon` | `joomla` | fixed | #7AC143 | #7AC143 | 3.58 | 6.23 | the official logo at cdn.joomla.org/images/joomla-colours-logo.svg |
| `JotformIcon` | `jotform` | pair | #0A1551 | #FFFFFF | 16.40 | 12.47 | jotform.com footer logomark (#jotform-logomark-fourth is filled with --jf-logo-img: #0A1551 light, #fff dark) |
| `JsonIcon` | — | fixed | #F9A825 | #F9A825 | 1.91 | 6.33 | Material Design Yellow 800 (api.flutter.dev Colors.yellow[800]); glyph is Google's Material Symbols "data_object" |
| `JumpCloudIcon` | `jumpcloud` | pair | #002B49 | #F7F7FB | 14.09 | 11.67 | jumpcloud.com/press (Ocean Blue / White Smoke); White Smoke is the brand's own reversed logo for dark backgrounds |
| `KafkaIcon` | `kafka` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | apache/kafka |
| `KanidmIcon` | `kanidm` | fixed | #B1DEF4 | #B1DEF4 | 20.32 | 12.47 | artwork/logo-square.svg in github.com/kanidm/kanidm (full palette: #FF6600 #803300 #D45500 #2A3455 #B1B3B8 #CCCCCC) |
| `KeycloakIcon` | `keycloak` | fixed | #00B8E3 | #00B8E3 | 8.18 | 10.65 | keycloak.org's own mark, https://www.keycloak.org/resources/images/icon.svg (cyan #00B8E3/#33C6E9/#008AAA over greys #4D4D4D–#EDEDED, single theme) |
| `KlaviyoIcon` | `klaviyo` | pair | #1D1E20 | #FFFFFF | 16.14 | 12.47 | klaviyo.com --color-core-charcoal; the flag mark is the standalone logomark the site header collapses to, and the shape of klaviyo.com/icons/icon-512x512.png |
| `KoboToolboxIcon` | `kobotoolbox` | fixed | #2095F3 | #2095F3 | 3.05 | 3.95 | the kobotoolbox.org header logo and $kobo-blue in kobotoolbox/kpi jsapp/scss/colors.scss |
| `KustomerIcon` | `kustomer` | fixed | #FBEC2A | #FBEC2A | 14.08 | 12.47 | kustomer.com/images/kustomer/Kusty.svg |
| `LangfuseIcon` | `langfuse` | fixed | #FF5D5F | #FF5D5F | 2.91 | 4.47 | langfuse.com/brand "Icon - Color (SVG)", used unmodified |
| `LessIcon` | — | pair | #274F82 | #FFFFFF | 8.04 | 12.47 | github.com/less/logo (MIT) |
| `LineIcon` | `line` | fixed | #06C755 | #06C755 | 2.18 | 5.53 | LINE's official brand icon asset (line.me/en/logo) |
| `LinearIcon` | `linear` | pair | #222326 | #F4F5F8 | 15.20 | 11.44 | linear.app/brand |
| `LinkdingIcon` | — | pair | #5856E0 | #ADABF7 | 5.32 | 5.91 | sissbruecker/linkding |
| `LinkedinIcon` | `linkedin` | mixed | #0A66C2 | #0A66C2 | 5.50 | 12.47 | the official inbug SVGs embedded in brand.linkedin.com/in-logo |
| `LinodeIcon` | `linode` | fixed | #004B16 | #004B16 | 10.07 | 4.54 | Linode's own packages/manager/src/assets/logo/logo.svg in linode/manager @3e53c92, the last revision before the Akamai rebrand dropped it |
| `LumaAiIcon` | `lumaai` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | lumalabs.ai (favicon-black.ico on light, favicon-white.ico on dark) |
| `MSSqlServerIcon` | — | fixed | #0094F0 | #0094F0 | 10.54 | 12.47 | learn.microsoft.com/en-us/azure/architecture/icons — Microsoft's anchor blue, a stop in its own SQL Server SVG and throughout the set's Fluent gradients |
| `MSTeamsIcon` | — | fixed | #A98AFF | #A98AFF | 12.96 | 12.47 | Microsoft's Teams-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com, served on microsoft.com/microsoft-teams); every gradient stop here is verbatim from it |
| `MagentoIcon` | `magento` | fixed | #F26322 | #F26322 | 3.09 | 3.91 | Magento's own logo asset, magento2 lib/web/images/logo.svg |
| `MailchimpIcon` | `mailchimp` | fixed | #241C15 | #241C15 | 16.23 | 10.74 | mailchimp.com/about/brand-assets |
| `MailerLiteIcon` | `mailerlite` | fixed | #09C269 | #09C269 | 2.27 | 5.31 | mailerlite.com/brand-assets |
| `MailgunIcon` | `mailgun` | fixed | #F04126 | #F04126 | 3.70 | 12.47 | mailgun.com's own logo-mailgun-icon.svg |
| `MandrillIcon` | `mandrill` | pair | #241C15 | #FFFFFF | 16.23 | 12.47 | mailchimp.com/about/brand-assets and Mandrill's own mandrillapp.com/img/navigation/freddie.svg |
| `MapboxIcon` | `mapbox` | pair | #0E1012 | #FFFFFF | 18.45 | 12.47 | mapbox.com |
| `MarkdownIcon` | — | pair | #000000 | #FFFFFF | 20.32 | 12.47 | dcurtis/markdown-mark (public domain) |
| `MastodonIcon` | `mastodon` | mixed | #6364FF | #6364FF | 7.07 | 12.47 | https://joinmastodon.org/branding |
| `MatrixIcon` | `matrix` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | matrix.org/branding |
| `MatteroomIcon` | `matteroom` | fixed | #134A81 | #134A81 | 8.74 | 12.47 | the MATTEROOM logomark vector at login.matteroom.com/images/login_logo.svg; square tile proportions taken from their own app icon at matteroom.com/favicon.ico |
| `MauticIcon` | `mautic` | pair | #4E5E9E | #FFFFFF | 5.94 | 12.47 | mautic.org/about/brand-logos-graphics (Mautic_Logo_LB.svg / Mautic_Logo_DB.svg); the "M" stays Sunglow #FDB933 in both, as the trademark policy requires the mark in its exact published form without alteration in colour |
| `McpIcon` | `mcp` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | modelcontextprotocol/modelcontextprotocol |
| `MediumIcon` | `medium` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | medium.design |
| `MeteosourceIcon` | `meteosource` | fixed | #FAD961 | #FAD961 | 2.87 | 9.01 | the logo in the meteosource.com site header (no brand page published) |
| `MezmoIcon` | `mezmo` | pair | #0A090C | #E6E6E5 | 19.22 | 9.99 | mezmo.com nav mark and docs.mezmo.com logo/light.png + logo/dark.png |
| `MicrosoftIcon` | `microsoft` | mixed | #F25022 | #F25022 | 3.88 | 7.24 | the official logo asset linked from Microsoft's logo third-party usage guidance |
| `MiroIcon` | `miro` | fixed | #FFDD33 | #FFDD33 | 16.46 | 9.29 | the Miro logo on miro.com |
| `MistralIcon` | `mistral` | inherits | #718096 | #A9B0BA | 3.88 | 5.71 | mistral.ai/favicon.svg (mid-band of the #FFAF01 -> #C4001D ramp); drawn here in currentColor, the monochrome variant mistral.ai/brand ships |
| `MixpanelIcon` | `mixpanel` | pair | #7856FF | #FFFFFF | 4.44 | 12.47 | brand.mixpanel.com/logo and /color (Purple 100); mixpanel.com ships the same pair as its light/dark favicons |
| `MollieIcon` | `mollie` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | Mollie's app icon (my.mollie.com/assets/images/favicons/apple-touch-icon-180x180.png): a full-bleed disc with the lowercase m knocked out |
| `MondayIcon` | `monday` | fixed | #FB275D | #FB275D | 3.66 | 8.25 | monday.com's official logo pack (brand-monday.com/logo) |
| `MongodbIcon` | `mongodb` | pair | #00684A | #00ED64 | 6.60 | 7.90 | MongoDB brand resources and their LeafyGreen palette |
| `MotimateIcon` | `motimate` | fixed | #2DC89C | #2DC89C | 2.06 | 5.85 | motimateapp.com theme assets |
| `MqttIcon` | `mqtt` | pair | #660066 | #FFFFFF | 11.57 | 12.47 | mqtt/mqttorg-graphics |
| `Mysql` | `mysql` | inherits | #718096 | #A9B0BA | 3.88 | 5.71 | — |
| `NatsIcon` | `nats` | pair | #375C93 | #27AAE1 | 6.52 | 4.71 | cncf/artwork |
| `NeonDbIcon` | `neondb` | pair | #37C38F | #34D59A | 2.17 | 6.61 | neon.com/brand (neon-logomark-light-color.svg / neon-logomark-dark-color.svg) |
| `NetBoxIcon` | `netbox` | pair | #001423 | #FFFFFF | 18.07 | 12.47 | theme, so the second path carries its own fill- utilities |
| `NetlifyIcon` | `netlify` | pair | #05BDBA | #32E6E2 | 10.06 | 12.47 | netlify.com/brand (netlify-logo-monogram.zip, full-colour lightmode/darkmode) |
| `NetsuiteIcon` | `netsuite` | fixed | #BACCDB | #BACCDB | 7.71 | 7.57 | — |
| `NewsApiIcon` | `newsapi` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | newsapi.org |
| `NextcloudIcon` | `ocs`, `nextcloud` | pair | #0082C9 | #FFFFFF | 4.03 | 12.47 | nextcloud.com |
| `NocoDbIcon` | `nocodb` | fixed | #4351E8 | #4351E8 | 11.12 | 3.30 | nocodb.com's own Logo.svg / favicon |
| `NotionIcon` | `notion` | fixed | #FFFFFF | #FFFFFF | 20.32 | 12.47 | Notion's own app icon (notion.com/front-static/logo-ios.png) |
| `NuIcon` | — | fixed | #4D9B05 | #4D9B05 | 3.38 | 3.57 | nushell/vscode-nushell-lang assets/nu.svg |
| `OdkIcon` | `odk` | fixed | #3E77B4 | #3E77B4 | 6.37 | 2.67 | ODK brand assets (getodk.org/legal/brand/) |
| `OktaIcon` | `okta` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | okta.com |
| `OneSignalIcon` | `onesignal` | pair | #051B2C | #FFFFFF | 16.94 | 12.47 | OneSignal's official media kit (OneSignal-Logomark.svg / OneSignal-Logomark-White.svg), matching the prefers-color-scheme pair in their own onesignal.com/favicon.svg |
| `OpenRouterIcon` | `openrouter` | pair | #7624F4 | #C8FF00 | 6.10 | 10.55 | openrouter.ai/brand/v2/openrouter-glyph-{light,dark}.svg |
| `OpenWeatherIcon` | `openweather` | pair | #EA6D4A | — | 2.99 | — | openweather.co.uk/brand_guidelines |
| `OpenaiIcon` | `openai` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | openai.com/brand (Blossom_Light.svg / Blossom_Dark.svg) |
| `OracleDBIcon` | `oracledb` | fixed | #C74634 | #C74634 | 4.67 | 2.59 | Oracle's own logo SVG at https://www.oracle.com/a/ocom/img/oracle-logo.svg |
| `OutreachIcon` | `outreach` | pair | #5951FF | #FFFFFF | 5.02 | 12.47 | outreach.ai |
| `PHPIcon` | — | fixed | #AEB2D5 | #AEB2D5 | 20.32 | 12.47 | php.net/images/logos/new-php-logo.svg (php.net/download-logos.php) |
| `PagerDutyIcon` | `pagerduty` | pair | #048A24 | #FFFFFF | 4.35 | 12.47 | pagerduty.com/brand "P icon" pack (P-GreenRGB.svg / P-WhiteRGB.svg) |
| `PandaDocIcon` | `pandadoc` | fixed | #248567 | #248567 | 4.39 | 12.47 | the PandaDoc logo shipped on pandadoc.com (header logo SVG and favicon); white monogram on the green tile in both themes |
| `PaychexIcon` | `paychex` | fixed | #004B8D | #004B8D | 8.50 | **1.42** | paychex.com's own logo SVG (themes/custom/paychex2/images/svg/logo-paychex.svg, .st0) |
| `PaylocityIcon` | `paylocity` | fixed | #ED2024 | #ED2024 | 4.20 | 12.47 | paylocity.com design-system CSS (.styleBGBrandGradient) |
| `PaypalIcon` | `paypal` | fixed | #002991 | #002991 | 11.77 | 6.93 | PayPal's own paypal-mark-color_new.svg (site header logo on paypal.com) |
| `PersonaIcon` | `persona` | fixed | #7379FD | #7379FD | 3.45 | 3.49 | https://withpersona.com/favicon.svg (Persona, identity verification) |
| `PersonioIcon` | `personio` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | personio.design/brand/how-we-look/logo |
| `PhraseIcon` | `phrase` | pair | #181818 | #FFFFFF | 17.18 | 12.47 | Logo_primary.svg and Logo_black_background.svg on phrase.com/brand |
| `PineconeIcon` | `pinecone` | pair | #201D1E | #FFFFFF | 16.18 | 12.47 | pinecone.io/newsroom/media-kit |
| `PinterestIcon` | `pinterest` | fixed | #E60023 | #E60023 | 4.63 | 2.61 | Pinterest Gestalt tokens (color.icon.brand.primary = red.pushpin.450, identical in sema-color-light and sema-color-dark) |
| `PipedriveIcon` | `pipedrive` | fixed | #017737 | #017737 | 5.50 | 12.47 | pipedrive.com logo token --pd-puco-global-color-green-500 |
| `PlanetScaleIcon` | `planetscale` | pair | #1A1A1A | #FAFAFA | 16.84 | 11.95 | planetscale.com |
| `PocketIdIcon` | `pocketid` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | pocket-id.org header logo (fill isDark ? #ffffff : #000000) and pocket-id/pocket-id frontend/src/lib/components/logo.svelte |
| `PostgresIcon` | `postgresql` | mixed | #336791 | #336791 | 20.32 | 12.47 | the official 3-colour Slonik SVG on wiki.postgresql.org/wiki/Logo |
| `PostmarkIcon` | `postmark` | fixed | #FFDE00 | #FFDE00 | 20.32 | 9.33 | postmarkapp.com/images/logo-stamp-simple.svg |
| `PowershellIcon` | — | fixed | #00FF18 | #00FF18 | 20.32 | 12.47 | github.com/PowerShell/PowerShell/blob/master/assets/ps_black_64.svg |
| `PusherIcon` | `pusher` | pair | #300D4F | #FFFFFF | 15.68 | 12.47 | pusher.com media kit (Pusher logo primary.png / Pusher logo secondary.png) |
| `PushoverIcon` | `pushover` | fixed | #249DF1 | #249DF1 | 2.83 | 12.47 | support.pushover.net/i63-pushover-logos-and-usage |
| `QoveryIcon` | `qovery` | fixed | #642DFF | #642DFF | 6.05 | 2.00 | qovery.com/logos/qovery-logo-black.svg |
| `QuickbooksIcon` | `quickbooks` | fixed | #2CA01C | #2CA01C | 3.30 | 3.65 | the QuickBooks logo SVG on intuit.com's press room |
| `RIcon` | — | fixed | #276DC3 | #276DC3 | 6.46 | 7.89 | r-project.org/logo (gradient stops copied from the authoritative Rlogo.svg) |
| `RaindropIcon` | `raindrop` | fixed | #1988E0 | #1988E0 | 5.33 | 12.47 | app.raindrop.io/assets/icon_raw.svg and raindrop.io icon_128.png |
| `ReactIcon` | — | pair | #087EA4 | #58C4DC | 4.48 | 6.14 | react.dev brand menu (images/brand/logo_light.svg, logo_dark.svg) |
| `ReadmeIcon` | `readme` | pair | #213AFF | #FFFFFF | 6.53 | 12.47 | readme.com's own prefers-color-scheme favicon pair (favicon-213aff.ico / favicon-ffffff.ico) |
| `ReadwiseIcon` | `readwise` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | readwise.io's logo-standalone-dark.svg (light) and logo-standalone-white.svg (dark) |
| `RecraftIcon` | `recraft` | fixed | #000000 | #000000 | 20.32 | 12.47 | Recraft's press-kit "Icon White" mark (https://www.recraft.ai/press-releases) |
| `RedditIcon` | `reddit` | fixed | #FF6600 | #FF6600 | 20.32 | 12.47 | redditinc.com/brand ("a stylized Snoo head contained within an OrangeRed (#FF4500) conversation bubble") |
| `RenderIcon` | `render` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | render.com |
| `ReplicateIcon` | `replicate` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | replicate.com header logo (glyph is currentColor; site CSS sets #000, and #FFF under .dark) |
| `ResendIcon` | `resend` | pair | #000000 | #FDFDFD | 20.32 | 12.26 | cdn.resend.com/brand/resend-icon-black.svg and resend-icon-white.svg |
| `RingCentralIcon` | `ringcentral` | pair | #FF7A00 | #FFFFFF | 2.53 | 12.47 | assets.ringcentral.com/us/brand-library/logos/ringcentral-logo.zip (RingCentral logo fullcolor.svg / RingCentral logo white.svg) |
| `RocketChatIcon` | `rocketchat` | fixed | #F5455C | #F5455C | 3.45 | 3.50 | Rocket.Chat brand colours (docs.rocket.chat/v1/docs/colors), the primary red of their logo |
| `RssIcon` | `rss` | fixed | #FFA500 | #FFA500 | 1.91 | 12.47 | Mozilla's feed icon guidelines (mozilla.org/en-US/foundation/feed-icon-guidelines/), which fix no exact hex |
| `RubyIcon` | — | fixed | #FB7655 | #FB7655 | 10.63 | 12.47 | the official logo kit at ruby-lang.org/en/about/logo |
| `RunPodIcon` | `runpod` | pair | #5D29F0 | #FFFFFF | 6.68 | 12.47 | runpod.io/brandkit |
| `RustIcon` | — | pair | #000000 | #FFFFFF | 20.32 | 12.47 | rust-lang/rust-artwork |
| `S3Icon` | `s3` | fixed | #7AA116 | #7AA116 | 2.93 | 12.47 | AWS Architecture Icons (Icon-package_07312026, Arch_Storage/64/Arch_Amazon-Simple-Storage-Service_64.svg) |
| `SageIcon` | `sage_intacct` | pair | #000000 | #00D639 | 20.32 | 6.34 | @sage/design-tokens --logo-sage-bg-default |
| `SalesflareIcon` | `salesflare` | fixed | #0053FF | #0053FF | 5.52 | 2.19 | salesflare.com's own `--color--major-blue` design token |
| `SalesforceIcon` | `salesforce` | fixed | #00B3FF | #00B3FF | 2.29 | 5.28 | brand.salesforce.com/brand/color |
| `SassIcon` | — | fixed | #CC6699 | #CC6699 | 3.43 | 12.47 | sass-lang.com's own style guide token --sl-color--hopbush (assets/dist/css/sass.css) |
| `SegmentIcon` | `segment` | fixed | #52BD94 | #52BD94 | 2.24 | 5.38 | Segment's own app favicon (app.segment.com) and Evergreen green500 #52BD95 |
| `SendflakeIcon` | `snowflake` | pair | #29B5E8 | — | 2.29 | — | snowflake.com/brand-guidelines |
| `SendgridIcon` | `sendgrid` | fixed | #00B3E3 | #00B3E3 | 3.81 | 8.57 | styleguide.sendgrid.com/colors.html |
| `SensorTowerIcon` | `sensortower` | fixed | #00CFB8 | #00CFB8 | 1.91 | 12.47 | sensortower.com/favicon.svg, copied verbatim |
| `SentryIcon` | `sentry` | pair | #181225 | #FFFFFF | 17.64 | 12.47 | sentry.io/branding logo generator (Dark/Light themes, "Invert in dark mode") |
| `ServiceNowIcon` | `servicenow` | fixed | #62D84E | #62D84E | 1.77 | 6.80 | servicenow.com/company/servicenow-logo.html (servicenow-logo-icon.svg) |
| `ShopifyIcon` | `shopify` | fixed | #95BF47 | #95BF47 | 3.75 | 12.47 | shopify.com/brand-assets (shopify-logo-shopping-bag-full-color.svg: #95BF47/#5E8E3E/#fff) |
| `ShortcutIcon` | `shortcut` | pair | #494BCB | #797ADE | 6.45 | 3.36 | shortcut.com/branding (mark-default.svg; the reversed lockup uses #797ADE on dark) |
| `ShutterstockIcon` | `shutterstock` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | brand.shutterstock.com |
| `SigNozIcon` | `signoz` | fixed | #FF5E19 | #FF5E19 | 3.63 | 12.47 | signoz.io/img/SigNozLogo-orange.svg |
| `Slack` | `slack` | mixed | #E01E5A | #E01E5A | 4.51 | 6.52 | slack.com's own nav logo (a.slack-edge.com/38f0e7c/marketing/img/nav/logo.svg, linked from slack.com/media-kit) |
| `SmartsheetIcon` | `smartsheet` | pair | #031C59 | #FFFFFF | 15.46 | 12.47 | brandguides.brandfolder.com/smartsheet-visual-guide/basics |
| `SnowflakeIcon` | — | pair | #29B5E8 | — | 2.29 | — | snowflake.com/brand-guidelines |
| `SpeechifyIcon` | `speechify` | pair | #2F43FA | #FFFFFF | 6.15 | 12.47 | the Speechify brand kit (speechify.com/brand-kit, Logomark_blue.svg and Logomark_white.svg) |
| `SplitwiseIcon` | `splitwise` | pair | #1CC29F | — | 2.19 | — | splitwise.com/press (sw.svg / sw-wide.svg / bg-primary.svg) |
| `SpotifyIcon` | `spotify` | pair | #1ED760 | #FFFFFF | 1.86 | 12.47 | developer.spotify.com/documentation/design (2024 Primary Logo icon pack) |
| `SquareIcon` | `square` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | Square_Logo_2025 in squareup.com/us/en/press/logo |
| `StraleIcon` | `strale` | pair | #0D0D0E | #F2F2F3 | 18.80 | 11.15 | strale.dev favicon.svg and the site's own --foreground token |
| `StravaIcon` | `strava` | fixed | #FC5200 | #FC5200 | 3.20 | 3.77 | developers.strava.com/guidelines (Strava API logo pack, orange SVGs) |
| `StripeIcon` | `stripe` | fixed | #533AFD | #533AFD | 5.99 | 12.47 | Stripe's own favicon.svg and Stripe_logo_kit.zip (stripe.com/newsroom/brand-assets) |
| `SupabaseIcon` | `supabase` | fixed | #3ECF8E | #3ECF8E | 3.75 | 6.25 | supabase.com/brand-assets |
| `SurrealdbIcon` | `surrealdb` | mixed | #D255FE | #D255FE | 7.33 | 5.71 | surrealdb.com/brand |
| `SvelteIcon` | — | fixed | #FF3E00 | #FF3E00 | 3.42 | 12.47 | sveltejs/branding (svelte-logo.svg, white cutout #fff) |
| `TallyIcon` | `tally` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | the "Tally Icon - Black" / "Tally Icon - White" files in the icon pack on tally.so/help/press-kit, matching the live tally.so/favicon.svg |
| `TaskadeIcon` | `taskade` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | taskade.com/press (Mascot Mark light = agent_taskade.svg, Genesis Icon dark = taskade-icon-dark.svg) |
| `TelegramIcon` | `telegram` | fixed | #2AABEE | #2AABEE | 2.92 | 12.47 | Telegram's press-kit Logo.svg (telegram.org/press) |
| `TelnyxIcon` | `telnyx` | pair | #000000 | #00E3AA | 20.32 | 12.47 | telnyx.com |
| `TerraIcon` | `terra` | fixed | #008AFF | #008AFF | 20.32 | 10.43 | tryterra.co/providers/terra_icon.svg, the only vector square mark Terra ships (the site logo is a "TERRA API" wordmark, the favicon a raster .ico) |
| `TheirStackIcon` | `their_stack` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | theirstack.com/en/docs/brand, which lists both as core brand colours |
| `ThreadsIcon` | `threads` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | Meta's Threads Brand Resource Center logo pack (meta.com/brand/resources/threads) |
| `TodoistIcon` | `todoist` | fixed | #E44232 | #E44232 | 3.97 | 3.04 | Todoist Brand Guidelines (doist.com/brand-assets/todoist-logo.zip), "Red — the primary brand color for Todoist" |
| `TogetherAiIcon` | `togetherai` | fixed | #EF2CC1 | #EF2CC1 | 3.50 | 6.46 | together.ai's brand page (https://www.together.ai/brand) |
| `TogglIcon` | `toggl` | pair | #2C1138 | #E57CD8 | 16.33 | 4.87 | Toggl Track media toolkit (toggl.com/track/media-toolkit, icon-dark-purple.svg / icon-pink.svg) |
| `TomorrowIoIcon` | `tomorrow` | fixed | #004CF8 | #004CF8 | 6.00 | 12.47 | tomorrow.io's own design tokens (--color-logo-blue in site-frame.min.css, matching the header lockup SVG and logo-490.png) |
| `TrelloIcon` | `trello` | fixed | #1558BC | #1558BC | 6.44 | 12.47 | Atlassian Design logo library (atlassian.design/foundations/logos → trello_app.zip, Trello_icon.svg) |
| `TripadvisorIcon` | `tripadvisor` | fixed | #002B11 | #002B11 | 15.01 | **1.24** | 2025 Tripadvisor Brand Guidelines for Partners, tripadvisor.mediaroom.com |
| `TursoIcon` | `turso` | pair | #183134 | #FFFFFF | 13.30 | 12.47 | turso.tech/brand (Dark Teal and white logomark variants) |
| `TwilioIcon` | `twilio` | fixed | #F22F46 | #F22F46 | 3.86 | 3.13 | twilio.com (mask-icon color, favicon and apple-touch-icon artwork) |
| `TwitchIcon` | `twitch` | fixed | #9146FF | #9146FF | 4.49 | 2.69 | brand.twitch.com |
| `TwitterIcon` | `twitter` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | about.x.com |
| `TypeformIcon` | `typeform` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | typeform.com/brand |
| `UltravoxIcon` | `ultravox` | fixed | #BB3B57 | #BB3B57 | 6.67 | 7.65 | the ultravox.ai favicon (framerusercontent.com/images/hzAEdihxJ11mv3l4trNh2WprE.svg) |
| `VectaraIcon` | `vectara` | fixed | #7E00FF | #7E00FF | 7.00 | 9.80 | — |
| `VercelIcon` | `vercel` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | vercel.com |
| `VismaIcon` | `visma` | pair | #131313 | #FFFFFF | 17.98 | 12.47 | design.visma.com/logo (VA symbol from the official Visma Logopack) |
| `VueIcon` | — | fixed | #42B883 | #42B883 | 8.97 | 5.00 | vuejs/art logo.svg |
| `WebflowIcon` | `webflow` | fixed | #146EF5 | #146EF5 | 4.44 | 2.72 | brand.webflow.com/brand-assets |
| `WhatsappBusinessIcon` | `whatsapp_business` | fixed | #25D366 | #25D366 | 1.92 | 6.29 | WhatsApp's Digital_Glyph_Green_RGB_2026.svg, shipped by whatsapp.com/business (→ whatsappbusiness.com) |
| `WizIcon` | `wiz` | pair | #0254EC | #FFFFFF | 5.84 | 12.47 | wiz.io/press media kit logo pack (WizLogo_Blue_Vector.svg / WizLogo_White_Vector.svg) |
| `WooCommerceIcon` | `woocommerce` | pair | #873EFF | #FFFFFF | 4.88 | 12.47 | the Woo logo pack at woocommerce.com/brand-and-logo-guidelines (Woo_logo_color.svg and Woo_logo_white.svg) |
| `WordpressIcon` | `wordpress` | pair | #32373C | #FFFFFF | 11.63 | 12.47 | wordpress.org/about/logos/ |
| `XataIcon` | `xata` | fixed | #8468F6 | #8468F6 | 3.84 | 3.14 | xata.io/brand (logo-symbol.svg) |
| `XeroIcon` | `xero` | fixed | #13B5EA | #13B5EA | 2.30 | 12.47 | xero.com favicon.svg and the site header logo (Xero__LogoPath fill) |
| `YamlIcon` | — | mixed | #CB171E | #CB171E | 5.52 | 5.71 | yaml.org's own assets/favicon.svg and assets/logo.png; the Y, M and L carry no fill in YAML's SVG, so they take the surrounding text colour |
| `YelpIcon` | `yelp` | fixed | #FF1A1A | #FF1A1A | 3.75 | 3.22 | yelp.com/brand (burst_red.svg and the official logo kit) |
| `YnabIcon` | `ynab` | pair | #3B5EDA | #FEF9E6 | 5.35 | 11.82 | ynab.com press kit tree logo (Tree Logo Blurple.svg / Tree Logo Buttermilk.svg — the buttermilk reverse is what ynab.com itself uses on its dark footer) |
| `YoutubeIcon` | `youtube` | fixed | #FF0033 | #FF0033 | 3.83 | 12.47 | brand.youtube/color (YouTube Red, updated from #FF0000) |
| `ZammadIcon` | `zammad` | fixed | #CD2015 | #CD2015 | 7.62 | 9.73 | zammad.com favicon-32x32.svg |
| `ZendeskIcon` | `zendesk` | pair | #11110D | #FFFFFF | 18.31 | 12.47 | zendesk.com |
| `ZeroTierIcon` | `zerotier` | fixed | #FFB25B | #FFB25B | 16.58 | 6.99 | zerotier.com's own icon.svg and logo lockups |
| `ZitadelIcon` | `zitadel` | pair | #232323 | #FFFFFF | 15.21 | 12.47 | zitadel/zitadel console assets zitadel-logo-solo-dark.svg / zitadel-logo-solo-light.svg |
| `ZixflowIcon` | `zixflow` | pair | #141414 | #FFFFFF | 17.83 | 12.47 | docs.zixflow.com logo pack (logo/light.svg / logo/dark.svg) |
| `ZohoIcon` | `zoho` | pair | #000000 | #FFFFFF | 20.32 | 12.47 | zoho.com/branding (zoho-logo-web.svg / zoho-logo-white.svg) |
| `ZoomIcon` | `zoom` | pair | #0B5CFF | #FFFFFF | 5.09 | 12.47 | brand.zoom.com |
| `ZuploIcon` | `zuplo` | fixed | #FF00BD | #FF00BD | 3.39 | 3.56 | https://zuplo.com/brand |

## Rules the brand imposes

Constraints that would otherwise be broken by a well-meaning change.

- **AblyIcon** — "Don't use other colours or gradients for the symbol."
- **AcceloIcon** — The mark keeps these three fills in both themes; only the wordmark (not drawn here) swaps #10202D for white.
- **AmqpIcon** — No brand colour: AMQP is an OASIS protocol, not a vendor, and amqp.org publishes no palette (https://www.amqp.org/legal.html). Generic glyph, deliberately monochrome — keep it on currentColor.
- **AnsibleIcon** — The mark is a solid disc knocked out with a white "A", so the pair is applied by inverting rather than currentColor: recolouring the disc alone would leave white on light grey.
- **ApifyIcon** — White/black variants are reserved for monochromatic contexts, so the tricolour mark stays in both themes.
- **AppwriteIcon** — Brand asks that the logo not be altered, so no per-theme variant.
- **ArcGisIcon** — Esri publishes no reversed variant for this badge and forbids altering its logos.
- **AsanaIcon** — Asana's guidelines forbid recolouring: the symbol always appears in coral, on light and dark backgrounds alike.
- **AssemblyAiIcon** — Two colours per theme, so the second stroke carries its own fill- utilities: #777673 on light, #FFFFFF on dark.
- **AttioIcon** — The mark is a filled compound path; stroking it instead thickens it and leaks the default black fill.
- **Auth0Icon** — Okta's content terms forbid altering the mark, so ship only these published variants.
- **AutheliaIcon** — authelia.com/reference/guides/branding permits format/layout changes only — do not alter the design.
- **AuthentikIcon** — The white variant is the brand's own asset for dark backgrounds; proportions and colour must not be altered otherwise.
- **AwsEcrIcon** — AWS ships one flat fill for both themes; the gradient tile was retired in the 2023 accessibility refresh.
- **AwsIcon** — aws.amazon.com/trademark-guidelines forbids altering the logo's colour, so only these two published variants may be used.
- **BaserowIcon** — The mark keeps these three colours on light and dark; only the wordmark reverses to white.
- **BeamerIcon** — The isotype is monochrome in all first-party artwork.
- **BigQueryIcon** — Google publishes no reversed variant, so the same mark is used on both themes.
- **BitbucketIcon** — Atlassian ships brand/neutral/inverse only: "don't use unapproved color combinations".
- **BitlyIcon** — Bitly's reversed logomark is white over orange, not over neutral dark, so the orange mark is used on both.
- **BloggerIcon** — Google publishes no reversed variant.
- **BlueskyIcon** — The downloadable media-kit butterfly still ships the older #006AFF, but the palette is the normative source: "use only the official color values above. Do not substitute, tint, or approximate." White is the approved monochrome variant for dark backgrounds.
- **BoxIcon** — box.com/legal/trademark forbids any other recolouring of the mark.
- **BrevoIcon** — Brevo publishes no per-theme variant of the app mark; the reversed "Mint" #F9FFF6 asset is the wordmark only.
- **BrowserlessIcon** — Its own prefers-color-scheme block sets black on light, white on dark.
- **BubbleIcon** — Bubble's brand terms forbid re-colouring the mark beyond its published dark/light pair.
- **BuildkiteIcon** — Buildkite ships a single mark "for any context", so there is no per-theme variant, and asks that it not be altered.
- **ButtondownIcon** — Brand forbids recolouring the logo, so no per-theme variant.
- **CSharpIcon** — Its README forbids altering the mark, so the same full-colour icon is used on light and dark.
- **CalcomIcon** — Cal.com's design system states it is deliberately a grayscale brand and publishes exactly two logo variants.
- **CalendlyIcon** — Guidelines: "Only show our logo and lockups in blue or white."
- **CertopusIcon** — Verbatim copy of the brand's own circle mark: #2C353D and the white disc are its other fixed tones, not a dark-theme variant.
- **CircleCiIcon** — Guidelines require Terminal (#161616) on light backgrounds and White on dark, and forbid any color not named in them.
- **CiscoIcon** — Cisco requires all parts of the mark be knocked out to white on dark backgrounds.
- **ClerkIcon** — Same two-tone symbol on light and dark; the mono symbol-dark/symbol-light pair is Clerk's alternate for single-colour contexts.
- **ClickhouseIcon** — Brand forbids recolouring the mark, so only its own published pair is used.
- **CloseIcon** — Close forbids modifying the logo, so the same colours are kept on light and dark.
- **CloudflareIcon** — Cloudflare's logo guidelines forbid altering the colours or filling the flare, so the flare stays knocked out on both themes.
- **CloudinaryIcon** — Cloudinary Blue is reserved for the logo; no other recolouring is permitted.
- **CockroachDbIcon** — The full-colour mark is a cyan-to-purple gradient; Cockroach Labs reduces it to solid white on dark backgrounds.
- **CoinbaseIcon** — Coinbase asks that the mark not be altered or recoloured, so only these two published variants are used.
- **ComapeoIcon** — Awana Digital publishes no reversed variant, so dark mode uses the brand's own accent blue #0066FF from CoMapeoLogo.svg (digidem/comapeo-mobile); the navy is 1.3:1 on dark surfaces.
- **ConfluenceIcon** — Atlassian requires the logo be used without modification, and its brand appearance is identical in light and dark.
- **ContentfulIcon** — Same full-colour mark on light and dark.
- **ContiguityIcon** — The `>_` glyph is knocked out to the opposite colour, so it carries its own fill- utilities.
- **ConvertKitIcon** — ConvertKit rebranded to Kit in 2024.
- **CssIcon** — Small-size variant; the only per-theme variants published are mono black/white fallbacks, so the rebeccapurple tile is kept in both themes.
- **DatadogIcon** — Datadog publishes one mark per background, the purple tile with Bits knocked out on light and the white Bits silhouette on dark, and forbids recolouring or inverting either.
- **DatoCmsIcon** — Brand kit forbids altering the logo's shape or colour.
- **DbtIcon** — Their Trademark Policy states "The dbt logo mark color cannot be altered", so this stays orange on both themes.
- **DeelIcon** — Post-rebrand the period is a square in the wordmark colour, not a blue circle.
- **DeepInfraIcon** — Their brand guidelines say "use the primary white logo on dark backgrounds", where the connector bars invert to #FFFFFF.
- **DenoIcon** — Deno publishes no hex and forbids colorizing; the black "Light (no outline)" and white "Dark (outlined)" marks are separate artworks to be swapped per background, never inverted.
- **DiscordIcon** — Discord forbids recolouring the logo, so no per-theme variant.
- **DiscourseIcon** — Only the outer bubble reverses; the five inner colours are the same in both variants.
- **DockerIcon** — Docker requires its logos appear only in its primary brand colours.
- **DropboxIcon** — Dropbox's branding terms forbid recolouring the logo, and their inverse-theme token keeps the same blue.
- **DuckDbIcon** — The pair is a full inversion, so the duck carries its own fill- utilities; the manual forbids recolouring outside these two brand hexes.
- **DustIcon** — Dust's guidelines forbid recolouring the logo.
- **DynatraceIcon** — Guidelines forbid colorizing the logo, so all six fills stay fixed in both themes.
- **EdgeDbIcon** — EdgeDB is now Gel — edgedb.com redirects to geldata.com — so this is Gel's "g" symbol, not the retired EDGE|DB wordmark.
- **EnodeIcon** — Their own favicon carries the pair in a prefers-color-scheme block.
- **ExaIcon** — Blue for standard applications, white on dark backgrounds.
- **FigmaIcon** — Figma's guidelines forbid modifying the marks, so the five-colour original is used on both themes.
- **FirebaseIcon** — Same full-colour artwork on light and dark; the guidelines forbid recolouring or redrawing the mark.
- **FoxentryIcon** — Fixed tri-tone mark, no per-theme variant: the brand ships a separate greyscale logo rather than a recoloured one.
- **FreshdeskIcon** — Freshworks publishes no reversed variant: the white glyph always sits on the green leaf.
- **FunkwhaleIcon** — Identity guidelines forbid recolouring.
- **GSheetsIcon** — Google forbids recolouring its marks, so the same full-colour artwork is used on both themes.
- **GcalIcon** — Google forbids modifying its logos, colour included, so this stays fixed with no per-theme pair.
- **GdriveIcon** — Google forbids modifying its logos "in any way, including changing the color", so this stays full-colour with no per-theme pair.
- **GiphyIcon** — Same full-colour mark on light and dark.
- **GitBookIcon** — #1C1917 is the marketing palette's dark base, not the logomark.
- **GitIcon** — A white reversed logomark exists, but git-scm.com's own dark theme exempts the mark from inversion and keeps it orange.
- **GithubIcon** — GitHub allows the Invertocat in white or black only and forbids recolouring it, so the pair is fixed here rather than inherited from the caller.
- **GmailIcon** — Google's brand guidelines forbid recolouring the mark.
- **GoogleAiIcon** — Google ships no reversed variant; the same gradient is used on light and dark.
- **GoogleCalendarIcon** — Google's trademark guidelines forbid distorting or altering a brand feature, so no per-theme recolour.
- **GoogleCloudIcon** — Google forbids recolouring its logos.
- **GoogleDriveIcon** — Google's Drive branding guide permits resizing only — no other change to the logo — so no per-theme recolour.
- **GoogleFormsIcon** — Google's brand guidelines forbid modifying or recolouring its product icons.
- **GoogleIcon** — developers.google.com/identity/branding-guidelines forbids changing the colour of the G.
- **GorgiasIcon** — The guide allows only black or white for the symbol: "Do not use gray!".
- **GristIcon** — "Keep it exactly as depicted — no recoloring, no cropping."
- **GroqIcon** — Logo use in a UI requires a license from Groq.
- **HoldedIcon** — Holded ships one flat red mark for both themes; the red-orange gradient is retired.
- **HubspotIcon** — The legacy Coral #FF7A59 is not the current logo color.
- **IfsIcon** — IFS's negative lockup reverses only the wordmark, so the symbol keeps its #8427E2-to-#72C9F8 gradient on dark.
- **IftttIcon** — IFTTT's brand guidelines state "Our wordmark may be used in solid white or black" and publish no other hex for the mark.
- **IntercomIcon** — Intercom ships the mark as fill="currentColor" bound to its nav foreground token, so it takes the colour of the surface it sits on.
- **JavaIcon** — The Coffee Cup mark is licensee-only and "you may not use a modified version of the Coffee Cup logo" — do not recolour it or flatten it to currentColor.
- **JavaScriptIcon** — Fixed mark: yellow field, black lettering, no per-theme variant.
- **JiraIcon** — The logomark is identical on light and dark; only the wordmark changes colour.
- **JoomlaIcon** — Joomla's trademark policy forbids recolouring the mark, so there is no per-theme variant.
- **JotformIcon** — The other three bars keep their fixed brand colours in both themes.
- **JsonIcon** — JSON itself has no brand owner or published colours — json.org states none — so this is a Material palette pick, not a brand colour.
- **KanidmIcon** — Kanidm's artwork is CC-BY-NC-ND — no recolouring or other derivatives.
- **KlaviyoIcon** — Klaviyo draws it in currentColor, hence the white swap on dark.
- **LangfuseIcon** — Langfuse's trademark terms forbid modifying the assets.
- **LineIcon** — LINE forbids any change to the logo's colour, so there is no reversed variant.
- **LinearIcon** — Guidelines ship a light/dark logomark pair and forbid altering the assets in any other way.
- **LinkedinIcon** — That page forbids recolouring: only the approved blue, black and white variants.
- **LinodeIcon** — The keyline path stays unfilled so it follows currentColor instead of the source's near-black #231f20.
- **LumaAiIcon** — The two faces ship at 65% opacity, which is what makes their overlap read as a cube.
- **MSSqlServerIcon** — Microsoft licenses its product icons for diagrams, docs, and training only, and forbids cropping, rotating, or reshaping them.
- **MSTeamsIcon** — Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes.
- **MailchimpIcon** — Mailchimp forbids altering the files, so both official tones are painted and neither is recoloured per theme.
- **MailerLiteIcon** — Their IP guidelines forbid altering or recolouring the mark.
- **MailgunIcon** — Mailgun ships no reversed variant; the tile is identical on light and dark.
- **MandrillIcon** — On dark their rule is the reversed (white) Freddie; Cavendish Yellow #FFE01B is a background colour, never the mark.
- **MarkdownIcon** — Spec: keep the enclosure's aspect ratio and radius, keep the M/arrow/box relative sizes, and draw all three in one colour.
- **MastodonIcon** — Swap to the black or white logo rather than recolouring when contrast fails.
- **MatrixIcon** — Artwork is the Foundation's matrix-icon.svg verbatim; the trademark policy forbids altering it.
- **MediumIcon** — Guidelines mandate black or white only for both the wordmark and the icon and forbid "any other colors, gradients, or filled with images".
- **MezmoIcon** — The star stays #F4B811 in both themes.
- **MicrosoftIcon** — Microsoft forbids recolouring the symbol, so it stays full-colour on both themes.
- **MistralIcon** — Brand forbids any other recolouring.
- **MixpanelIcon** — "The Mixpanel logo is only ever used in three colors: black, white and the primary brand purple."
- **MollieIcon** — The m is the glyph from Mollie-Logo-Black-2023.svg (Mollie logo pack, mollie.com/resources), scaled and placed to match that icon pixel for pixel; the logo pack itself ships only the 320x94 wordmark. Mollie publishes black and white variants, so the pair flips for dark mode.
- **MondayIcon** — All three colours are required; the brand forbids monochrome or recoloured versions, so no dark-theme variant.
- **MongodbIcon** — MongoDB permits only four logo colours, chosen for contrast with the background, and forbids any other recolour.
- **MotimateIcon** — Motimate is a registered trademark of Motimate AS (Kahoot!).
- **Mysql** — Used under Fair Use: https://fr.wikipedia.org/wiki/Fichier:MySQL.svg
- **NeonDbIcon** — Neon forbids recolouring, so only these published variants may be used.
- **NetlifyIcon** — Two colours per theme, so the "n" carries its own fill- utilities: #014847 on light, #FFFFFF on dark.
- **NetsuiteIcon** — Pre-Oracle NetSuite "N" mark. #125580/#baccdb approximate netsuite.com's own 2014 logo art, which is itself inconsistent: /portal/common/img/ns-logo.png is #14487e/#b9c9d5 and /portal/common/img/logo-ns-mobile.png is #13527d/#b6c7d5 (both via web.archive.org/web/2014/). Not Oracle's current NetSuite mark, which is a different logo in a different palette (#264759/#36677D/#94BFCE/#E2C06B).
- **NocoDbIcon** — NocoDB publishes no reversed variant; the full-colour mark is used on light and dark alike.
- **NotionIcon** — The plate is fixed, not theme-swapped: the mark is pure black and disappears on dark backgrounds without it.
- **NuIcon** — Nushell registers that one file as both the `light` and `dark` icon, so the green is not theme-swapped.
- **OdkIcon** — ODK publishes no reversed or monochrome variant.
- **OktaIcon** — Okta's official April-2025 logo package (logos-04-2025.zip) ships the mark in Black and White only.
- **OpenWeatherIcon** — Their negative (dark-background) logo reverses only the wordmark; the symbol stays brand orange.
- **OpenaiIcon** — The guidelines state "DON'T add any colors to the Blossom" — black or white only.
- **OracleDBIcon** — Oracle reserves its logo for licensees.
- **PHPIcon** — Official logo, CC BY-SA 4.0: keep it verbatim and credit Colin Viebrock rather than recolouring.
- **PaychexIcon** — The isolated P is the square mark Paychex ships as its own 192x192 app icon. Paychex requires prior approval for any use of its marks.
- **PaypalIcon** — The third fill is the deep/bright blue overlap: a two-colour or flat fill loses it.
- **PersonioIcon** — Black on light, white on dark or coloured backgrounds.
- **PhraseIcon** — The green wedge stays #03EAB3 in both — Phrase forbids altering the logo mark colour.
- **PineconeIcon** — The mark is stroke-only, so fill must stay none.
- **PinterestIcon** — Brand guidelines: "Do not alter the logo colour."
- **PipedriveIcon** — Pipedrive's partner media kit says "do not alter, rotate, modify or animate the logo", so the mark keeps its published colours on both themes.
- **PostgresIcon** — The PostgreSQL trademark policy forbids recolouring the mark without prior approval.
- **PostmarkIcon** — Postmark publishes no reversed variant.
- **PowershellIcon** — Trademarked Microsoft logo, exempt from that repo's MIT license. The #00FF18 line below is opacity-0 in the upstream asset and paints nothing.
- **PusherIcon** — Only the colourways shown in their brand guidelines are permitted.
- **PushoverIcon** — Forbids recolouring.
- **QoveryIcon** — The mark keeps the same purple in Qovery's white lockup for dark backgrounds, so there is no reversed variant.
- **QuickbooksIcon** — Intuit forbids altering the mark.
- **RIcon** — R Foundation licenses the mark CC-BY-SA 4.0 / GPL-2 — attribution required, changes must be indicated.
- **RaindropIcon** — Full-colour mark, no reversed variant published.
- **ReadwiseIcon** — The serif R and its highlight block are knocked out to the opposite colour, so they carry their own fill- utilities. Do not restore the mix-blend-mode: multiply wrapper Readwise's dark file carries: it turns the knocked-out white to the backdrop colour on anything but a white page.
- **RecraftIcon** — The plated mark carries its own background: recraft.ai serves it to prefers-color-scheme light and dark alike — not a theme pair.
- **RedditIcon** — Reddit publishes no reversed variant; the icon must always appear in Orangered when in colour.
- **RenderIcon** — Official Render Brand Kit contains only Black and White logomark folders and the SVGs use pure black / pure white.
- **ResendIcon** — Brand guidelines forbid multi-color use or altering the mark, so these are the only two published fills.
- **RssIcon** — Never rotate or flip the mark.
- **RubyIcon** — CC BY-SA 2.5; the kit's LICENSE asks that the mark not represent anything other than the Ruby language.
- **RunPodIcon** — Brand forbids recolouring, so both values are its own published cube-icon variants.
- **RustIcon** — rust-lang.org ships only rust-logo-blk.svg (pure black).
- **S3Icon** — AWS ships no dark variant for service icons.
- **SageIcon** — Sage sets its logo black on light surfaces and Sage green only on dark ones.
- **SalesforceIcon** — Salesforce reserves the white/reversed cloud for its own blue backgrounds, so the blue mark stands in both themes.
- **SegmentIcon** — Segment ships the mark in one flat green and publishes no reversed variant.
- **SendflakeIcon** — Snowflake Blue is the only approved logo color; the sole alternate is a white reverse reserved for full-bleed Snowflake Blue.
- **SendgridIcon** — Twilio's trademark guidelines forbid recolouring the mark, so it stays multicolour in both themes.
- **ServiceNowIcon** — Trademark guidelines require the mark in the graphic form provided.
- **ShopifyIcon** — The "S" stays white regardless of background; no gradients, shadows or recolouring.
- **ShortcutIcon** — The mark "is used across various colors but never changes its visual structure."
- **ShutterstockIcon** — Brand rule: the logo is only ever black or white.
- **SigNozIcon** — SigNoz ships no reversed variant; the tile mark is used unchanged on light and dark.
- **Slack** — Fixed full-colour mark, no per-theme variant.
- **SmartsheetIcon** — Those are two of the approved logo colorways; the guide forbids any other recolouring.
- **SnowflakeIcon** — Snowflake Blue is the only approved logo color; the sole alternate is a white reverse reserved for full-bleed Snowflake Blue.
- **SpeechifyIcon** — The blue logomark is reserved for white backgrounds; every other background takes the black or white monochrome version.
- **SplitwiseIcon** — Splitwise's logos carry a single green and no reversed variant, so the same colour is used on light and dark.
- **SpotifyIcon** — Spotify permits the green icon only on black or white backgrounds and requires the white monochrome colourway on any other dark background, so the pair is fixed rather than caller-set.
- **SquareIcon** — Square ships only black and white logo files and states "Do not change the color", so no tinted variant is allowed.
- **StraleIcon** — Strale's own logo component fills with currentColor, so the mark is meant to take the theme's foreground.
- **StravaIcon** — Strava's guidelines forbid modifying or altering its logos, and the orange Echelon is the mark Strava itself uses on light and dark alike.
- **StripeIcon** — Stripe's Marks Usage Terms forbid altering the marks, so this ships verbatim in both themes rather than as a recoloured pair.
- **SupabaseIcon** — Forbids modifying or recolouring the mark.
- **SurrealdbIcon** — Same gradient mark on light and dark; monochrome variants are for subtle placements only.
- **SvelteIcon** — Its guidelines count the official colour scheme as part of the mark — do not recolour.
- **TaskadeIcon** — Brand forbids recolouring, so only those two published variants are used.
- **TelegramIcon** — The shaded-plane drawing is Telegram's retired Logo_old.
- **TerraIcon** — The outlines are part of the artwork and stay black in both themes; the blue T carries the mark on dark backgrounds.
- **TheirStackIcon** — The mark ships black-only, but the same brand rules forbid placing it on low-contrast backgrounds; on a dark surface black is 1.68:1, so it is tinted to the brand's own white.
- **ThreadsIcon** — The pack ships the mark in black and white only, so it must never be tinted.
- **TogglIcon** — Toggl requires the mark be used as is, unmodified.
- **TomorrowIoIcon** — Their stylesheet reverses only the wordmark on dark headers (path.logo-letter{fill:#fff}); the mark itself stays logo blue in both themes.
- **TrelloIcon** — Atlassian: never compose your own versions or deconstruct official assets.
- **TripadvisorIcon** — Dark backgrounds take Tripadvisor's separate outlined Ollie, never an inverted or recoloured one.
- **TwilioIcon** — Twilio reserves its corporate logo for permitted use and forbids recreating or modifying it.
- **TwitchIcon** — Trademark guidelines forbid recolouring.
- **TwitterIcon** — The component draws the X glyph, not the legacy bird.
- **TypeformIcon** — Default brand colours are "Paper (white) and Ink (black)".
- **UltravoxIcon** — Gradient mark; ultravox.ai links that same asset for both prefers-color-scheme light and dark, so there is no per-theme variant.
- **VectaraIcon** — #7E00FF → #07FEEE iridescent sweep, sampled from the logo mark on vectara.com (no brand kit is published). Vectara reserves its trademarks: do not recolour.
- **VercelIcon** — Vercel ships only light-theme (black) and dark-theme (white) triangle marks and explicitly forbids modifying or recoloring the trademarks.
- **VismaIcon** — Positive black on light, negative white on dark; the logo may not be given any other colour.
- **VueIcon** — Vue's dark-background variant is separate outlined artwork rather than a recolour, so the two-tone mark is used in both themes.
- **WebflowIcon** — Mark ships in blue, black or white only.
- **WhatsappBusinessIcon** — "You shouldn't modify any colors in our logos."
- **WooCommerceIcon** — Automattic requires the mark in its exact, most up-to-date form, so only their two published colorways are used, never a recolour.
- **WordpressIcon** — Every official logotype vector is BaseGray #32373C, shipped alongside a White/transparent version for dark backgrounds.
- **XataIcon** — Brand forbids recolouring, and the full-colour symbol is the same purple in light and dark modes.
- **XeroIcon** — Single-colour mark: white wordmark on the blue badge in both themes.
- **YamlIcon** — YAML publishes no reversed variant, and its black letters would be invisible on the dark surface.
- **YelpIcon** — Yelp forbids altering the logos and requires the ® to accompany the mark at all times.
- **YoutubeIcon** — "The triangle in the full-color red icon must always be white."
- **ZendeskIcon** — Zendesk's brand guidelines specify the logo in Licorice #11110D and Coconut #FFFFFF only and forbid unapproved color variations.
- **ZeroTierIcon** — The tile is identical on light and dark; only the wordmark inverts.
- **ZitadelIcon** — The gradient chevrons stay #FF8F00→#FE00FF in both variants.
- **ZohoIcon** — The four squares carry their own brand hexes (#E42527/#089949/#226DB4/#F9B21D) on light; Zoho's reversed lock-up is entirely white, so they invert with the wordmark on dark.
- **ZoomIcon** — The logo "may only be used in Bloom (#0B5CFF), White, or Black", with White reserved for dark backgrounds and Black requiring prior brand approval.
- **ZuploIcon** — Brand guidelines forbid recolouring the mark; pink is the official variant on both light and dark surfaces.

## Concept icons

Not brands. These inherit `currentColor` on purpose and must not be given a pair.

`AgentInstructionsIcon`, `AiAgentIcon`, `ApiKeyAuthIcon`, `AssetDatabaseIcon`, `AssetDucklakeIcon`, `AssetGenericIcon`, `AssetResIcon`, `AssetS3Icon`, `BarsStaggered`, `BasicHttpAuthIcon`, `BcryptIcon`, `CACertificate`, `CustomAiIcon`, `DbIcon`, `FormInputIcon`, `FunnelCog`, `GpgKeyIcon`, `HttpIcon`, `JsonSchemaIcon`, `LdapIcon`, `Mail`, `OauthIcon`, `PaintbrushOff`, `QRCodeIcon`, `QuestionInputIcon`, `RecordIcon`, `RestIcon`, `SchedulePollIcon`, `SignatureAuthIcon`, `SparklesOffIcon`, `WebdavIcon`, `WindmillAiIcon`, `WindmillIcon`, `WindmillIcon2`

## Coverage

- brand icons: **314**, of which **310** carry a recorded source
- per-theme pairs applied: **136** (5 of them by inversion or a two-SVG swap, see above)
- concept icons: **34**
- effectively invisible on light: **1** (AbstractApiIcon)
- effectively invisible on dark: **2** (PaychexIcon, TripadvisorIcon)
