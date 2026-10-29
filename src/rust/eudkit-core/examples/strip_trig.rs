use eudkit_core::chk::Chk;
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use std::env;
use std::io::Write;

fn main() {
    let src = env::args().nth(1).expect("usage: strip_trig <in.scx> <out.scx>");
    let dst = env::args().nth(2).expect("usage: strip_trig <in.scx> <out.scx>");
    std::fs::copy(&src, &dst).unwrap();
    let mut archive = Archive::open(&src, OpenArchiveFlags::empty()).unwrap();
    let raw = archive
        .open_file("staredit\\scenario.chk")
        .unwrap()
        .read_all()
        .unwrap();
    drop(archive);
    let mut chk = Chk::parse(&raw).unwrap();
    chk.remove_section(b"TRIG");
    let bytes = chk.to_bytes();
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(&bytes).unwrap();
    tmp.flush().unwrap();
    Archive::set_file_locale(0);
    let mut out = Archive::open(&dst, OpenArchiveFlags::empty()).unwrap();
    out.add_file(tmp.path(), "staredit\\scenario.chk", true).unwrap();
    Archive::set_file_locale(0);
    out.compact().unwrap();
}
