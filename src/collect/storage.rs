use sysinfo::Disks;

use crate::pb::{Filesystem, StorageUsage};

pub fn collect() -> StorageUsage {
    let filesystems = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| Filesystem {
            mount_point: d.mount_point().to_string_lossy().into_owned(),
            fs_type: d.file_system().to_string_lossy().into_owned(),
            device: d.name().to_string_lossy().into_owned(),
            total_bytes: d.total_space(),
            used_bytes: d.total_space().saturating_sub(d.available_space()),
        })
        .collect();
    StorageUsage { filesystems }
}
