// Adapted from https://github.com/denoland/deno/blob/a1764f7690cfdc3e42724fcad29ef954b7e576a4/cli/module_loader.rs - Licensed under MIT by the Deno authors

use deno_cli::args::TsTypeLib;
use deno_cli::emit::emit_parsed_source;
use deno_cli::node;
use deno_cli::node::NodeResolution;
use deno_cli::proc_state::ProcState;
use deno_cli::util::text_encoding::code_without_source_map;
use deno_cli::util::text_encoding::source_map_from_code;

use deno_ast::MediaType;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::Context;
use deno_core::error::custom_error;
use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::futures::future::FutureExt;
use deno_core::futures::Future;
use deno_core::resolve_url;
use deno_core::ModuleCode;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::OpState;
use deno_core::ResolutionKind;
use deno_core::SourceMapGetter;
use deno_graph::EsmModule;
use deno_graph::JsonModule;
use deno_runtime::deno_node::NodeResolutionMode;
use deno_runtime::permissions::PermissionsContainer;
use std::cell::RefCell;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::str;

struct ModuleCodeSource {
    pub code: ModuleCode,
    pub found_url: ModuleSpecifier,
    pub media_type: MediaType,
}

pub struct WindmillModuleLoader {
    pub lib: TsTypeLib,
    /// The initial set of permissions used to resolve the static imports in the
    /// worker. These are "allow all" for main worker, and parent thread
    /// permissions for Web Worker.
    pub root_permissions: PermissionsContainer,
    /// Permissions used to resolve dynamic imports, these get passed as
    /// "root permissions" for Web Worker.
    dynamic_permissions: PermissionsContainer,
    pub ps: ProcState,
}

impl WindmillModuleLoader {
    pub fn new(
        ps: ProcState,
        root_permissions: PermissionsContainer,
        dynamic_permissions: PermissionsContainer,
    ) -> Rc<Self> {
        Rc::new(WindmillModuleLoader {
            lib: ps.options.ts_type_lib_window(),
            root_permissions,
            dynamic_permissions,
            ps,
        })
    }

    pub fn new_for_worker(
        ps: ProcState,
        root_permissions: PermissionsContainer,
        dynamic_permissions: PermissionsContainer,
    ) -> Rc<Self> {
        Rc::new(WindmillModuleLoader {
            lib: ps.options.ts_type_lib_worker(),
            root_permissions,
            dynamic_permissions,
            ps,
        })
    }

    fn load_prepared_module(
        &self,
        specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
    ) -> Result<ModuleCodeSource, AnyError> {
        if specifier.scheme() == "node" {
            unreachable!(); // Node built-in modules should be handled internally.
        }

        let graph = self.ps.graph();
        match graph.get(specifier) {
            Some(deno_graph::Module::Json(JsonModule {
                source, media_type, specifier, ..
            })) => Ok(ModuleCodeSource {
                code: source.clone().into(),
                found_url: specifier.clone(),
                media_type: *media_type,
            }),
            Some(deno_graph::Module::Esm(EsmModule { source, media_type, specifier, .. })) => {
                let code: ModuleCode = match media_type {
                    MediaType::JavaScript
                    | MediaType::Unknown
                    | MediaType::Cjs
                    | MediaType::Mjs
                    | MediaType::Json => source.clone().into(),
                    MediaType::Dts | MediaType::Dcts | MediaType::Dmts => Default::default(),
                    MediaType::TypeScript
                    | MediaType::Mts
                    | MediaType::Cts
                    | MediaType::Jsx
                    | MediaType::Tsx => {
                        // get emit text
                        emit_parsed_source(
                            &self.ps.emit_cache,
                            &self.ps.parsed_source_cache,
                            specifier,
                            *media_type,
                            source,
                            &self.ps.emit_options,
                            self.ps.emit_options_hash,
                        )?
                    }
                    MediaType::TsBuildInfo | MediaType::Wasm | MediaType::SourceMap => {
                        panic!("Unexpected media type {media_type} for {specifier}")
                    }
                };

                // at this point, we no longer need the parsed source in memory, so free it
                self.ps.parsed_source_cache.free(specifier);

                Ok(
                    ModuleCodeSource {
                        code,
                        found_url: specifier.clone(),
                        media_type: *media_type,
                    },
                )
            }
            _ => {
                let mut msg = format!("Loading unprepared module: {specifier}");
                if let Some(referrer) = maybe_referrer {
                    msg = format!("{}, imported from: {}", msg, referrer.as_str());
                }
                Err(anyhow!(msg))
            }
        }
    }

    fn load_sync(
        &self,
        specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dynamic: bool,
    ) -> Result<ModuleSource, AnyError> {
        let code_source = if self.ps.npm_resolver.in_npm_package(specifier) {
            let file_path = specifier.to_file_path().unwrap();
            let code = std::fs::read_to_string(&file_path).with_context(|| {
                let mut msg = "Unable to load ".to_string();
                msg.push_str(&file_path.to_string_lossy());
                if let Some(referrer) = &maybe_referrer {
                    msg.push_str(" imported from ");
                    msg.push_str(referrer.as_str());
                }
                msg
            })?;

            let code = if self.ps.cjs_resolutions.lock().contains(specifier) {
                let mut permissions = if is_dynamic {
                    self.dynamic_permissions.clone()
                } else {
                    self.root_permissions.clone()
                };
                // translate cjs to esm if it's cjs and inject node globals
                node::translate_cjs_to_esm(
                    &self.ps.file_fetcher,
                    specifier,
                    code,
                    MediaType::Cjs,
                    &self.ps.npm_resolver,
                    &self.ps.node_analysis_cache,
                    &mut permissions,
                )?
            } else {
                // only inject node globals for esm
                node::esm_code_with_node_globals(&self.ps.node_analysis_cache, specifier, code)?
            };
            ModuleCodeSource {
                code: code.into(),
                found_url: specifier.clone(),
                media_type: MediaType::from_specifier(specifier),
            }
        } else {
            self.load_prepared_module(specifier, maybe_referrer)?
        };
        let code = if self.ps.options.is_inspecting() {
            // we need the code with the source map in order for
            // it to work with --inspect or --inspect-brk
            code_source.code
        } else {
            // reduce memory and throw away the source map
            // because we don't need it
            code_without_source_map(code_source.code)
        };
        Ok(ModuleSource::new_with_redirect(
            match code_source.media_type {
                MediaType::Json => ModuleType::Json,
                _ => ModuleType::JavaScript,
            },
            code,
            specifier,
            &code_source.found_url,
        ))
    }
}

impl ModuleLoader for WindmillModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, AnyError> {
        let mut permissions = if matches!(kind, ResolutionKind::DynamicImport) {
            self.dynamic_permissions.clone()
        } else {
            self.root_permissions.clone()
        };
        resolve(
            &self.ps,
            specifier,
            referrer,
            &mut permissions,
            self.ps.options.initial_cwd(),
        )
    }

    fn load(
        &self,
        specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dynamic: bool,
    ) -> Pin<Box<deno_core::ModuleSourceFuture>> {
        // NOTE: this block is async only because of `deno_core` interface
        // requirements; module was already loaded when constructing module graph
        // during call to `prepare_load` so we can load it synchronously.
        Box::pin(deno_core::futures::future::ready(self.load_sync(
            specifier,
            maybe_referrer,
            is_dynamic,
        )))
    }

    fn prepare_load(
        &self,
        _op_state: Rc<RefCell<OpState>>,
        specifier: &ModuleSpecifier,
        _maybe_referrer: Option<String>,
        is_dynamic: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), AnyError>>>> {
        if self.ps.npm_resolver.in_npm_package(specifier) {
            // nothing to prepare
            return Box::pin(deno_core::futures::future::ready(Ok(())));
        }

        let specifier = specifier.clone();
        let ps = self.ps.clone();

        let dynamic_permissions = self.dynamic_permissions.clone();
        let root_permissions = if is_dynamic {
            self.dynamic_permissions.clone()
        } else {
            self.root_permissions.clone()
        };
        let lib = self.lib;

        async move {
            ps.prepare_module_load(
                vec![specifier],
                is_dynamic,
                lib,
                root_permissions,
                dynamic_permissions,
            )
            .await
        }
        .boxed_local()
    }
}

impl SourceMapGetter for WindmillModuleLoader {
    fn get_source_map(&self, file_name: &str) -> Option<Vec<u8>> {
        let specifier = resolve_url(file_name).ok()?;
        match specifier.scheme() {
            // we should only be looking for emits for schemes that denote external
            // modules, which the disk_cache supports
            "wasm" | "file" | "http" | "https" | "data" | "blob" => (),
            _ => return None,
        }
        let source = self.load_prepared_module(&specifier, None).ok()?;
        source_map_from_code(&source.code)
    }

    fn get_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
        let graph = self.ps.graph();
        let code = match graph.get(&resolve_url(file_name).ok()?) {
            Some(deno_graph::Module::Esm(module)) => &module.source,
            Some(deno_graph::Module::Json(module)) => &module.source,
            _ => return None,
        };
        // Do NOT use .lines(): it skips the terminating empty line.
        // (due to internally using_terminator() instead of .split())
        let lines: Vec<&str> = code.split('\n').collect();
        if line_number >= lines.len() {
            Some(format!(
        "{} Couldn't format source line: Line {} is out of bounds (source may have changed at runtime)",
        deno_runtime::colors::yellow("Warning"), line_number + 1,
      ))
        } else {
            Some(lines[line_number].to_string())
        }
    }
}

fn handle_node_resolve_result(
    ps: &ProcState,
    result: Result<Option<node::NodeResolution>, AnyError>,
) -> Result<ModuleSpecifier, AnyError> {
    let response = match result? {
        Some(response) => response,
        None => return Err(generic_error("not found")),
    };
    if let NodeResolution::CommonJs(specifier) = &response {
        // remember that this was a common js resolution
        ps.cjs_resolutions.lock().insert(specifier.clone());
    } else if let NodeResolution::BuiltIn(specifier) = &response {
        return node::resolve_builtin_node_module(specifier);
    }
    Ok(response.into_url())
}

pub fn resolve(
    ps: &ProcState,
    specifier: &str,
    referrer: &str,
    permissions: &mut PermissionsContainer,
    cwd: &Path,
) -> Result<ModuleSpecifier, AnyError> {
    let referrer_result = deno_core::resolve_url_or_path(specifier, cwd);

    if let Ok(referrer) = referrer_result.as_ref() {
        if ps.npm_resolver.in_npm_package(referrer) {
            todo!("node::node_resolve uses RealFs - unsupported right now");
            // we're in an npm package, so use node resolution
            return handle_node_resolve_result(
                ps,
                node::node_resolve(
                    specifier,
                    referrer,
                    NodeResolutionMode::Execution,
                    &ps.npm_resolver,
                    permissions,
                ),
            )
            .with_context(|| format!("Could not resolve '{specifier}' from '{referrer}'."));
        }

        let graph = ps.graph_container.graph();
        let maybe_resolved = match graph.get(referrer) {
            Some(deno_graph::Module::Esm(module)) => {
                module.dependencies.get(specifier).map(|d| &d.maybe_code)
            }
            _ => None,
        };

        match maybe_resolved {
            Some(deno_graph::Resolution::Ok(resolved)) => {
                let specifier = &resolved.specifier;

                return match graph.get(specifier) {
                    Some(deno_graph::Module::Npm(module)) => {
                        todo!(
                            "node::node_resolve_npm_reference uses RealFs - unsupported right now"
                        );
                        handle_node_resolve_result(
                            ps,
                            node::node_resolve_npm_reference(
                                &module.nv_reference,
                                NodeResolutionMode::Execution,
                                &ps.npm_resolver,
                                permissions,
                            ),
                        )
                        .with_context(|| format!("Could not resolve '{}'.", module.nv_reference))
                    }
                    Some(deno_graph::Module::Node(module)) => {
                        node::resolve_builtin_node_module(&module.module_name)
                    }
                    Some(deno_graph::Module::Esm(module)) => Ok(module.specifier.clone()),
                    Some(deno_graph::Module::Json(module)) => Ok(module.specifier.clone()),
                    Some(deno_graph::Module::External(module)) => {
                        Ok(node::resolve_specifier_into_node_modules(&module.specifier))
                    }
                    None => Ok(specifier.clone()),
                };
            }
            Some(deno_graph::Resolution::Err(err)) => {
                return Err(custom_error(
                    "TypeError",
                    format!("{}\n", err.to_string_with_range()),
                ))
            }
            Some(deno_graph::Resolution::None) | None => {}
        }
    }

    // Built-in Node modules
    if let Some(module_name) = specifier.strip_prefix("node:") {
        return node::resolve_builtin_node_module(module_name);
    }

    let referrer = referrer_result?;

    let resolution = resolve_graph(&ps.resolver, specifier, &referrer);

    resolution
}

fn resolve_graph(
    resolver: &deno_cli::resolver::CliGraphResolver,
    specifier: &str,
    referrer: &ModuleSpecifier,
) -> Result<ModuleSpecifier, AnyError> {
    fn resolve_package_json_dep(
        specifier: &str,
        deps: &deno_cli::args::package_json::PackageJsonDeps,
    ) -> Result<Option<ModuleSpecifier>, AnyError> {
        for (bare_specifier, req_result) in deps {
            if specifier.starts_with(bare_specifier) {
                let path = &specifier[bare_specifier.len()..];
                if path.is_empty() || path.starts_with('/') {
                    let req = req_result.as_ref().map_err(|err| {
          anyhow!(
            "Parsing version constraints in the application-level package.json is more strict at the moment.\n\n{:#}",
            err.clone()
          )
        })?;
                    return Ok(Some(ModuleSpecifier::parse(&format!("npm:{req}{path}"))?));
                }
            }
        }

        Ok(None)
    }

    // attempt to resolve with the import map first
    let err = match resolver
        .maybe_import_map
        .as_ref()
        .expect("Windmill always uses an import map")
        .resolve(specifier, referrer)
    {
        Ok(value) => return Ok(value),
        Err(err) => err,
    };

    // then with package.json
    if let Some(deps) = resolver.package_json_deps_installer.package_deps().as_ref() {
        if let Some(specifier) = resolve_package_json_dep(specifier, deps)? {
            resolver.found_package_json_dep_flag.raise();
            return Ok(specifier);
        }
    }

    return Err(err.into());
}
