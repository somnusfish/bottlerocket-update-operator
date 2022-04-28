/// A package aim to enable migrations to new Custom Resource Definitions.
/// Each edition in BottlerocketShadowSpec, BottlerocketShadowState, BottlerocketShadowStatus
/// should result into a new version for BottlerocketShadow.
///
/// Edit the following line with the latest BottlerocketShadow version so that
/// the new version could be exposed to the rest of the program
#[cfg_attr(doctest, doc = " ````no_test")]
/// ```
/// pub use self::v2::{
///    BottlerocketShadow, BottlerocketShadowSpec, BottlerocketShadowState, BottlerocketShadowStatus,
/// };
/// ```
///
/// Push the BottlerocketShadow version to the end of BOTTLEROCKETSHADOW_CRD_METHODS
/// So the build tool could find all BottlerocketShadow versions and generate correct
/// yaml file for CustomResrouceDefinition
#[cfg_attr(doctest, doc = " ````no_test")]
/// ```
/// static ref BOTTLEROCKETSHADOW_CRD_METHODS: Vec<fn() -> CustomResourceDefinition> = {
///    // A list of CRD methods for different BottlerocketShadow version
///    // The latest version should be added at the end of the vector.
///    let mut crd_methods = Vec::new();
///    crd_methods.push(v1::BottlerocketShadow::crd as fn()->CustomResourceDefinition);
///    crd_methods.push(v2::BottlerocketShadow::crd as fn()->CustomResourceDefinition);
///    crd_methods
/// };
/// ```
///
pub mod v1;
pub mod v2;

pub use self::v2::{
    BottlerocketShadow, BottlerocketShadowSpec, BottlerocketShadowState, BottlerocketShadowStatus,
};

use crate::constants::{
    APISERVER, APISERVER_CRD_CONVERT_ENDPOINT, APISERVER_SERVICE_PORT, NAMESPACE,
};
use crate::node::{error, error::Error};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::{
    CustomResourceConversion, CustomResourceDefinition, ServiceReference, WebhookClientConfig,
    WebhookConversion,
};
use kube::CustomResourceExt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;

lazy_static! {
    static ref BOTTLEROCKETSHADOW_CRD_METHODS: Vec<fn() -> CustomResourceDefinition> = {
        // A list of CRD methods for different BottlerocketShadow version
        // The latest version should be added at the end of the vector.
        let mut crd_methods = Vec::new();
        crd_methods.push(v1::BottlerocketShadow::crd as fn()->CustomResourceDefinition);
        crd_methods.push(v2::BottlerocketShadow::crd as fn()->CustomResourceDefinition);
        crd_methods
    };
}
//const CRD_LATEST_VERSION: &str = "v2"; // TODO, work with constants API_VERSION

pub trait BottlerocketShadowResource: kube::ResourceExt {}

pub trait Selector {
    fn selector(&self) -> error::Result<BottlerocketShadowSelector>;
}

impl<T: BottlerocketShadowResource> Selector for T {
    fn selector(&self) -> error::Result<BottlerocketShadowSelector> {
        let node_owner = self
            .meta()
            .owner_references
            .as_ref()
            .ok_or(Error::MissingOwnerReference { name: self.name() })?
            .first()
            .ok_or(Error::MissingOwnerReference { name: self.name() })?;

        Ok(BottlerocketShadowSelector {
            node_name: node_owner.name.clone(),
            node_uid: node_owner.uid.clone(),
        })
    }
}

/// Indicates the specific k8s node that BottlerocketShadow object is associated with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BottlerocketShadowSelector {
    pub node_name: String,
    pub node_uid: String,
}

impl BottlerocketShadowSelector {
    pub fn brs_resource_name(&self) -> String {
        brs_name_from_node_name(&self.node_name)
    }
}

impl fmt::Display for BottlerocketShadowSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.node_name, self.node_uid)
    }
}

pub fn brs_name_from_node_name(node_name: &str) -> String {
    format!("brs-{}", node_name)
}

/// Combine all different versions of custom resources into one CustomeResourceDefinition yaml
/// kube-rs didn't provide a good way to combine CRDs: https://github.com/kube-rs/kube-rs/issues/569
/// In the combination, this method will keep all settings (metadata, apiVersion, etc.) in lastet_crd,
/// and add the spec.versions part in each old_crd to spec.versions part in latest_crd.
/// When adding those old version, the storage value would be set to false,
/// since only one storage true is allowed among all CRD versions.
fn combine_version_in_crds(
    mut latest_crd: CustomResourceDefinition,
    old_crds: Vec<CustomResourceDefinition>,
) -> CustomResourceDefinition {
    for old_crd in old_crds {
        let mut old_versions = old_crd.spec.versions;

        // Adjust storage value via #derive(CustomResource) is supported yet.
        for old_version in &mut old_versions {
            old_version.storage = false;
        }
        latest_crd.spec.versions.append(&mut old_versions);
    }
    latest_crd
}

/// TODO Explain more
/// TODO add conversion_review_versions as varialble
fn generate_webhook_conversion() -> CustomResourceConversion {
    let service = ServiceReference {
        name: APISERVER.to_string(),
        namespace: NAMESPACE.to_string(),
        path: Some(APISERVER_CRD_CONVERT_ENDPOINT.to_string()),
        port: Some(APISERVER_SERVICE_PORT),
    };

    let config = WebhookClientConfig {
        service: Some(service),
        ..Default::default()
    };

    let webhook_conversion = WebhookConversion {
        client_config: Some(config),
        conversion_review_versions: ["v1".to_string(), "v2".to_string()].to_vec(),
    };

    let crd_conversion = CustomResourceConversion {
        strategy: "Webhook".to_string(),
        webhook: Some(webhook_conversion),
    };
    crd_conversion
}

/// TODO explain more
fn add_webhook_setting(
    mut combined_version_crds: CustomResourceDefinition,
) -> CustomResourceDefinition {
    combined_version_crds.spec.conversion = Some(generate_webhook_conversion());
    combined_version_crds
}

pub fn combined_crds() -> CustomResourceDefinition {
    let mut crds: Vec<CustomResourceDefinition> = BOTTLEROCKETSHADOW_CRD_METHODS
        .iter()
        .map(|crd_method| crd_method())
        .collect();
    let latest_crd = crds.pop().unwrap();
    let combined_version_crds = combine_version_in_crds(latest_crd, crds);
    add_webhook_setting(combined_version_crds)
}
