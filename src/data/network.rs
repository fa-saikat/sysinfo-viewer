use super::fixture;
use std::fs;
use sysinfo::Networks;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InterfaceState {
    Up,
    Down,
    Unknown,
}

impl InterfaceState {
    pub fn label(self) -> &'static str {
        match self {
            InterfaceState::Up => "up",
            InterfaceState::Down => "down",
            InterfaceState::Unknown => "unknown",
        }
    }
}

pub struct NetworkInterface {
    pub name: String,
    pub state: InterfaceState,
    pub mac_address: String,
    /// First non-loopback IPv4 (falls back to IPv6) bound to this
    /// interface, or `None` if it has no address (e.g. link down).
    pub ip_address: Option<String>,
}

pub fn collect() -> Vec<NetworkInterface> {
    let networks = Networks::new_with_refreshed_list();
    let ip_addresses = interface_ip_addresses();

    let mut interfaces: Vec<NetworkInterface> = networks
        .iter()
        .map(|(name, data)| NetworkInterface {
            name: name.clone(),
            state: read_operstate(name),
            mac_address: data.mac_address().to_string(),
            ip_address: ip_addresses.get(name).cloned(),
        })
        .collect();

    interfaces.sort_by(|a, b| a.name.cmp(&b.name));
    interfaces
}

/// `/sys/class/net/<iface>/operstate` — Linux-specific but authoritative
/// and doesn't require shelling out. Reports "unknown" for interfaces the
/// kernel itself can't determine link state for (common on virtual
/// interfaces like some bridges), which we surface as-is.
fn read_operstate(iface: &str) -> InterfaceState {
    match fs::read_to_string(fixture::sys_path(&format!(
        "/sys/class/net/{iface}/operstate"
    ))) {
        Ok(state) => classify_operstate(state.trim()),
        Err(_) => InterfaceState::Unknown,
    }
}

/// Pure classifier over an `operstate` file body, kept separate from the
/// file read so tests can drive it with canned inputs.
fn classify_operstate(state: &str) -> InterfaceState {
    match state {
        "up" => InterfaceState::Up,
        "down" => InterfaceState::Down,
        _ => InterfaceState::Unknown,
    }
}

/// Maps interface name -> first assigned IP (v4 preferred, v6 fallback).
/// `sysinfo::Networks` doesn't carry addresses, so `if-addrs` fills that
/// gap; it's the only place this crate is used.
fn interface_ip_addresses() -> std::collections::HashMap<String, String> {
    // Track a (is_ipv4, address) pair per interface while scanning, so a
    // later IPv4 entry can replace an earlier IPv6 one; the final map
    // returned to callers only keeps the address string.
    let mut best: std::collections::HashMap<String, (bool, String)> = std::collections::HashMap::new();

    let Ok(addrs) = if_addrs::get_if_addrs() else {
        return std::collections::HashMap::new();
    };

    for addr in addrs {
        let is_v4 = addr.ip().is_ipv4();
        best.entry(addr.name.clone())
            .and_modify(|existing| {
                if is_v4 && !existing.0 {
                    *existing = (true, addr.ip().to_string());
                }
            })
            .or_insert((is_v4, addr.ip().to_string()));
    }

    best.into_iter().map(|(name, (_, ip))| (name, ip)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_operstates() {
        assert_eq!(classify_operstate("up"), InterfaceState::Up);
        assert_eq!(classify_operstate("down"), InterfaceState::Down);
        assert_eq!(InterfaceState::Up.label(), "up");
        assert_eq!(InterfaceState::Down.label(), "down");
    }

    #[test]
    fn unknown_for_anything_the_kernel_cannot_determine() {
        for raw in ["unknown", "dormant", "", "UP"] {
            assert_eq!(classify_operstate(raw), InterfaceState::Unknown);
        }
        assert_eq!(InterfaceState::Unknown.label(), "unknown");
    }
}
