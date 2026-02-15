#[cfg(feature = "private")]
pub use crate::reconciler_ee::run;

#[cfg(feature = "private")]
pub fn print_crd_yaml() {
    use kube::CustomResourceExt;
    let crd = crate::crd_ee::WindmillInstance::crd();
    println!("{}", serde_yml::to_string(&crd).unwrap());
}

#[cfg(not(feature = "private"))]
pub async fn run(_db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
    anyhow::bail!("K8s operator is not available in this build")
}

#[cfg(not(feature = "private"))]
pub fn print_crd_yaml() {
    eprintln!("K8s operator CRD generation is not available in this build");
}
