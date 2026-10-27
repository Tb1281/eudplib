#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

#[cfg(target_os = "windows")]
include!("./bindings_windows.rs");
#[cfg(target_os = "linux")]
include!("./bindings_linux.rs");
#[cfg(target_os = "macos")]
include!("./bindings_macos.rs");

extern "C" {
  pub fn SetLastError(dwErrCode: u32);
}
extern "C" {
  pub fn GetLastError() -> u32;
}

#[test]
fn test_w3x() {
  use std::ffi::*;
  use std::ptr;

  let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
  let archive_path = manifest_dir.join("../../samples/test_tft.w3x");
  let sample_path = manifest_dir.join("../../samples/war3map.j");

  #[cfg(not(target_os = "windows"))]
  let path = CString::new(archive_path.to_str().unwrap()).unwrap();
  #[cfg(target_os = "windows")]
  let path: Vec<_> = {
    archive_path
      .to_str()
      .unwrap()
      .as_bytes()
      .iter()
      .cloned()
      .map(u16::from)
      .chain(std::iter::once(0))
      .collect()
  };

  let file = CString::new("war3map.j").unwrap();
  unsafe {
    let mut handle: HANDLE = ptr::null_mut();
    let ok = SFileOpenArchive(
      path.as_ptr(),
      0,
      MPQ_OPEN_NO_LISTFILE | MPQ_OPEN_NO_ATTRIBUTES,
      &mut handle as *mut HANDLE,
    );

    assert!(ok);

    let mut file_handle: HANDLE = ptr::null_mut();
    let ok = SFileOpenFileEx(handle, file.as_ptr(), 0, &mut file_handle as *mut HANDLE);
    assert!(ok);

    let mut size_high: DWORD = 0;
    let size = SFileGetFileSize(file_handle, &mut size_high as *mut DWORD);
    assert!(ok);

    println!("file size = {}", size);

    let mut buf = Vec::<u8>::with_capacity(size as usize);
    buf.resize(buf.capacity(), 0);

    let mut read: DWORD = 0;
    let ok = SFileReadFile(
      file_handle,
      std::mem::transmute(buf.as_mut_ptr()),
      size,
      &mut read as *mut DWORD,
      ptr::null_mut(),
    );
    assert!(ok);

    println!("read size = {}", read);

    let expected = std::fs::read(sample_path).unwrap();
    let buf_norm: Vec<u8> = buf.into_iter().filter(|&b| b != b'\r').collect();
    let exp_norm: Vec<u8> = expected.into_iter().filter(|&b| b != b'\r').collect();
    assert_eq!(buf_norm, exp_norm);

    assert!(SFileCloseFile(file_handle));

    assert!(SFileCloseArchive(handle));
  }
}
