//! Static Huffman coding for DCT coefficients.
//!
//! Uses the standard JPEG Huffman tables (ISO 10918-1 Annex K):
//!   - Luma DC + full luminance AC for Y (and A); chroma DC for Cb/Cr; **same luminance AC** for
//!     all AC planes so .bg v4/v5 matches historical bitgrain streams.
//!
//! The bitstream is packed MSB-first, continuous across all blocks in a plane.
//! 0xFF bytes are stuffed as 0xFF 0x00 (JPEG convention).
//! A single flush (pad with 1s) is written at the end of each plane.

use crate::block::Block;
use crate::zigzag::ZIGZAG;

// ---------------------------------------------------------------------------
// Standard JPEG Huffman tables (ISO 10918-1 Annex K)
// Each entry: (code_length_bits, code_value)
// ---------------------------------------------------------------------------

static LUMA_DC_TABLE: &[(u8, u16)] = &[
    (2, 0b00),
    (3, 0b010),
    (3, 0b011),
    (3, 0b100),
    (3, 0b101),
    (3, 0b110),
    (4, 0b1110),
    (5, 0b11110),
    (6, 0b111110),
    (7, 0b1111110),
    (8, 0b11111110),
    (9, 0b111111110),
];

static CHROMA_DC_TABLE: &[(u8, u16)] = &[
    (2, 0b00),
    (2, 0b01),
    (2, 0b10),
    (3, 0b110),
    (4, 0b1110),
    (5, 0b11110),
    (6, 0b111110),
    (7, 0b1111110),
    (8, 0b11111110),
    (9, 0b111111110),
    (10,0b1111111110),
    (11,0b11111111110),
];

/// AC table indexed by symbol byte: high nibble = run (0–15), low nibble = category (1–10).
/// Symbol 0x00 = EOB, 0xF0 = ZRL. Entry (0,0) means "not in table".
type AcTable = [(u8, u16); 256];

use crate::jpeg_luma_ac_ht::JPEG_LUMA_AC_HT;
use std::sync::OnceLock;

/// stb_image_write / ISO 10918-1 Annex K luminance AC Huffman (full 256-entry encode table).
fn ac_table_from_stb_ht(ht: &[[u16; 2]; 256]) -> AcTable {
    let mut t = [(0u8, 0u16); 256];
    for i in 0..256 {
        let code = ht[i][0];
        let len = ht[i][1];
        if len > 0 {
            t[i] = (len as u8, code);
        }
    }
    t
}

/// Full JPEG luminance AC table. Used for **every** plane in .bg v4/v5 (Y, Cb, Cr, A) so the
/// on-wire format stays compatible with earlier bitgrain, which always applied the luma AC spec
/// to chroma too (not the separate chrominance AC table).
static JPEG_AC_TABLE: OnceLock<AcTable> = OnceLock::new();
static JPEG_CHROMA_AC_TABLE: OnceLock<AcTable> = OnceLock::new();

fn jpeg_ac_table() -> &'static AcTable {
    JPEG_AC_TABLE.get_or_init(|| ac_table_from_stb_ht(&JPEG_LUMA_AC_HT))
}

fn ac_table_from_canonical(nrcodes: &[u8; 17], values: &[u8]) -> AcTable {
    let mut t = [(0u8, 0u16); 256];
    let mut code: u16 = 0;
    let mut k = 0usize;
    for len in 1..=16usize {
        let n = nrcodes[len] as usize;
        for _ in 0..n {
            let sym = values[k] as usize;
            t[sym] = (len as u8, code);
            code = code.wrapping_add(1);
            k += 1;
        }
        code <<= 1;
    }
    t
}

fn jpeg_chroma_ac_table() -> &'static AcTable {
    JPEG_CHROMA_AC_TABLE.get_or_init(|| {
        // JPEG Annex K / stb_image_write chrominance AC Huffman table.
        const NR_CODES: [u8; 17] = [0, 0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
        const VALUES: [u8; 162] = [
            0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21,
            0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
            0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91,
            0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
            0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34,
            0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
            0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
            0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
            0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
            0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
            0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
            0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96,
            0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
            0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
            0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
            0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2,
            0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
            0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9,
            0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
            0xf9, 0xfa,
        ];
        ac_table_from_canonical(&NR_CODES, &VALUES)
    })
}

#[derive(Clone, Copy)]
struct DecodeNode {
    child: [i16; 2],
    sym: i16,
}

impl DecodeNode {
    #[inline]
    fn new() -> Self {
        Self { child: [-1, -1], sym: -1 }
    }
}

struct DecodeTree {
    nodes: Vec<DecodeNode>,
}

impl DecodeTree {
    fn with_root() -> Self {
        Self { nodes: vec![DecodeNode::new()] }
    }

    fn insert(&mut self, code: u16, len: u8, sym: u8) -> bool {
        if len == 0 || len > 16 {
            return false;
        }
        let mut idx = 0usize;
        for shift in (0..len).rev() {
            // A leaf cannot be extended.
            if self.nodes[idx].sym >= 0 {
                return false;
            }
            let bit = ((code >> shift) & 1) as usize;
            let next = self.nodes[idx].child[bit];
            let next_idx = if next >= 0 {
                next as usize
            } else {
                self.nodes.push(DecodeNode::new());
                let ni = (self.nodes.len() - 1) as i16;
                self.nodes[idx].child[bit] = ni;
                ni as usize
            };
            idx = next_idx;
        }
        // A node with children cannot become a leaf; an existing leaf cannot be overwritten.
        if self.nodes[idx].sym >= 0 || self.nodes[idx].child[0] >= 0 || self.nodes[idx].child[1] >= 0 {
            return false;
        }
        self.nodes[idx].sym = sym as i16;
        true
    }
}

static LUMA_DC_TREE: OnceLock<DecodeTree> = OnceLock::new();
static CHROMA_DC_TREE: OnceLock<DecodeTree> = OnceLock::new();
static AC_TREE_LUMA: OnceLock<DecodeTree> = OnceLock::new();
static AC_TREE_CHROMA: OnceLock<DecodeTree> = OnceLock::new();

fn build_dc_tree(table: &[(u8, u16)]) -> DecodeTree {
    let mut t = DecodeTree::with_root();
    for (sym, &(len, code)) in table.iter().enumerate() {
        assert!(t.insert(code, len, sym as u8), "invalid DC Huffman table");
    }
    t
}

fn build_ac_tree(table: &AcTable) -> DecodeTree {
    let mut t = DecodeTree::with_root();
    for (sym, &(len, code)) in table.iter().enumerate() {
        if len > 0 {
            assert!(t.insert(code, len, sym as u8), "invalid AC Huffman table");
        }
    }
    t
}

#[inline]
fn luma_dc_tree() -> &'static DecodeTree {
    LUMA_DC_TREE.get_or_init(|| build_dc_tree(LUMA_DC_TABLE))
}

#[inline]
fn chroma_dc_tree() -> &'static DecodeTree {
    CHROMA_DC_TREE.get_or_init(|| build_dc_tree(CHROMA_DC_TABLE))
}

#[inline]
fn ac_tree(use_chroma_ac: bool) -> &'static DecodeTree {
    if use_chroma_ac {
        AC_TREE_CHROMA.get_or_init(|| build_ac_tree(jpeg_chroma_ac_table()))
    } else {
        AC_TREE_LUMA.get_or_init(|| build_ac_tree(jpeg_ac_table()))
    }
}

// ---------------------------------------------------------------------------
// Magnitude helpers
// ---------------------------------------------------------------------------

#[inline]
fn category(v: i16) -> u8 {
    if v == 0 { return 0; }
    (16 - v.unsigned_abs().leading_zeros()) as u8
}

#[inline]
fn magnitude_bits(v: i16, cat: u8) -> u16 {
    if v >= 0 { v as u16 } else { ((1u16 << cat) - 1).wrapping_add(v as u16) }
}

#[inline]
fn magnitude_decode(bits: u16, cat: u8) -> i16 {
    let threshold = 1u16 << (cat - 1);
    if bits >= threshold { bits as i16 } else { bits as i16 - (1i16 << cat) + 1 }
}

/// JPEG-style DC/AC Huffman tables only cover DC category ≤11 and AC category ≤10.
/// Without this clamp, `encode_plane` can hit the `al == 0` fallback and emit a bitstream
/// the decoder cannot parse (decode returns `None` / CLI reports corrupt .bg).
pub(crate) fn clamp_block_jpeg_coeffs(block: &mut Block) {
    const DC_MAX: i16 = 2047; // category 11 magnitude
    const AC_MAX: i16 = 1023; // category 10 magnitude
    block.data[ZIGZAG[0]] = block.data[ZIGZAG[0]].clamp(-DC_MAX, DC_MAX);
    for i in 1..64 {
        let z = ZIGZAG[i];
        block.data[z] = block.data[z].clamp(-AC_MAX, AC_MAX);
    }
}

// ---------------------------------------------------------------------------
// Bit writer — continuous across blocks, flush once per plane
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub buf:     Vec<u8>,
    bit_buf:     u64,
    bits_in:     u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(4096), bit_buf: 0, bits_in: 0 }
    }

    #[inline]
    fn push_entropy_byte(&mut self, byte: u8) {
        self.buf.push(byte);
        if byte == 0xFF {
            self.buf.push(0x00);
        }
    }

    #[inline]
    pub fn write_bits(&mut self, code: u16, n: u8) {
        if n == 0 { return; }
        let mask = (1u64 << n) - 1;
        self.bit_buf = (self.bit_buf << n) | ((code as u64) & mask);
        self.bits_in += n;

        while self.bits_in >= 8 {
            let shift = self.bits_in - 8;
            let byte = (self.bit_buf >> shift) as u8;
            self.push_entropy_byte(byte);
            self.bits_in -= 8;
            if self.bits_in == 0 {
                self.bit_buf = 0;
            } else {
                self.bit_buf &= (1u64 << self.bits_in) - 1;
            }
        }
    }

    /// Flush remaining bits, padding with 1s (JPEG convention). Call once per plane.
    pub fn flush(&mut self) {
        if self.bits_in > 0 {
            let pad = 8 - self.bits_in;
            self.write_bits((1u16 << pad) - 1, pad);
        }
    }
}

// ---------------------------------------------------------------------------
// Bit reader — continuous across blocks
// ---------------------------------------------------------------------------

pub struct BitReader<'a> {
    buf:     &'a [u8],
    pos:     usize,
    bit_buf: u64,
    bits_in: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(buf: &'a [u8], start: usize) -> Self {
        Self { buf, pos: start, bit_buf: 0, bits_in: 0 }
    }

    #[inline]
    fn refill(&mut self) {
        while self.bits_in <= 56 && self.pos < self.buf.len() {
            let byte = self.buf[self.pos];
            self.pos += 1;
            // JPEG byte-unstuffing
            if byte == 0xFF && self.pos < self.buf.len() && self.buf[self.pos] == 0x00 {
                self.pos += 1;
            }
            self.bit_buf = (self.bit_buf << 8) | byte as u64;
            self.bits_in += 8;
        }
    }

    #[inline]
    pub fn read_bits(&mut self, n: u8) -> Option<u16> {
        if n == 0 { return Some(0); }
        if n == 1 {
            return self.read_one_bit().map(|b| b as u16);
        }
        self.refill();
        if self.bits_in < n {
            return None;
        }
        let shift = self.bits_in - n;
        let out = ((self.bit_buf >> shift) & ((1u64 << n) - 1)) as u16;
        self.bits_in -= n;
        if self.bits_in == 0 {
            self.bit_buf = 0;
        } else {
            self.bit_buf &= (1u64 << self.bits_in) - 1;
        }
        Some(out)
    }

    #[inline]
    pub fn read_one_bit(&mut self) -> Option<u8> {
        if self.bits_in == 0 {
            self.refill();
            if self.bits_in == 0 {
                return None;
            }
        }
        self.bits_in -= 1;
        let bit = ((self.bit_buf >> self.bits_in) & 1) as u8;
        if self.bits_in == 0 {
            self.bit_buf = 0;
        }
        Some(bit)
    }

    pub fn byte_position(&self) -> usize { self.pos }
}

// ---------------------------------------------------------------------------
// Huffman symbol decode via prebuilt binary tree
// ---------------------------------------------------------------------------

fn decode_sym(reader: &mut BitReader, tree: &DecodeTree) -> Option<u8> {
    let mut idx = 0usize;
    for _ in 0..16 {
        let bit = reader.read_one_bit()? as usize;
        let next = tree.nodes[idx].child[bit];
        if next < 0 {
            return None;
        }
        idx = next as usize;
        let sym = tree.nodes[idx].sym;
        if sym >= 0 {
            return Some(sym as u8);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Encode / decode a full plane of blocks
// ---------------------------------------------------------------------------

/// Encode all blocks of a plane into a Vec<u8> prefixed with its byte length (u32 LE).
/// Format: [len: u32 LE][bitstream bytes]
/// The length prefix allows the decoder to skip exactly to the next plane boundary.
pub fn encode_plane(blocks: &[Block], is_chroma: bool) -> Vec<u8> {
    encode_plane_with_profile(blocks, is_chroma, false, false)
}

pub fn encode_plane_with_ac(blocks: &[Block], is_chroma: bool, use_chroma_ac: bool) -> Vec<u8> {
    encode_plane_with_profile(blocks, is_chroma, use_chroma_ac, false)
}

pub fn encode_plane_with_profile(
    blocks: &[Block],
    is_chroma: bool,
    use_chroma_ac: bool,
    use_dc_delta: bool,
) -> Vec<u8> {
    let dc_table = if is_chroma { CHROMA_DC_TABLE } else { LUMA_DC_TABLE };
    let ac_table = if use_chroma_ac { jpeg_chroma_ac_table() } else { jpeg_ac_table() };
    let mut w = BitWriter::new();
    let mut prev_dc: i16 = 0;

    for block in blocks {
        // DC
        let dc_val = block.data[ZIGZAG[0]];
        let dc_emit = if use_dc_delta {
            let d = dc_val.wrapping_sub(prev_dc);
            prev_dc = dc_val;
            d
        } else {
            dc_val
        };
        let dc_cat = category(dc_emit);
        let (dc_len, dc_code) = dc_table[dc_cat as usize];
        w.write_bits(dc_code, dc_len);
        if dc_cat > 0 { w.write_bits(magnitude_bits(dc_emit, dc_cat), dc_cat); }

        // AC
        let mut zero_run: u8 = 0;
        for i in 1..64 {
            let val = block.data[ZIGZAG[i]];
            if val == 0 {
                zero_run += 1;
                if zero_run == 16 {
                    let (zl, zc) = ac_table[0xF0]; w.write_bits(zc, zl);
                    zero_run = 0;
                }
            } else {
                let cat = category(val);
                let sym = (zero_run << 4) | cat;
                let (al, ac) = ac_table[sym as usize];
                assert!(al > 0, "missing AC Huffman for RS {sym:#04x} (run={zero_run} cat={cat})");
                w.write_bits(ac, al);
                w.write_bits(magnitude_bits(val, cat), cat);
                zero_run = 0;
            }
        }
        let (el, ec) = ac_table[0x00]; w.write_bits(ec, el);
    }
    w.flush();

    // Prepend 4-byte length so decoder can skip exactly to next plane
    let data = w.buf;
    let len = data.len() as u32;
    let mut out = Vec::with_capacity(4 + data.len());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// Decode a plane of `n_blocks` blocks from `buf[start..]`.
/// Reads the 4-byte length prefix, then decodes exactly that many bytes.
/// Returns (blocks, new_byte_position) or None on error.
pub fn decode_plane(buf: &[u8], start: usize, n_blocks: usize, is_chroma: bool) -> Option<(Vec<Block>, usize)> {
    decode_plane_with_profile(buf, start, n_blocks, is_chroma, false, false)
}

pub fn decode_plane_with_ac(
    buf: &[u8],
    start: usize,
    n_blocks: usize,
    is_chroma: bool,
    use_chroma_ac: bool,
) -> Option<(Vec<Block>, usize)> {
    decode_plane_with_profile(buf, start, n_blocks, is_chroma, use_chroma_ac, false)
}

pub fn decode_plane_with_profile(
    buf: &[u8],
    start: usize,
    n_blocks: usize,
    is_chroma: bool,
    use_chroma_ac: bool,
    use_dc_delta: bool,
) -> Option<(Vec<Block>, usize)> {
    if start + 4 > buf.len() { return None; }
    let plane_len = u32::from_le_bytes(buf[start..start+4].try_into().unwrap()) as usize;
    let data_start = start + 4;
    let data_end   = data_start + plane_len;
    if data_end > buf.len() {
        return None;
    }

    let dc_tree = if is_chroma { chroma_dc_tree() } else { luma_dc_tree() };
    let ac_tree = ac_tree(use_chroma_ac);
    let data = &buf[data_start..data_end];

    let mut reader = BitReader::new(data, 0);
    let blocks = decode_plane_blocks(&mut reader, n_blocks, dc_tree, ac_tree, use_dc_delta)?;

    // Return data_end as the next byte position (exact plane boundary)
    Some((blocks, data_end))
}

fn decode_plane_blocks(
    reader: &mut BitReader,
    n_blocks: usize,
    dc_tree: &DecodeTree,
    ac_tree: &DecodeTree,
    use_dc_delta: bool,
) -> Option<Vec<Block>> {
    let mut blocks = Vec::with_capacity(n_blocks);
    let mut prev_dc: i16 = 0;
    for _bi in 0..n_blocks {
        let mut block = Block::new();

        // DC
        let dc_cat = match decode_sym(reader, dc_tree) {
            Some(c) => c,
            None => return None,
        };
        let dc_diff = if dc_cat == 0 {
            0i16
        } else {
            let bits = match reader.read_bits(dc_cat) {
                Some(b) => b,
                None => return None,
            };
            magnitude_decode(bits, dc_cat)
        };
        let dc_val = if use_dc_delta {
            let v = prev_dc.wrapping_add(dc_diff);
            prev_dc = v;
            v
        } else {
            dc_diff
        };
        block.data[ZIGZAG[0]] = dc_val;

        // AC
        let mut ac_idx = 1usize;
        loop {
            let sym = match decode_sym(reader, ac_tree) {
                Some(s) => s,
                None => return None,
            };
            if sym == 0x00 { break; } // EOB
            if sym == 0xF0 {
                // ZRL: exactly 16 consecutive zeros.
                if ac_idx + 16 > 64 { return None; }
                ac_idx += 16;
                continue;
            }
            let run = (sym >> 4) as usize;
            let cat = sym & 0x0F;
            // Need room for run zeros plus one non-zero coefficient.
            if ac_idx + run >= 64 { return None; }
            ac_idx += run;
            let bits = match reader.read_bits(cat) {
                Some(b) => b,
                None => return None,
            };
            block.data[ZIGZAG[ac_idx]] = magnitude_decode(bits, cat);
            ac_idx += 1;
            if ac_idx > 64 { return None; }
        }

        blocks.push(block);
    }

    Some(blocks)
}

