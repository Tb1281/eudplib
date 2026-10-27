use crate::allocator::allocate_lir_module;
use crate::chk::Chk;
use crate::error::{EudkitError, Result};
use crate::hir::HirModule;
use crate::injector::{inject_payload_into_chk, CompilationReport};
use crate::lir::{lower_hir_to_lir, LirModule, LirObject};
use crate::map_context::{MapContext, MapString};
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompilationConfig {
    pub obscurate: bool,
    pub obscurate_seed: Option<u64>,
    pub preserve_classic_triggers: bool,
}

#[derive(Debug, Clone)]
pub struct CompilationSession {
    input_map_path: Option<PathBuf>,
    raw_mpq_bytes: Option<Vec<u8>>,
    chk: Chk,
    map_context: MapContext,
    config: CompilationConfig,
    hir_module: Option<HirModule>,
    lir_module: Option<LirModule>,
}

impl CompilationSession {
    pub fn from_map<P: AsRef<Path>>(path: P, config: CompilationConfig) -> Result<Self> {
        let path_ref = path.as_ref();
        let raw_mpq_bytes = fs::read(path_ref)?;

        Archive::set_file_locale(0);
        let mut archive = Archive::open(path_ref, OpenArchiveFlags::empty())?;
        let mut raw_chk = archive.open_file("staredit\\scenario.chk")?.read_all()?;
        if raw_chk.len() <= 1200 {
            Archive::set_file_locale(0x409);
            if let Ok(mut file) = archive.open_file("staredit\\scenario.chk") {
                if let Ok(alt_chk) = file.read_all() {
                    if alt_chk.len() > raw_chk.len() {
                        raw_chk = alt_chk;
                    }
                }
            }
            Archive::set_file_locale(0);
        }
        drop(archive);

        let chk = Chk::parse(&raw_chk)?;
        let map_context = MapContext::from_chk(&chk)?;

        Ok(Self {
            input_map_path: Some(path_ref.to_path_buf()),
            raw_mpq_bytes: Some(raw_mpq_bytes),
            chk,
            map_context,
            config,
            hir_module: None,
            lir_module: None,
        })
    }

    pub fn from_chk_bytes(raw_chk: &[u8], config: CompilationConfig) -> Result<Self> {
        let chk = Chk::parse(raw_chk)?;
        let map_context = MapContext::from_chk(&chk)?;
        Ok(Self {
            input_map_path: None,
            raw_mpq_bytes: None,
            chk,
            map_context,
            config,
            hir_module: None,
            lir_module: None,
        })
    }

    pub fn input_map_path(&self) -> Option<&Path> {
        self.input_map_path.as_deref()
    }

    pub fn chk(&self) -> &Chk {
        &self.chk
    }

    pub fn map_context(&self) -> &MapContext {
        &self.map_context
    }

    pub fn map_context_mut(&mut self) -> &mut MapContext {
        &mut self.map_context
    }

    pub fn config(&self) -> &CompilationConfig {
        &self.config
    }

    pub fn register_map_string(&mut self, text: &str) -> Result<MapString> {
        self.map_context.register_map_string(text)
    }

    pub fn set_hir_module(&mut self, hir: HirModule) {
        self.hir_module = Some(hir);
        self.lir_module = None;
    }

    pub fn hir_module(&self) -> Option<&HirModule> {
        self.hir_module.as_ref()
    }

    pub fn lir_module(&self) -> Option<&LirModule> {
        self.lir_module.as_ref()
    }

    pub fn lower_to_lir(&mut self) -> Result<&LirModule> {
        let hir = self
            .hir_module
            .as_ref()
            .ok_or_else(|| EudkitError::Ir("no HIR module set in CompilationSession".to_string()))?;
        let lir = lower_hir_to_lir(hir, &mut self.map_context)?;
        self.lir_module = Some(lir);
        Ok(self.lir_module.as_ref().unwrap())
    }

    pub fn dump_hir(&self) -> String {
        match &self.hir_module {
            Some(hir) => hir.dump_hir(),
            None => String::new(),
        }
    }

    pub fn dump_lir(&mut self) -> Result<String> {
        if self.lir_module.is_none() {
            self.lower_to_lir()?;
        }
        Ok(self.lir_module.as_ref().unwrap().dump_lir())
    }

    /// Compiles the current `HIR`/`LIR` module, allocates and statically relocates
    /// the `Payload` into `STRx`, emits the single-stage `TRIG` entry bootstrap,
    /// and writes the resulting deterministic `.scx` MPQ archive to `output_path`.
    pub fn compile_map<P: AsRef<Path>>(&mut self, output_path: P) -> Result<CompilationReport> {
        let (raw_chk, report) = self.compile_chk_bytes()?;
        let raw_mpq = self.raw_mpq_bytes.as_ref().ok_or_else(|| {
            EudkitError::Config(
                "cannot write .scx archive when session was initialized without an input MPQ"
                    .to_string(),
            )
        })?;

        let out_ref = output_path.as_ref();
        if let Some(parent) = out_ref.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(out_ref, raw_mpq)?;

        let mut tmp_chk = tempfile::NamedTempFile::new()?;
        tmp_chk.write_all(&raw_chk)?;
        tmp_chk.flush()?;

        Archive::set_file_locale(0);
        let mut archive = Archive::open(out_ref, OpenArchiveFlags::empty())?;
        archive.add_file(tmp_chk.path(), "staredit\\scenario.chk", true)?;
        Archive::set_file_locale(0);
        archive.compact()?;
        drop(archive);

        Ok(report)
    }

    /// Compiles the current `HIR`/`LIR` module and returns the serialized `scenario.chk`
    /// bytes along with the `CompilationReport`.
    pub fn compile_chk_bytes(&mut self) -> Result<(Vec<u8>, CompilationReport)> {
        if self.lir_module.is_none() {
            self.lower_to_lir()?;
        }
        let lir = self.lir_module.as_ref().unwrap();
        let allocated = allocate_lir_module(lir)?;

        let trigger_count = lir
            .objects()
            .iter()
            .filter(|o| matches!(o, LirObject::Trigger(_)))
            .count();
        let byte_buf_count = lir
            .objects()
            .iter()
            .filter(|o| matches!(o, LirObject::ByteBuf(_)))
            .count();

        let mut out_chk = self.chk.clone();
        let report = inject_payload_into_chk(
            &mut out_chk,
            &self.map_context,
            &allocated,
            trigger_count,
            byte_buf_count,
        )?;
        Ok((out_chk.to_bytes(), report))
    }
}
