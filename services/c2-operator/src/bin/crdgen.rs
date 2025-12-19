use c2_operator::C2Cluster;
use kube::CustomResourceExt;

fn main() {
    let crd = C2Cluster::crd();
    let yaml = serde_yaml::to_string(&crd).expect("failed to render CRD");
    println!("{yaml}");
}
