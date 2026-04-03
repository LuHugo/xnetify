use std::io::{Cursor, Read};

pub struct SniExtractor;

impl SniExtractor {
    pub fn extract_sni(tls_data: &[u8]) -> Option<String> {
        let mut cursor = Cursor::new(tls_data);

        if let Ok(content_type) = read_u8(&mut cursor) {
            if content_type != 0x16 {
                return None;
            }
        } else {
            return None;
        }

        skip_bytes(&mut cursor, 2).ok()?;
        skip_bytes(&mut cursor, 32).ok()?;

        let session_id_len = read_u8(&mut cursor).ok()?;
        skip_bytes(&mut cursor, session_id_len as usize).ok()?;

        let cipher_suites_len = read_u16_be(&mut cursor).ok()?;
        skip_bytes(&mut cursor, cipher_suites_len as usize).ok()?;

        let compression_methods_len = read_u8(&mut cursor).ok()?;
        skip_bytes(&mut cursor, compression_methods_len as usize).ok()?;

        read_u16_be(&mut cursor).ok()?;

        loop {
            let extension_type = match read_u16_be(&mut cursor) {
                Ok(t) => t,
                Err(_) => break,
            };

            let extension_len = read_u16_be(&mut cursor).ok()? as usize;

            if extension_type == 0x0000 {
                let server_name = Self::parse_server_name_list(&mut cursor, extension_len);
                if let Some(name) = server_name {
                    return Some(name);
                }
            } else {
                skip_bytes(&mut cursor, extension_len).ok()?;
            }
        }

        None
    }

    fn parse_server_name_list(cursor: &mut Cursor<&[u8]>, max_len: usize) -> Option<String> {
        let start_pos = cursor.position() as usize;
        let end_pos = start_pos + max_len;

        read_u8(cursor).ok()?;
        if read_u8(cursor).ok()? != 0x00 {
            return None;
        }

        let name_len = read_u16_be(cursor).ok()? as usize;
        if name_len == 0 || cursor.position() as usize + name_len > end_pos {
            return None;
        }

        let name_bytes = read_exact_bytes(cursor, name_len).ok()?;
        String::from_utf8(name_bytes.to_vec()).ok()
    }
}

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, std::io::Error> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16_be(cursor: &mut Cursor<&[u8]>) -> Result<u16, std::io::Error> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_exact_bytes(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>, std::io::Error> {
    let pos = cursor.position() as usize;
    let data = cursor.get_ref();
    if pos + len > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF",
        ));
    }
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf)?;
    Ok(buf)
}

fn skip_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<(), std::io::Error> {
    let new_pos = cursor.position().saturating_add(count as u64);
    if new_pos as usize <= cursor.get_ref().len() {
        cursor.set_position(new_pos);
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF",
        ))
    }
}

pub struct DnsParser;

impl DnsParser {
    pub fn extract_domain(dns_packet: &[u8]) -> Option<String> {
        if dns_packet.len() < 12 {
            return None;
        }

        let flags = u16::from_be_bytes([dns_packet[2], dns_packet[3]]);
        let qr = (flags >> 15) & 0x01;
        if qr != 0 {
            return None;
        }

        let qdcount = u16::from_be_bytes([dns_packet[4], dns_packet[5]]);
        if qdcount == 0 {
            return None;
        }

        let mut pos = 12;
        let mut labels: Vec<Vec<u8>> = Vec::new();

        loop {
            if pos >= dns_packet.len() {
                return None;
            }

            let len = dns_packet[pos] as usize;
            if len == 0 {
                break;
            }

            if len >= 0xC0 {
                break;
            }

            pos += 1;
            if pos + len > dns_packet.len() {
                return None;
            }

            labels.push(dns_packet[pos..pos + len].to_vec());
            pos += len;
        }

        if labels.is_empty() {
            return None;
        }

        let domain: String = labels
            .iter()
            .map(|l| String::from_utf8_lossy(l).to_string())
            .collect::<Vec<_>>()
            .join(".");

        Some(domain)
    }

    pub fn build_blocked_response(original: &[u8]) -> Option<Vec<u8>> {
        if original.len() < 12 {
            return None;
        }

        let mut response = original.to_vec();

        response[2] |= 0x80;
        response[3] = 0x83;

        let ancount = [0x00, 0x01];
        response.splice(6..8, ancount);

        let mut additional = Vec::new();
        additional.push(0xC0);
        additional.push(0x0C);

        additional.push(0x00);
        additional.push(0x01);
        additional.push(0x00);
        additional.push(0x01);
        additional.extend_from_slice(&[0x00, 0x00]);
        additional.extend_from_slice(&[0x00, 0x78]);
        additional.extend_from_slice(&[0x00, 0x04]);
        additional.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        response.extend_from_slice(&additional);

        Some(response)
    }
}
