use sysinfo::{Disks, Networks, System};

use crate::pb::{Disk, Heartbeat, Inventory, NetworkInterface};

pub struct SystemCollector {
    sys: System,
}

impl SystemCollector {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn heartbeat(&mut self) -> Heartbeat {
        self.sys.refresh_memory();
        let load = System::load_average();
        Heartbeat {
            sent_at_unix_seconds: now_unix(),
            uptime_seconds: System::uptime(),
            load_1m: load.one,
            load_5m: load.five,
            load_15m: load.fifteen,
            memory_total_bytes: self.sys.total_memory(),
            memory_used_bytes: self.sys.used_memory(),
        }
    }

    pub fn inventory(&mut self) -> Inventory {
        self.sys.refresh_all();

        let disks = Disks::new_with_refreshed_list()
            .iter()
            .map(|d| Disk {
                name: d.name().to_string_lossy().into_owned(),
                model: String::new(), // not exposed by sysinfo; SMART fills this in
                serial: String::new(), // ditto
                size_bytes: d.total_space(),
            })
            .collect();

        let network_interfaces = Networks::new_with_refreshed_list()
            .iter()
            .map(|(name, data)| NetworkInterface {
                name: name.clone(),
                mac_address: data.mac_address().to_string(),
                addresses: data
                    .ip_networks()
                    .iter()
                    .map(|ip| ip.addr.to_string())
                    .collect(),
            })
            .collect();

        Inventory {
            hostname: System::host_name().unwrap_or_default(),
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            kernel: System::kernel_version().unwrap_or_default(),
            cpu_model: self
                .sys
                .cpus()
                .first()
                .map(|c| c.brand().to_string())
                .unwrap_or_default(),
            cpu_cores: self.sys.cpus().len() as u32,
            memory_total_bytes: self.sys.total_memory(),
            disks,
            network_interfaces,
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
