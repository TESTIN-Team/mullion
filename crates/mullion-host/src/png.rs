//! Minimal dependency-free PNG writer for RGBA8 images.
//!
//! Uses zlib "stored" (uncompressed) deflate blocks: no compression code,
//! just correct framing, Adler-32 and CRC-32. Output is a valid PNG that
//! every decoder accepts; files are larger than compressed PNGs, which is
//! fine for smoke receipts.

fn crc32(data: &[u8]) -> u32 {
    // IEEE CRC-32, bitwise (tables are overkill for receipts).
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn push_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc_input);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 16);
    out.push(0x78); // CMF: deflate, 32K window
    out.push(0x01); // FLG: no dict, fastest
    let mut chunks = raw.chunks(65_535).peekable();
    if raw.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    }
    while let Some(chunk) = chunks.next() {
        let last = chunks.peek().is_none();
        out.push(if last { 1 } else { 0 });
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// Encode an RGBA8 image (w*h*4 bytes) as PNG.
pub fn encode_rgba(w: u32, h: u32, px: &[u8]) -> Vec<u8> {
    assert_eq!(
        px.len(),
        (w as usize) * (h as usize) * 4,
        "pixel buffer size mismatch"
    );
    let mut out = Vec::with_capacity(px.len() + px.len() / 32 + 128);
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    push_chunk(&mut out, b"IHDR", &ihdr);

    // Raw scanlines: filter byte 0 + row bytes.
    let stride = (w as usize) * 4;
    let mut raw = Vec::with_capacity((stride + 1) * h as usize);
    for row in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&px[row * stride..(row + 1) * stride]);
    }
    push_chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    push_chunk(&mut out, b"IEND", &[]);
    out
}

/// Write a PNG file (best-effort for receipts; not a general IO layer).
pub fn write_png(path: &std::path::Path, w: u32, h: u32, px: &[u8]) -> std::io::Result<()> {
    let data = encode_rgba(w, h, px);
    std::fs::write(path, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    #[test]
    fn adler32_known_vector() {
        // "Wikipedia" -> 0x11E60398
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn png_structure() {
        let px = vec![255u8; 2 * 3 * 4];
        let png = encode_rgba(2, 3, &px);
        // Signature.
        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        // IHDR length and fields.
        assert_eq!(&png[8..12], &13u32.to_be_bytes());
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &2u32.to_be_bytes());
        assert_eq!(&png[20..24], &3u32.to_be_bytes());
        assert_eq!(png[24], 8);
        assert_eq!(png[25], 6);
        // IHDR CRC verified by construction; roundtrip zlib framing.
        let raw = vec![7u8; 70_000];
        let z = zlib_stored(&raw);
        assert_eq!(z[0], 0x78);
        // First block: not final (BFINAL=0, stored), LEN = 65535, NLEN = !LEN.
        assert_eq!(z[2], 0x00);
        assert_eq!(z[3], 0xFF);
        assert_eq!(z[4], 0xFF);
        assert_eq!(z[5], 0x00);
        assert_eq!(z[6], 0x00);
    }

    #[test]
    fn zlib_stored_roundtrip_length() {
        let raw = b"hello world".to_vec();
        let z = zlib_stored(&raw);
        // header(2) + block(5+11) + adler(4)
        assert_eq!(z.len(), 2 + 5 + 11 + 4);
        assert_eq!(&z[7..18], b"hello world");
        assert_eq!(&z[18..22], &adler32(&raw).to_be_bytes());
    }
}
