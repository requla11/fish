//! MicroVM hardware isolation configuration and lifecycle management.
//!
//! Generates Firecracker / Cloud Hypervisor VM configurations for hermetic
//! build execution. Actual VM creation requires Linux + KVM; this module
//! provides the declarative config, image specification, and lifecycle state
//! machine so callers can integrate with their hypervisor of choice.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Hypervisor backend selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hypervisor {
    /// AWS Firecracker microVM (Linux + KVM required).
    Firecracker,
    /// Cloud Hypervisor (Linux + KVM, Rust implementation).
    CloudHypervisor,
    /// QEMU (portable but heavier).
    Qemu,
}

/// Resource allocation for one microVM instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroVmConfig {
    pub vcpus: u8,
    pub memory_mb: u64,
    /// Path to the rootfs image (ext4 or initramfs).
    pub rootfs_path: PathBuf,
    /// Path to the kernel image (vmlinux or bzImage).
    pub kernel_path: PathBuf,
    /// Kernel boot arguments.
    pub boot_args: String,
    /// Host directories shared into the guest via virtiofs.
    pub shared_dirs: HashMap<String, PathBuf>,
    /// Network: `none` for fully isolated builds.
    pub network_mode: NetworkMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    /// No network — maximum hermeticity.
    None,
    /// NAT through host — allows package downloads during setup.
    Nat,
    /// Bridge to a specific host interface.
    Bridged(String),
}

impl Default for MicroVmConfig {
    fn default() -> Self {
        Self {
            vcpus: 2,
            memory_mb: 2048,
            rootfs_path: PathBuf::from("/opt/fish/vm/rootfs.ext4"),
            kernel_path: PathBuf::from("/opt/fish/vm/vmlinux"),
            boot_args: "console=ttyS0 reboot=k panic=1".to_string(),
            shared_dirs: HashMap::new(),
            network_mode: NetworkMode::None,
        }
    }
}

/// Lifecycle states for a microVM build sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Created,
    Booting,
    Running,
    ExecutingTask,
    ShuttingDown,
    Terminated,
    Failed(String),
}

/// Generate a Firecracker JSON config from a [`MicroVmConfig`].
///
/// The output is compatible with `firecracker --config-file`.
pub fn generate_firecracker_config(config: &MicroVmConfig) -> String {
    let network_section = match &config.network_mode {
        NetworkMode::None => r#"    "network_interfaces": []"#.to_string(),
        NetworkMode::Nat => r#"    "network_interfaces": [{
        "guest_mac": "06:00:AC:10:00:02",
        "host_dev_name": "tap0"
    }]"#
        .to_string(),
        NetworkMode::Bridged(dev) => {
            format!(r#"    "network_interfaces": [{{"host_dev_name": "{dev}"}}]"#)
        }
    };

    let drives = format!(
        r#"    "drives": [
        {{
            "drive_id": "rootfs",
            "path_on_host": "{}",
            "is_root_device": true,
            "is_read_only": false
        }}
    ]"#,
        config.rootfs_path.display()
    );

    format!(
        r#"{{
    "machine-config": {{
        "vcpu_count": {},
        "mem_size_mib": {}
    }},
    "boot-source": {{
        "kernel_image_path": "{}",
        "boot_args": "{}"
    }},
{},
{}
}}"#,
        config.vcpus,
        config.memory_mb,
        config.kernel_path.display(),
        config.boot_args,
        drives,
        network_section,
    )
}
