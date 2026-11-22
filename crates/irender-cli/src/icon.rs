//! An app icon made from a letter or two: a plain square with the letters drawn on a grid,
//! written as a PNG without libraries (stored, not compressed).

/// 5 by 7, a row a byte, the left bit first.
const FONT: &[(char, [u8; 7])] = &[
    ('A', [14, 17, 17, 31, 17, 17, 17]), ('B', [30, 17, 17, 30, 17, 17, 30]), ('C', [14, 17, 16, 16, 16, 17, 14]), ('D', [30, 17, 17, 17, 17, 17, 30]),
    ('E', [31, 16, 16, 30, 16, 16, 31]), ('F', [31, 16, 16, 30, 16, 16, 16]), ('G', [14, 17, 16, 23, 17, 17, 15]), ('H', [17, 17, 17, 31, 17, 17, 17]),
    ('I', [14, 4, 4, 4, 4, 4, 14]), ('J', [7, 2, 2, 2, 2, 18, 12]), ('K', [17, 18, 20, 24, 20, 18, 17]), ('L', [16, 16, 16, 16, 16, 16, 31]),
    ('M', [17, 27, 21, 21, 17, 17, 17]), ('N', [17, 25, 21, 19, 17, 17, 17]), ('O', [14, 17, 17, 17, 17, 17, 14]), ('P', [30, 17, 17, 30, 16, 16, 16]),
    ('Q', [14, 17, 17, 17, 21, 18, 13]), ('R', [30, 17, 17, 30, 20, 18, 17]), ('S', [15, 16, 16, 14, 1, 1, 30]), ('T', [31, 4, 4, 4, 4, 4, 4]),
    ('U', [17, 17, 17, 17, 17, 17, 14]), ('V', [17, 17, 17, 17, 17, 10, 4]), ('W', [17, 17, 17, 21, 21, 21, 10]), ('X', [17, 17, 10, 4, 10, 17, 17]),
    ('Y', [17, 17, 10, 4, 4, 4, 4]), ('Z', [31, 1, 2, 4, 8, 16, 31]), ('0', [14, 17, 19, 21, 25, 17, 14]), ('1', [4, 12, 4, 4, 4, 4, 14]),
    ('2', [14, 17, 1, 2, 4, 8, 31]), ('3', [31, 2, 4, 2, 1, 17, 14]), ('4', [2, 6, 10, 18, 31, 2, 2]), ('5', [31, 16, 30, 1, 1, 17, 14]),
    ('6', [6, 8, 16, 30, 17, 17, 14]), ('7', [31, 1, 2, 4, 8, 8, 8]), ('8', [14, 17, 17, 14, 17, 17, 14]), ('9', [14, 17, 17, 15, 1, 2, 12]),
];

fn rgb(hex: &str) -> [u8; 3] {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    [(n >> 16) as u8, (n >> 8) as u8, n as u8]
}

/// A square PNG of `size`. `part` is how much of its width the letters take at most.
pub fn png(text: &str, color: &str, background: &str, size: u32, part: f64) -> Vec<u8> {
    let (fg, bg) = (rgb(color), rgb(background));
    let glyphs: Vec<[u8; 7]> = text.to_uppercase().chars().filter_map(|c| FONT.iter().find(|(f, _)| *f == c).map(|(_, g)| *g)).collect();
    let cols = (glyphs.len() as u32 * 6).saturating_sub(1).max(1);
    // Whole pixels per dot, so the letters' edges are sharp.
    let dot = ((size as f64 * part / cols as f64).min(size as f64 * part * 0.8 / 7.0)).floor().max(1.0) as u32;
    let (x0, y0) = ((size - (cols * dot).min(size)) / 2, (size - (7 * dot).min(size)) / 2);
    let mut raw = Vec::with_capacity((size * (size * 3 + 1)) as usize);
    for y in 0..size {
        raw.push(0); // no filter
        for x in 0..size {
            let on = x >= x0 && y >= y0 && {
                let (cx, cy) = ((x - x0) / dot, (y - y0) / dot);
                cy < 7 && cx < cols && cx % 6 < 5 && glyphs.get((cx / 6) as usize).is_some_and(|g| g[cy as usize] >> (4 - cx % 6) & 1 == 1)
            };
            raw.extend_from_slice(if on { &fg } else { &bg });
        }
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut head = Vec::new();
    head.extend_from_slice(&size.to_be_bytes());
    head.extend_from_slice(&size.to_be_bytes());
    head.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB
    chunk(&mut out, b"IHDR", &head);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let at = out.len();
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let crc = crc32(&out[at..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |mut c, b| {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        c
    })
}

/// Run lengths as fixed-code deflate: a plain icon is rows of the same few pixels, so
/// copying from three bytes back (a pixel) or a row back makes it small.
fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut bits = Bits { out: vec![0x78, 0x01], bit: 0, acc: 0 };
    bits.put(1, 1); // the last block
    bits.put(1, 2); // fixed codes
    let mut i = 0;
    while i < raw.len() {
        let mut best = (0usize, 0usize);
        for dist in [3usize, row(raw.len())] {
            if dist > 0 && i >= dist && dist <= 32768 {
                let n = (0..258).take_while(|k| i + k < raw.len() && raw[i + k] == raw[i + k - dist]).count();
                if n > best.0 {
                    best = (n, dist);
                }
            }
        }
        if best.0 >= 4 {
            bits.length(best.0 as u32);
            bits.distance(best.1 as u32);
            i += best.0;
        } else {
            bits.symbol(raw[i] as u32);
            i += 1;
        }
    }
    bits.symbol(256);
    let mut out = bits.finish();
    let (mut a, mut b) = (1u32, 0u32);
    for x in raw {
        a = (a + *x as u32) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&(b << 16 | a).to_be_bytes());
    out
}

/// The bytes of a row, when the picture is a square of RGB pixels with a byte before each.
fn row(len: usize) -> usize {
    let mut n = ((len as f64) / 3.0).sqrt() as usize;
    while n > 0 && n * (n * 3 + 1) > len {
        n -= 1;
    }
    if n * (n * 3 + 1) == len { n * 3 + 1 } else { 0 }
}

struct Bits {
    out: Vec<u8>,
    bit: u32,
    acc: u32,
}

impl Bits {
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= v << self.bit;
        self.bit += n;
        while self.bit >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.bit -= 8;
        }
    }
    /// Huffman codes go highest bit first.
    fn code(&mut self, v: u32, n: u32) {
        self.put(v.reverse_bits() >> (32 - n), n);
    }
    fn symbol(&mut self, s: u32) {
        match s {
            0..=143 => self.code(0x30 + s, 8),
            144..=255 => self.code(0x190 + s - 144, 9),
            256..=279 => self.code(s - 256, 7),
            _ => self.code(0xc0 + s - 280, 8),
        }
    }
    fn length(&mut self, n: u32) {
        const BASE: [u32; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
        const EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
        let i = BASE.iter().rposition(|b| *b <= n).unwrap_or(0);
        self.symbol(257 + i as u32);
        self.put(n - BASE[i], EXTRA[i]);
    }
    fn distance(&mut self, d: u32) {
        const BASE: [u32; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
        let i = BASE.iter().rposition(|b| *b <= d).unwrap_or(0);
        self.code(i as u32, 5);
        self.put(d - BASE[i], if i < 2 { 0 } else { (i as u32 - 2) / 2 });
    }
    fn finish(mut self) -> Vec<u8> {
        if self.bit > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}
