/*!

The custom resource definitions are modeled as Rust structs. Here we generate
the corresponding k8s yaml files.

!*/

use models::{
    agent::{
        agent_cluster_role, agent_cluster_role_binding, agent_daemonset, agent_service_account,
    },
    apiserver::{
        apiserver_auth_delegator_cluster_role_binding, apiserver_cluster_role,
        apiserver_cluster_role_binding, apiserver_deployment, apiserver_service,
        apiserver_service_account,
    },
    controller::{
        controller_cluster_role, controller_cluster_role_binding, controller_deployment,
        controller_priority_class, controller_service, controller_service_account,
    },
    namespace::brupop_namespace,
    node::combined_crds,
};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

const YAMLGEN_DIR: &str = env!("CARGO_MANIFEST_DIR");
const HEADER: &str = "# This file is generated. Do not edit.\n";

fn main() {
    dotenv::dotenv().ok();
    // Re-run this build script if the model changes.
    println!("cargo:rerun-if-changed=../models/src");
    // Re-run the yaml generation if these variables change
    println!("cargo:rerun-if-env-changed=BRUPOP_CONTAINER_IMAGE");
    println!("cargo:rerun-if-env-changed=BRUPOP_CONTAINER_IMAGE_PULL_SECRET");

    let path = PathBuf::from(YAMLGEN_DIR)
        .join("deploy")
        .join("bottlerocket-update-operator.yaml");
    let mut brupop_resources = File::create(&path).unwrap();

    // testsys-crd related K8S manifest
    brupop_resources.write_all(HEADER.as_bytes()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &combined_crds()).unwrap();

    let brupop_image = env::var("BRUPOP_CONTAINER_IMAGE").ok().unwrap();
    let brupop_image_pull_secrets = env::var("BRUPOP_CONTAINER_IMAGE_PULL_SECRET").ok();
    let exclude_from_lb_wait_time: u64 = env::var("EXCLUDE_FROM_LB_WAIT_TIME_IN_SEC")
        .ok()
        .unwrap()
        .parse()
        .unwrap();

    serde_yaml::to_writer(&brupop_resources, &brupop_namespace()).unwrap();

    // cert-manager and secret
    let cert_path = PathBuf::from(YAMLGEN_DIR).join("deploy").join("cert.yaml");
    let mut cert_file = File::open(cert_path).unwrap();
    let mut contents = String::new();
    cert_file.read_to_string(&mut contents).unwrap();
    brupop_resources.write_all(contents.as_bytes()).unwrap();

    // apiserver resources
    serde_yaml::to_writer(&brupop_resources, &apiserver_service_account()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &apiserver_cluster_role()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &apiserver_cluster_role_binding()).unwrap();
    serde_yaml::to_writer(
        &brupop_resources,
        &apiserver_auth_delegator_cluster_role_binding(),
    )
    .unwrap();
    serde_yaml::to_writer(
        &brupop_resources,
        &apiserver_deployment(brupop_image.clone(), brupop_image_pull_secrets.clone()),
    )
    .unwrap();
    serde_yaml::to_writer(&brupop_resources, &apiserver_service()).unwrap();

    // agent resources
    serde_yaml::to_writer(&brupop_resources, &agent_service_account()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &agent_cluster_role()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &agent_cluster_role_binding()).unwrap();
    serde_yaml::to_writer(
        &brupop_resources,
        &agent_daemonset(
            brupop_image.clone(),
            brupop_image_pull_secrets.clone(),
            exclude_from_lb_wait_time,
        ),
    )
    .unwrap();

    // controller resources
    serde_yaml::to_writer(&brupop_resources, &controller_service_account()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &controller_cluster_role()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &controller_cluster_role_binding()).unwrap();
    serde_yaml::to_writer(&brupop_resources, &controller_priority_class()).unwrap();
    serde_yaml::to_writer(
        &brupop_resources,
        &controller_deployment(brupop_image.clone(), brupop_image_pull_secrets.clone()),
    )
    .unwrap();
    serde_yaml::to_writer(&brupop_resources, &controller_service()).unwrap();
}
