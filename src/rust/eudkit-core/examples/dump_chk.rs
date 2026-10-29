use eudkit_core::{CompilationConfig, CompilationSession};
use std::env;

fn main() {
    let path = env::args().nth(1).expect("usage: dump_chk <map.scx>");
    let session = CompilationSession::from_map(&path, CompilationConfig::default()).unwrap();
    let chk = session.chk();
    let trig = chk.section(b"TRIG").unwrap();
    let strx = chk.section(b"STRx").unwrap();
    let n = trig.len() / 2400;
    println!("triggers={n} strx_len={} payload_guess={}", strx.len(), strx.len() - 4204);
    for i in 0..n {
        let raw = &trig[i * 2400..(i + 1) * 2400];
        if i < 9 || i >= n - 3 {
            print_trigger(i, raw);
        }
    }
    // dump the first payload trigger, which starts at strx len - 2408
    let entry = strx.len() - 2408;
    println!("--- payload trigger at strx+{entry} ---");
    let p = &strx[entry..];
    println!("payload bytes: {}", hex_prefix(&p[..96], 96));
    println!(
        "prev={:#x} next={:#x}",
        u32::from_le_bytes(p[0..4].try_into().unwrap()),
        u32::from_le_bytes(p[4..8].try_into().unwrap())
    );
    print_mem_trigger(0, p);
    println!("--- str offsets ---");
    for i in 0..12 {
        let off = u32::from_le_bytes(strx[i * 4..i * 4 + 4].try_into().unwrap());
        println!("  str[{i}] off={off:#x}");
    }
    println!("--- str start ---");
    println!("{}", String::from_utf8_lossy(&strx[0x400.min(strx.len())..strx.len().min(0x460)]));
}

fn hex_prefix(data: &[u8], n: usize) -> String {
    data.iter()
        .take(n)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn print_mem_trigger(i: usize, raw: &[u8]) {
    let flags = u32::from_le_bytes(raw[2376..2380].try_into().unwrap());
    print!("M{i}: flags={flags:#x} conds=[");
    for c in 0..16 {
        let b = &raw[8 + c * 20..8 + (c + 1) * 20];
        let locid = u32::from_le_bytes(b[0..4].try_into().unwrap());
        let player = i32::from_le_bytes(b[4..8].try_into().unwrap());
        let amount = u32::from_le_bytes(b[8..12].try_into().unwrap());
        let unit = u16::from_le_bytes(b[12..14].try_into().unwrap());
        let cmp = b[14];
        let ct = b[15];
        let eudx = u16::from_le_bytes(b[18..20].try_into().unwrap());
        if ct != 0 || locid != 0 || player != 0 {
            print!("[loc={locid:#x} p={player} amt={amount} u={unit} cmp={cmp} t={ct} x={eudx:#x}]");
        }
    }
    print!("] acts=[");
    for a in 0..64 {
        let b = &raw[328 + a * 32..328 + (a + 1) * 32];
        let loc1 = u32::from_le_bytes(b[0..4].try_into().unwrap());
        let strid = u32::from_le_bytes(b[4..8].try_into().unwrap());
        let p1 = i32::from_le_bytes(b[16..20].try_into().unwrap());
        let p2 = i32::from_le_bytes(b[20..24].try_into().unwrap());
        let unit = u16::from_le_bytes(b[24..26].try_into().unwrap());
        let at = b[26];
        let amt = b[27];
        let fl = b[28];
        let eudx = u16::from_le_bytes(b[30..32].try_into().unwrap());
        if at != 0 || p1 != 0 || p2 != 0 || loc1 != 0 || strid != 0 {
            print!("[loc={loc1:#x} str={strid} p1={p1} p2={p2:#x} u={unit} t={at} amt={amt} fl={fl} x={eudx:#x}]");
        }
    }
    println!("]");
}

fn print_trigger(i: usize, raw: &[u8]) {
    let players = (0..27).filter(|p| raw[2372 + p] != 0).collect::<Vec<_>>();
    let flags = u32::from_le_bytes(raw[2368..2372].try_into().unwrap());
    print!("T{i}: players={players:?} flags={flags:#x} conds=[");
    for c in 0..16 {
        let b = &raw[c * 20..(c + 1) * 20];
        let locid = u32::from_le_bytes(b[0..4].try_into().unwrap());
        let player = i32::from_le_bytes(b[4..8].try_into().unwrap());
        let amount = u32::from_le_bytes(b[8..12].try_into().unwrap());
        let unit = u16::from_le_bytes(b[12..14].try_into().unwrap());
        let cmp = b[14];
        let ct = b[15];
        let eudx = u16::from_le_bytes(b[18..20].try_into().unwrap());
        if ct != 0 || locid != 0 || player != 0 {
            print!("[loc={locid:#x} p={player} amt={amount} u={unit} cmp={cmp} t={ct} x={eudx:#x}]");
        }
    }
    print!("] acts=[");
    for a in 0..64 {
        let b = &raw[320 + a * 32..320 + (a + 1) * 32];
        let loc1 = u32::from_le_bytes(b[0..4].try_into().unwrap());
        let strid = u32::from_le_bytes(b[4..8].try_into().unwrap());
        let p1 = i32::from_le_bytes(b[16..20].try_into().unwrap());
        let p2 = i32::from_le_bytes(b[20..24].try_into().unwrap());
        let unit = u16::from_le_bytes(b[24..26].try_into().unwrap());
        let at = b[26];
        let amt = b[27];
        let fl = b[28];
        let eudx = u16::from_le_bytes(b[30..32].try_into().unwrap());
        if at != 0 || p1 != 0 || p2 != 0 || loc1 != 0 || strid != 0 {
            print!("[loc={loc1:#x} str={strid} p1={p1} p2={p2:#x} u={unit} t={at} amt={amt} fl={fl} x={eudx:#x}]");
        }
    }
    println!("]");
}
