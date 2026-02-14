pub mod crd;
pub mod db_sync;
pub mod reconciler;
pub mod resolve;

pub use reconciler::run;

/// Print the CRD YAML definition to stdout.
pub fn print_crd_yaml() {
    use kube::CustomResourceExt;
    let crd = crd::WindmillInstance::crd();
    println!("{}", serde_yml::to_string(&crd).unwrap());
}
