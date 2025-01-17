/*!
  updater helps running brupop on existing EKS cluster and clean up all
  resources once completing integration test
!*/
use snafu::{ensure, ResultExt};
use std::process::Command;

use k8s_openapi::api::core::v1::Node;
use kube::api::{Api, ListParams};

const CURRENT_LOCATION_PATH: &str = "integ/src";
const PODS_TEMPLATE: &str = "pods-template.yaml";
const KUBECTL_BINARY: &str = "kubectl";

#[derive(strum_macros::Display, Debug)]
pub enum Action {
    Apply,
    Delete,
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^= Deletion and Creation of brupop resources  =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

// install or destroy all brupop resources
pub async fn process_brupop_resources(action: Action, kube_config_path: &str) -> UpdaterResult<()> {
    let action_string: String = action.to_string();

    let brupop_resource_status = Command::new(KUBECTL_BINARY)
        .args([
            &action_string.to_lowercase(),
            "-f",
            "yamlgen/deploy/bottlerocket-update-operator.yaml",
            "--kubeconfig",
            kube_config_path,
        ])
        .status()
        .context(update_error::BrupopProcess)?;

    ensure!(
        brupop_resource_status.success(),
        update_error::BrupopRun {
            action: action_string
        }
    );
    Ok(())
}

// =^..^=   =^..^=   =^..^=   =^..^=   =^..^= Deletion and Creation of test pods  =^..^=   =^..^=   =^..^=   =^..^=   =^..^=

// create or delete statefulset pods, stateless nginx pods, and pods with PDBs on EKS cluster
pub async fn process_pods_test(action: Action, kube_config_path: &str) -> UpdaterResult<()> {
    let action_string: String = action.to_string();

    let pods_status = Command::new(KUBECTL_BINARY)
        .args([
            &action_string.to_lowercase(),
            "-f",
            format!("{}/{}", CURRENT_LOCATION_PATH, PODS_TEMPLATE).as_str(),
            "--kubeconfig",
            kube_config_path,
        ])
        .status()
        .context(update_error::ProcessPodsTest {
            action: action_string.clone(),
        })?;

    ensure!(
        pods_status.success(),
        update_error::PodsRun {
            action: action_string
        }
    );
    Ok(())
}

// Find if any node is running in the cluster
pub async fn nodes_exist(k8s_client: kube::client::Client) -> UpdaterResult<bool> {
    let nodes: Api<Node> = Api::all(k8s_client.clone());

    let nodes_objectlist = nodes
        .list(&ListParams::default())
        .await
        .context(update_error::FindNodes {})?;

    Ok(nodes_objectlist.iter().count() > 0)
}

/// The result type returned by instance create and termiante operations.
type UpdaterResult<T> = std::result::Result<T, update_error::Error>;

pub mod update_error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility = "pub")]
    pub enum Error {
        #[snafu(display("Failed to install brupop: {}", source))]
        BrupopProcess { source: std::io::Error },

        #[snafu(display("Failed to process brupop resources: {:?} brupop", action))]
        BrupopRun { action: String },

        #[snafu(display("Failed to {:?} pods", action))]
        ProcessPodsTest {
            action: String,
            source: std::io::Error,
        },

        #[snafu(display("Failed to process pods test: {:?} pods", action))]
        PodsRun { action: String },

        #[snafu(display("Unable to convert kubeconfig path to string path"))]
        ConvertPathToStr {},

        #[snafu(display("Fail to list EKS cluster nodes: {}", source))]
        FindNodes { source: kube::Error },
    }
}
