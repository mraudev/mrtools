// G.722 (64 kbit/s) – Breitband-Sprachcodec, 16 kHz Audio, im RTP mit 8-kHz-Zeitmarke (RFC 3551).
// Übertragung von src/g722.js der Electron-Version (ITU-Referenz: Sub-Band-ADPCM mit QMF-Filterbank),
// bitgenau gleich (siehe Test). Ein 20-ms-Block sind 320 Samples und ergibt 160 Byte. Der Zustand läuft
// über die Blöcke hinweg, darum je Gespräch eine eigene Instanz.
// JavaScript rechnet Summen als Gleitkommazahl und kürzt erst beim Schieben auf 32 Bit (ToInt32) –
// deshalb Summen in i64 und `as i32` vor dem Schieben.

const QMF: [i32; 12] = [3, -11, 12, 32, -210, 951, 3876, -805, 362, -156, 53, -11];
const WL: [i32; 8] = [-60, -30, 58, 172, 334, 538, 1198, 3042];
const RL42: [usize; 16] = [0, 7, 6, 5, 4, 3, 2, 1, 7, 6, 5, 4, 3, 2, 1, 0];
const ILB: [i32; 32] = [
    2048, 2093, 2139, 2186, 2233, 2282, 2332, 2383, 2435, 2489, 2543, 2599, 2656, 2714, 2774, 2834,
    2896, 2960, 3025, 3091, 3158, 3228, 3298, 3371, 3444, 3520, 3597, 3676, 3756, 3838, 3922, 4008,
];
const WH: [i32; 3] = [0, -214, 798];
const RH2: [usize; 4] = [2, 1, 2, 1];
const QM2: [i32; 4] = [-7408, -1616, 7408, 1616];
const QM4: [i32; 16] = [
    0, -20456, -12896, -8968, -6288, -4240, -2584, -1200, 20456, 12896, 8968, 6288, 4240, 2584,
    1200, 0,
];
const QM6: [i32; 64] = [
    -136, -136, -136, -136, -24808, -21904, -19008, -16704, -14984, -13512, -12280, -11192, -10232,
    -9360, -8576, -7856, -7192, -6576, -6000, -5456, -4944, -4464, -4008, -3576, -3168, -2776,
    -2400, -2032, -1688, -1360, -1040, -728, 24808, 21904, 19008, 16704, 14984, 13512, 12280,
    11192, 10232, 9360, 8576, 7856, 7192, 6576, 6000, 5456, 4944, 4464, 4008, 3576, 3168, 2776,
    2400, 2032, 1688, 1360, 1040, 728, 432, 136, -432, -136,
];
const Q6: [i32; 32] = [
    0, 35, 72, 110, 150, 190, 233, 276, 323, 370, 422, 473, 530, 587, 650, 714, 786, 858, 940,
    1023, 1121, 1219, 1339, 1458, 1612, 1765, 1980, 2195, 2557, 2919, 0, 0,
];
const ILN: [i32; 32] = [
    0, 63, 62, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11,
    10, 9, 8, 7, 6, 5, 4, 0,
];
const ILP: [i32; 32] = [
    0, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39,
    38, 37, 36, 35, 34, 33, 32, 0,
];
const IHN: [i32; 3] = [0, 1, 0];
const IHP: [i32; 3] = [0, 3, 2];
const DEC_SHIFT: u32 = 11; // Ausgangsskalierung der Synthese-QMF (wie die Electron-Version)

fn sat(x: i32) -> i32 {
    x.clamp(-32768, 32767)
}

#[derive(Default, Clone)]
struct Band {
    s: i32,
    sp: i32,
    sz: i32,
    r: [i32; 3],
    a: [i32; 3],
    ap: [i32; 3],
    p: [i32; 3],
    d: [i32; 7],
    b: [i32; 7],
    bp: [i32; 7],
    sg: [i32; 7],
    nb: i32,
    det: i32,
}

fn new_band() -> Band {
    Band {
        det: 32,
        ..Default::default()
    }
}

// Adaptiver Prädiktor (gemeinsam für Encoder und Decoder), aktualisiert die Band-Filter mit dem Fehler dx.
fn block4(b: &mut Band, dx: i32) {
    b.d[0] = dx;
    b.r[0] = sat(b.s + dx);
    b.p[0] = sat(b.sz + dx);
    // UPPOL2
    for i in 0..3 {
        b.sg[i] = b.p[i] >> 15;
    }
    let mut wd1 = sat(b.a[1] << 2);
    let mut wd2 = if b.sg[0] == b.sg[1] { -wd1 } else { wd1 };
    if wd2 > 32767 {
        wd2 = 32767;
    }
    let mut wd3 =
        (if b.sg[0] == b.sg[2] { 128 } else { -128 }) + (wd2 >> 7) + ((b.a[2] * 32512) >> 15);
    wd3 = wd3.clamp(-12288, 12288);
    b.ap[2] = wd3;
    // UPPOL1
    b.sg[0] = b.p[0] >> 15;
    b.sg[1] = b.p[1] >> 15;
    wd1 = if b.sg[0] == b.sg[1] { 192 } else { -192 };
    wd2 = (b.a[1] * 32640) >> 15;
    b.ap[1] = sat(wd1 + wd2);
    wd3 = sat(15360 - b.ap[2]);
    if b.ap[1] > wd3 {
        b.ap[1] = wd3;
    } else if b.ap[1] < -wd3 {
        b.ap[1] = -wd3;
    }
    // UPZERO
    wd1 = if dx == 0 { 0 } else { 128 };
    b.sg[0] = dx >> 15;
    for i in 1..7 {
        b.sg[i] = b.d[i] >> 15;
        wd2 = if b.sg[i] == b.sg[0] { wd1 } else { -wd1 };
        wd3 = (b.b[i] * 32640) >> 15;
        b.bp[i] = sat(wd2 + wd3);
    }
    // DELAYA
    for i in (1..7).rev() {
        b.d[i] = b.d[i - 1];
        b.b[i] = b.bp[i];
    }
    for i in (1..3).rev() {
        b.r[i] = b.r[i - 1];
        b.p[i] = b.p[i - 1];
        b.a[i] = b.ap[i];
    }
    // FILTEP
    wd1 = sat(b.r[1] + b.r[1]);
    wd1 = (b.a[1] * wd1) >> 15;
    wd2 = sat(b.r[2] + b.r[2]);
    wd2 = (b.a[2] * wd2) >> 15;
    b.sp = sat(wd1 + wd2);
    // FILTEZ
    let mut sz = 0i32;
    for i in (1..7).rev() {
        wd1 = sat(b.d[i] + b.d[i]);
        sz += (b.b[i] * wd1) >> 15;
    }
    b.sz = sat(sz);
    // PREDIC
    b.s = sat(b.sp + b.sz);
}

fn scale(nb: i32, base: i32) -> i32 {
    let wd1 = ((nb >> 6) & 31) as usize;
    let wd2 = base - (nb >> 11);
    (if wd2 < 0 {
        ILB[wd1] << -wd2
    } else {
        ILB[wd1] >> wd2
    }) << 2
}

// QMF-Filterbank über die letzten 24 Werte: (Summe gerade, Summe ungerade) als JS-Doppelsumme.
fn qmf(x: &[i32; 24]) -> (i64, i64) {
    let (mut even, mut odd) = (0i64, 0i64);
    for i in 0..12 {
        odd += x[2 * i] as i64 * QMF[i] as i64;
        even += x[2 * i + 1] as i64 * QMF[11 - i] as i64;
    }
    (even, odd)
}

pub struct Encoder {
    low: Band,
    high: Band,
    x: [i32; 24],
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            low: new_band(),
            high: new_band(),
            x: [0; 24],
        }
    }
}

impl Encoder {
    // 16-kHz-Samples (gerade Anzahl) -> halb so viele Bytes.
    pub fn encode(&mut self, amp: &[i16]) -> Vec<u8> {
        let mut out = Vec::with_capacity(amp.len() / 2);
        for pair in amp.as_chunks::<2>().0 {
            self.x.copy_within(2..24, 0);
            self.x[22] = pair[0] as i32;
            self.x[23] = pair[1] as i32;
            let (even, odd) = qmf(&self.x);
            let xlow = ((even + odd) as i32) >> 14;
            let xhigh = ((even - odd) as i32) >> 14;

            // Tiefband: 6-Bit-Quantisierer
            let lo = &mut self.low;
            let el = sat(xlow - lo.s);
            let wd = if el >= 0 { el } else { -(el + 1) };
            let mut mil = 1;
            while mil < 30 {
                if wd < ((Q6[mil] * lo.det) >> 12) {
                    break;
                }
                mil += 1;
            }
            let ilow = if el < 0 { ILN[mil] } else { ILP[mil] };
            let ril = (ilow >> 2) as usize;
            let dlow = (lo.det * QM4[ril]) >> 15;
            lo.nb = (((lo.nb * 127) >> 7) + WL[RL42[ril]]).clamp(0, 18432);
            lo.det = scale(lo.nb, 8);
            block4(lo, dlow);

            // Hochband: 2-Bit-Quantisierer
            let hi = &mut self.high;
            let eh = sat(xhigh - hi.s);
            let wd = if eh >= 0 { eh } else { -(eh + 1) };
            let mih = if wd >= ((564 * hi.det) >> 12) { 2 } else { 1 };
            let ihigh = if eh < 0 { IHN[mih] } else { IHP[mih] };
            let dhigh = (hi.det * QM2[ihigh as usize]) >> 15;
            hi.nb = (((hi.nb * 127) >> 7) + WH[RH2[ihigh as usize]]).clamp(0, 22528);
            hi.det = scale(hi.nb, 10);
            block4(hi, dhigh);

            out.push(((ihigh << 6) | ilow) as u8);
        }
        out
    }
}

pub struct Decoder {
    low: Band,
    high: Band,
    x: [i32; 24],
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            low: new_band(),
            high: new_band(),
            x: [0; 24],
        }
    }
}

impl Decoder {
    // Bytes -> doppelt so viele 16-kHz-Samples.
    pub fn decode(&mut self, g722: &[u8]) -> Vec<i16> {
        let mut out = Vec::with_capacity(g722.len() * 2);
        for &code in g722 {
            let ilow = (code & 0x3f) as usize;
            let ihigh = ((code >> 6) & 0x03) as usize;

            let lo = &mut self.low;
            let ril = ilow >> 2;
            let dlowt = (lo.det * QM6[ilow]) >> 15;
            let rlow = sat(lo.s + dlowt).clamp(-16384, 16383);
            let dpred = (lo.det * QM4[ril]) >> 15; // Prädiktor läuft mit dem groben 4-Bit-Wert (wie der Encoder)
            lo.nb = (((lo.nb * 127) >> 7) + WL[RL42[ril]]).clamp(0, 18432);
            lo.det = scale(lo.nb, 8);
            block4(lo, dpred);

            let hi = &mut self.high;
            let dhigh = (hi.det * QM2[ihigh]) >> 15;
            let rhigh = sat(hi.s + dhigh).clamp(-16384, 16383);
            hi.nb = (((hi.nb * 127) >> 7) + WH[RH2[ihigh]]).clamp(0, 22528);
            hi.det = scale(hi.nb, 10);
            block4(hi, dhigh);

            self.x.copy_within(2..24, 0);
            self.x[22] = rlow + rhigh;
            self.x[23] = rlow - rhigh;
            let (even, odd) = qmf(&self.x);
            out.push(sat((even as i32) >> DEC_SHIFT) as i16);
            out.push(sat((odd as i32) >> DEC_SHIFT) as i16);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::{Digest, Md5};

    // Testsignal nur aus Ganzzahlen (Sägezahn, Dreieck, Rauschen nach festem Muster), 1 s bei 16 kHz –
    // so erzeugen JavaScript und Rust garantiert dieselben Eingangswerte.
    fn signal() -> Vec<i16> {
        let mut seed: u32 = 12345;
        (0..16000i32)
            .map(|n| {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let noise = ((seed >> 16) % 2001) as i32 - 1000;
                let saw = ((n * 149) % 3000 - 1500) * 6;
                let k = (n * 53) % 800;
                let tri = ((if k < 400 { k } else { 800 - k }) - 200) * 15;
                (saw + tri + noise) as i16
            })
            .collect()
    }

    // Prüfsummen aus src/g722.js der Electron-Version mit demselben Testsignal.
    #[test]
    fn bit_identical_to_electron() {
        let input = signal();
        let raw: Vec<u8> = input.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(
            format!("{:x}", Md5::digest(&raw)),
            "289f1f8874374d71ca493da2e579147b"
        );
        let mut enc = Encoder::default();
        let coded: Vec<u8> = input.chunks(320).flat_map(|f| enc.encode(f)).collect();
        assert_eq!(coded.len(), 8000);
        assert_eq!(
            format!("{:x}", Md5::digest(&coded)),
            "d5049ca96ce4c570110b56907a7ac4d7"
        );
        let mut dec = Decoder::default();
        let decoded: Vec<i16> = coded.chunks(160).flat_map(|f| dec.decode(f)).collect();
        let bytes: Vec<u8> = decoded.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(
            format!("{:x}", Md5::digest(&bytes)),
            "8bcb0259646c25cf33f2fb5acb62aee8"
        );
    }
}
