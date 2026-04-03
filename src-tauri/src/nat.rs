use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NatManager {
    mappings: Arc<RwLock<HashMap<u64, NatEntry>>>,
    next_port: Arc<RwLock<u16>>,
}

#[derive(Clone)]
pub struct NatEntry {
    pub internal_ip: u32,
    pub internal_port: u16,
    pub external_port: u16,
    pub external_ip: u32,
    pub protocol: Protocol,
    pub target_ip: u32,
    pub target_port: u16,
    pub created_at: u64,
    pub last_active: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Protocol {
    TCP,
    UDP,
}

impl NatManager {
    pub fn new() -> Self {
        Self {
            mappings: Arc::new(RwLock::new(HashMap::new())),
            next_port: Arc::new(RwLock::new(40000)),
        }
    }

    pub fn create_mapping(
        &self,
        internal_ip: u32,
        internal_port: u16,
        external_ip: u32,
        protocol: Protocol,
        target_ip: u32,
        target_port: u16,
    ) -> Option<NatEntry> {
        let port = {
            let mut next = self.next_port.write().ok()?;
            let p = *next;
            *next = if *next >= 60000 { 40000 } else { *next + 1 };
            p
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = NatEntry {
            internal_ip,
            internal_port,
            external_port: port,
            external_ip,
            protocol,
            target_ip,
            target_port,
            created_at: now,
            last_active: now,
        };

        let key = Self::make_key(external_ip, port, protocol);
        let mut mappings = self.mappings.write().ok()?;
        mappings.insert(key, entry.clone());

        Some(entry)
    }

    pub fn find_by_external(
        &self,
        external_ip: u32,
        external_port: u16,
        protocol: Protocol,
    ) -> Option<NatEntry> {
        let key = Self::make_key(external_ip, external_port, protocol);
        let mappings = self.mappings.read().ok()?;
        let mut entry = mappings.get(&key)?.clone();
        entry.last_active = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        drop(mappings);

        let mut mappings = self.mappings.write().ok()?;
        mappings.insert(key, entry.clone());
        Some(entry)
    }

    pub fn find_by_internal(
        &self,
        internal_ip: u32,
        internal_port: u16,
        protocol: Protocol,
    ) -> Option<NatEntry> {
        let mappings = self.mappings.read().ok()?;
        for entry in mappings.values() {
            if entry.internal_ip == internal_ip
                && entry.internal_port == internal_port
                && entry.protocol == protocol
            {
                let mut e = entry.clone();
                e.last_active = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return Some(e);
            }
        }
        None
    }

    pub fn remove_mapping(&self, external_ip: u32, external_port: u16, protocol: Protocol) -> bool {
        let key = Self::make_key(external_ip, external_port, protocol);
        let mut mappings = self.mappings.write().ok();
        mappings
            .as_mut()
            .map(|m| m.remove(&key).is_some())
            .unwrap_or(false)
    }

    pub fn cleanup_stale(&self, max_age_secs: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut mappings = match self.mappings.write() {
            Ok(m) => m,
            Err(_) => return,
        };

        mappings.retain(|_, entry| now - entry.last_active < max_age_secs);
    }

    fn make_key(ip: u32, port: u16, protocol: Protocol) -> u64 {
        let proto_bits = match protocol {
            Protocol::TCP => 0u64,
            Protocol::UDP => 1u64,
        };
        ((ip as u64) << 16) | (port as u64) | (proto_bits << 32)
    }
}

impl Default for NatManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn ip_to_u32(ip: &str) -> Option<u32> {
    let parts: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();

    if parts.len() != 4 {
        return None;
    }

    Some(
        ((parts[0] as u32) << 24)
            | ((parts[1] as u32) << 16)
            | ((parts[2] as u32) << 8)
            | (parts[3] as u32),
    )
}

pub fn u32_to_ip(addr: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (addr >> 24) & 0xFF,
        (addr >> 16) & 0xFF,
        (addr >> 8) & 0xFF,
        addr & 0xFF
    )
}

pub fn parse_ipv4_header(packet: &[u8]) -> Option<IpHeader> {
    if packet.len() < 20 {
        return None;
    }

    let version = (packet[0] >> 4) & 0xF;
    if version != 4 {
        return None;
    }

    let header_len = ((packet[0] & 0xF) as usize) * 4;
    if packet.len() < header_len {
        return None;
    }

    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    let protocol = packet[9];
    let src_ip = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
    let dst_ip = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);

    Some(IpHeader {
        header_len,
        total_len,
        protocol,
        src_ip,
        dst_ip,
    })
}

pub struct IpHeader {
    pub header_len: usize,
    pub total_len: u16,
    pub protocol: u8,
    pub src_ip: u32,
    pub dst_ip: u32,
}

pub fn parse_tcp_header(packet: &[u8], ip_header: &IpHeader) -> Option<TcpHeader> {
    let offset = ip_header.header_len;
    if packet.len() < offset + 20 {
        return None;
    }

    let src_port = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
    let dst_port = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
    let seq = u32::from_be_bytes([
        packet[offset + 4],
        packet[offset + 5],
        packet[offset + 6],
        packet[offset + 7],
    ]);
    let ack = u32::from_be_bytes([
        packet[offset + 8],
        packet[offset + 9],
        packet[offset + 10],
        packet[offset + 11],
    ]);
    let data_offset = ((packet[offset + 12] >> 4) as usize) * 4;
    let flags = packet[offset + 13];

    Some(TcpHeader {
        src_port,
        dst_port,
        seq,
        ack,
        data_offset,
        flags,
    })
}

pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub data_offset: usize,
    pub flags: u8,
}

pub fn parse_udp_header(packet: &[u8], ip_header: &IpHeader) -> Option<UdpHeader> {
    let offset = ip_header.header_len;
    if packet.len() < offset + 8 {
        return None;
    }

    let src_port = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
    let dst_port = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
    let len = u16::from_be_bytes([packet[offset + 4], packet[offset + 5]]);

    Some(UdpHeader {
        src_port,
        dst_port,
        len,
    })
}

pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub len: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_conversion() {
        let ip = "192.168.1.1";
        let addr = ip_to_u32(ip).unwrap();
        assert_eq!(u32_to_ip(addr), ip);
    }

    #[test]
    fn test_nat_key() {
        let manager = NatManager::new();
        let ip = ip_to_u32("10.0.0.1").unwrap();

        let entry = manager
            .create_mapping(
                ip,
                12345,
                ip_to_u32("1.2.3.4").unwrap(),
                Protocol::TCP,
                ip_to_u32("8.8.8.8").unwrap(),
                80,
            )
            .unwrap();

        assert_eq!(entry.external_port, 40000);
    }
}
