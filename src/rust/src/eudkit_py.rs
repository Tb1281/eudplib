use eudkit_core::config::BuildRequest;
use eudkit_core::hir::{
    parse_format_template, Comparison, FuncId, FuncPtrId, HirBinOp, HirByteBuf, HirExpr,
    HirUnaryOp, HirValue, Modifier, PlayerId, PrintTarget, ResourceType, StockAction,
    StockCondition, SwitchAction, SwitchState, VarId,
};
use eudkit_core::{CompilationConfig, CompilationSession, EudkitError, HirBuilder};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::{Path, PathBuf};

fn to_py_err(err: EudkitError) -> PyErr {
    match err {
        EudkitError::UnknownSymbol { .. } | EudkitError::AmbiguousSymbol { .. } => {
            PyKeyError::new_err(err.to_string())
        }
        EudkitError::Ir(_) | EudkitError::Config(_) | EudkitError::InvalidChkSection { .. } => {
            PyValueError::new_err(err.to_string())
        }
        other => PyRuntimeError::new_err(other.to_string()),
    }
}

fn decode_comparison(code: u8) -> PyResult<Comparison> {
    match code {
        0 => Ok(Comparison::AtLeast),
        1 => Ok(Comparison::AtMost),
        10 => Ok(Comparison::Exactly),
        other => Err(PyValueError::new_err(format!(
            "invalid comparison code: {other}"
        ))),
    }
}

fn decode_modifier(code: u8) -> PyResult<Modifier> {
    match code {
        7 => Ok(Modifier::SetTo),
        8 => Ok(Modifier::Add),
        9 => Ok(Modifier::Subtract),
        other => Err(PyValueError::new_err(format!(
            "invalid modifier code: {other}"
        ))),
    }
}

fn decode_resource(code: u8) -> PyResult<ResourceType> {
    match code {
        0 => Ok(ResourceType::Ore),
        1 => Ok(ResourceType::Gas),
        2 => Ok(ResourceType::OreAndGas),
        other => Err(PyValueError::new_err(format!(
            "invalid resource code: {other}"
        ))),
    }
}

fn decode_switch_state(code: u8) -> PyResult<SwitchState> {
    match code {
        2 => Ok(SwitchState::Set),
        3 => Ok(SwitchState::Cleared),
        other => Err(PyValueError::new_err(format!(
            "invalid switch state code: {other}"
        ))),
    }
}

fn decode_switch_action(code: u8) -> PyResult<SwitchAction> {
    match code {
        4 | 2 => Ok(SwitchAction::Set),
        5 | 3 => Ok(SwitchAction::Clear),
        6 => Ok(SwitchAction::Toggle),
        11 | 7 => Ok(SwitchAction::Random),
        other => Err(PyValueError::new_err(format!(
            "invalid switch action code: {other}"
        ))),
    }
}

fn decode_player(code: i32) -> PlayerId {
    match code {
        0 => PlayerId::Player1,
        1 => PlayerId::Player2,
        2 => PlayerId::Player3,
        3 => PlayerId::Player4,
        4 => PlayerId::Player5,
        5 => PlayerId::Player6,
        6 => PlayerId::Player7,
        7 => PlayerId::Player8,
        8 => PlayerId::Player9,
        9 => PlayerId::Player10,
        10 => PlayerId::Player11,
        11 => PlayerId::Player12,
        13 => PlayerId::CurrentPlayer,
        14 => PlayerId::Foes,
        15 => PlayerId::Allies,
        16 => PlayerId::NeutralPlayers,
        17 => PlayerId::AllPlayers,
        18 => PlayerId::Force1,
        19 => PlayerId::Force2,
        20 => PlayerId::Force3,
        21 => PlayerId::Force4,
        26 => PlayerId::NonAlliedVictoryPlayers,
        other => PlayerId::Epd(other),
    }
}

fn decode_bin_op(code: u8) -> PyResult<HirBinOp> {
    match code {
        0 => Ok(HirBinOp::Add),
        1 => Ok(HirBinOp::Sub),
        2 => Ok(HirBinOp::Mul),
        3 => Ok(HirBinOp::Div),
        4 => Ok(HirBinOp::Rem),
        5 => Ok(HirBinOp::BitAnd),
        6 => Ok(HirBinOp::BitOr),
        7 => Ok(HirBinOp::BitXor),
        8 => Ok(HirBinOp::Shl),
        9 => Ok(HirBinOp::Shr),
        10 => Ok(HirBinOp::SubSat),
        other => Err(PyValueError::new_err(format!(
            "invalid binary op code: {other}"
        ))),
    }
}

#[derive(Clone, Debug)]
#[pyclass(from_py_object, name = "RawExpr", module = "eudkit._rust")]
pub struct PyExpr(pub HirExpr);

#[pymethods]
impl PyExpr {
    #[staticmethod]
    fn const_val(val: i32) -> Self {
        Self(HirExpr::Const(val))
    }

    #[staticmethod]
    fn var(var_id: u32) -> Self {
        Self(HirExpr::Var(VarId(var_id)))
    }

    #[staticmethod]
    fn unary_neg(operand: &PyExpr) -> Self {
        Self(HirExpr::UnaryOp {
            op: HirUnaryOp::Neg,
            operand: Box::new(operand.0.clone()),
        })
    }

    #[staticmethod]
    fn unary_not(operand: &PyExpr) -> Self {
        Self(HirExpr::UnaryOp {
            op: HirUnaryOp::BitNot,
            operand: Box::new(operand.0.clone()),
        })
    }

    #[staticmethod]
    fn bin_op(op: u8, lhs: &PyExpr, rhs: &PyExpr) -> PyResult<Self> {
        Ok(Self(HirExpr::BinOp {
            op: decode_bin_op(op)?,
            lhs: Box::new(lhs.0.clone()),
            rhs: Box::new(rhs.0.clone()),
        }))
    }

    fn __repr__(&self) -> String {
        format!("{}", self.0)
    }
}

#[derive(Clone, Debug)]
#[pyclass(from_py_object, name = "RawMapString", module = "eudkit._rust")]
pub struct PyMapString {
    #[pyo3(get)]
    pub id: u32,
    #[pyo3(get)]
    pub offset: u32,
    #[pyo3(get)]
    pub address: u32,
    #[pyo3(get)]
    pub epd: i32,
}

#[pymethods]
impl PyMapString {
    fn __repr__(&self) -> String {
        format!(
            "RawMapString(id={}, offset=0x{:X}, address=0x{:X}, epd={})",
            self.id, self.offset, self.address, self.epd
        )
    }
}

#[derive(Clone, Debug)]
#[pyclass(from_py_object, name = "RawCondition", module = "eudkit._rust")]
pub struct PyCondition(pub StockCondition);

#[pymethods]
impl PyCondition {
    #[staticmethod]
    fn always() -> Self {
        Self(StockCondition::Always)
    }

    #[staticmethod]
    fn never() -> Self {
        Self(StockCondition::Never)
    }

    #[staticmethod]
    fn is_user_cp() -> Self {
        Self(StockCondition::IsUserCp)
    }

    #[staticmethod]
    fn deaths(player: i32, comparison: u8, amount: u32, unit: u16) -> PyResult<Self> {
        Ok(Self(StockCondition::Deaths {
            player: decode_player(player),
            comparison: decode_comparison(comparison)?,
            amount,
            unit,
        }))
    }

    #[staticmethod]
    fn deaths_x(player: i32, comparison: u8, amount: u32, unit: u16, mask: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::DeathsX {
            player: decode_player(player),
            comparison: decode_comparison(comparison)?,
            amount,
            unit,
            mask,
        }))
    }

    #[staticmethod]
    fn memory(address: u32, comparison: u8, value: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::Memory {
            address,
            comparison: decode_comparison(comparison)?,
            value,
        }))
    }

    #[staticmethod]
    fn memory_x(address: u32, comparison: u8, value: u32, mask: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::MemoryX {
            address,
            comparison: decode_comparison(comparison)?,
            value,
            mask,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (var_id, comparison, value, mask=None))]
    fn var_cmp(var_id: u32, comparison: u8, value: u32, mask: Option<u32>) -> PyResult<Self> {
        Ok(Self(StockCondition::VarCmp {
            var: VarId(var_id),
            comparison: decode_comparison(comparison)?,
            value,
            mask,
        }))
    }

    #[staticmethod]
    fn expr_cmp(lhs: &PyExpr, comparison: u8, rhs: &PyExpr) -> PyResult<Self> {
        Ok(Self(StockCondition::ExprCmp {
            lhs: lhs.0.clone(),
            comparison: decode_comparison(comparison)?,
            rhs: rhs.0.clone(),
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (player, comparison, amount, unit, mask=None))]
    fn deaths_dynamic(
        player: &PyExpr,
        comparison: u8,
        amount: &PyExpr,
        unit: u16,
        mask: Option<u32>,
    ) -> PyResult<Self> {
        Ok(Self(StockCondition::DeathsDynamic {
            player: player.0.clone(),
            comparison: decode_comparison(comparison)?,
            amount: amount.0.clone(),
            unit,
            mask,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (address, comparison, value, mask=None))]
    fn memory_dynamic(
        address: u32,
        comparison: u8,
        value: &PyExpr,
        mask: Option<u32>,
    ) -> PyResult<Self> {
        Ok(Self(StockCondition::MemoryDynamic {
            address,
            comparison: decode_comparison(comparison)?,
            value: value.0.clone(),
            mask,
        }))
    }

    #[staticmethod]
    fn accumulate(player: i32, comparison: u8, amount: u32, resource: u8) -> PyResult<Self> {
        Ok(Self(StockCondition::Accumulate {
            player: decode_player(player),
            comparison: decode_comparison(comparison)?,
            amount,
            resource: decode_resource(resource)?,
        }))
    }

    #[staticmethod]
    fn command(player: i32, comparison: u8, amount: u32, unit: u16) -> PyResult<Self> {
        Ok(Self(StockCondition::Command {
            player: decode_player(player),
            comparison: decode_comparison(comparison)?,
            amount,
            unit,
        }))
    }

    #[staticmethod]
    fn bring(player: i32, comparison: u8, amount: u32, unit: u16, location: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::Bring {
            player: decode_player(player),
            comparison: decode_comparison(comparison)?,
            amount,
            unit,
            location,
        }))
    }

    #[staticmethod]
    fn switch(switch_id: u8, state: u8) -> PyResult<Self> {
        Ok(Self(StockCondition::Switch {
            switch: switch_id,
            state: decode_switch_state(state)?,
        }))
    }

    #[staticmethod]
    fn elapsed_time(comparison: u8, seconds: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::ElapsedTime {
            comparison: decode_comparison(comparison)?,
            seconds,
        }))
    }

    #[staticmethod]
    fn score(player: i32, score_type: u8, comparison: u8, amount: u32) -> PyResult<Self> {
        Ok(Self(StockCondition::Score {
            player: decode_player(player),
            score_type,
            comparison: decode_comparison(comparison)?,
            amount,
        }))
    }

    fn __repr__(&self) -> String {
        format!("{}", self.0)
    }
}

#[derive(Clone, Debug)]
#[pyclass(from_py_object, name = "RawAction", module = "eudkit._rust")]
pub struct PyAction(pub StockAction);

#[pymethods]
impl PyAction {
    #[staticmethod]
    fn set_deaths(player: i32, modifier: u8, value: i32, unit: u16) -> PyResult<Self> {
        Ok(Self(StockAction::SetDeaths {
            player: decode_player(player),
            modifier: decode_modifier(modifier)?,
            value: HirValue::Const(value),
            unit,
        }))
    }

    #[staticmethod]
    fn set_deaths_x(player: i32, modifier: u8, value: i32, unit: u16, mask: u32) -> PyResult<Self> {
        Ok(Self(StockAction::SetDeathsX {
            player: decode_player(player),
            modifier: decode_modifier(modifier)?,
            value: HirValue::Const(value),
            unit,
            mask,
        }))
    }

    #[staticmethod]
    fn set_memory(address: u32, modifier: u8, value: i32) -> PyResult<Self> {
        Ok(Self(StockAction::SetMemory {
            address,
            modifier: decode_modifier(modifier)?,
            value: HirValue::Const(value),
        }))
    }

    #[staticmethod]
    fn set_memory_x(address: u32, modifier: u8, value: i32, mask: u32) -> PyResult<Self> {
        Ok(Self(StockAction::SetMemoryX {
            address,
            modifier: decode_modifier(modifier)?,
            value: HirValue::Const(value),
            mask,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (var_id, modifier, value, mask=None))]
    fn set_var_const(var_id: u32, modifier: u8, value: i32, mask: Option<u32>) -> PyResult<Self> {
        Ok(Self(StockAction::SetVarConst {
            var: VarId(var_id),
            modifier: decode_modifier(modifier)?,
            value,
            mask,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (player, modifier, value, unit, mask=None))]
    fn set_deaths_dynamic(
        player: &PyExpr,
        modifier: u8,
        value: &PyExpr,
        unit: u16,
        mask: Option<u32>,
    ) -> PyResult<Self> {
        Ok(Self(StockAction::SetDeathsDynamic {
            player: player.0.clone(),
            modifier: decode_modifier(modifier)?,
            value: value.0.clone(),
            unit,
            mask,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (address, modifier, value, mask=None))]
    fn set_memory_dynamic(
        address: u32,
        modifier: u8,
        value: &PyExpr,
        mask: Option<u32>,
    ) -> PyResult<Self> {
        Ok(Self(StockAction::SetMemoryDynamic {
            address,
            modifier: decode_modifier(modifier)?,
            value: value.0.clone(),
            mask,
        }))
    }

    #[staticmethod]
    fn set_resources(player: i32, modifier: u8, amount: u32, resource: u8) -> PyResult<Self> {
        Ok(Self(StockAction::SetResources {
            player: decode_player(player),
            modifier: decode_modifier(modifier)?,
            amount,
            resource: decode_resource(resource)?,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (text, always_display=true))]
    fn display_text(text: String, always_display: bool) -> Self {
        Self(StockAction::DisplayText {
            text,
            always_display,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (string_id, always_display=true))]
    fn display_text_id(string_id: u32, always_display: bool) -> Self {
        Self(StockAction::DisplayTextId {
            string_id,
            always_display,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (template, args=None, always_display=true))]
    fn format_display_text(
        template: String,
        args: Option<Vec<PyExpr>>,
        always_display: bool,
    ) -> PyResult<Self> {
        let arg_exprs: Vec<HirExpr> = args.unwrap_or_default().into_iter().map(|e| e.0).collect();
        let parts =
            parse_format_template(&template, &arg_exprs).map_err(|e| PyValueError::new_err(e))?;
        Ok(Self(StockAction::FormatPrint {
            target: PrintTarget::DisplayText,
            parts,
            always_display,
        }))
    }

    #[staticmethod]
    #[pyo3(signature = (line, template, args=None))]
    fn print_at(line: u8, template: String, args: Option<Vec<PyExpr>>) -> PyResult<Self> {
        let arg_exprs: Vec<HirExpr> = args.unwrap_or_default().into_iter().map(|e| e.0).collect();
        let parts =
            parse_format_template(&template, &arg_exprs).map_err(|e| PyValueError::new_err(e))?;
        Ok(Self(StockAction::PrintAt { line, parts }))
    }

    #[staticmethod]
    #[pyo3(signature = (string_id, template, args=None))]
    fn format_map_string(
        string_id: u32,
        template: String,
        args: Option<Vec<PyExpr>>,
    ) -> PyResult<Self> {
        let arg_exprs: Vec<HirExpr> = args.unwrap_or_default().into_iter().map(|e| e.0).collect();
        let parts =
            parse_format_template(&template, &arg_exprs).map_err(|e| PyValueError::new_err(e))?;
        Ok(Self(StockAction::FormatPrint {
            target: PrintTarget::MapString(string_id),
            parts,
            always_display: false,
        }))
    }

    #[staticmethod]
    fn create_unit(count: u8, unit: u16, location: u32, player: i32) -> Self {
        Self(StockAction::CreateUnit {
            count,
            unit,
            location,
            player: decode_player(player),
        })
    }

    #[staticmethod]
    fn kill_unit_at(count: u8, unit: u16, location: u32, player: i32) -> Self {
        Self(StockAction::KillUnitAt {
            count,
            unit,
            location,
            player: decode_player(player),
        })
    }

    #[staticmethod]
    fn minimap_ping(location: u32) -> Self {
        Self(StockAction::MinimapPing { location })
    }

    #[staticmethod]
    fn set_score(player: i32, modifier: u8, amount: u32, score_type: u8) -> PyResult<Self> {
        Ok(Self(StockAction::SetScore {
            player: decode_player(player),
            modifier: decode_modifier(modifier)?,
            amount,
            score_type,
        }))
    }

    #[staticmethod]
    fn set_switch(switch: u8, action: u8) -> PyResult<Self> {
        Ok(Self(StockAction::SetSwitch {
            switch,
            action: decode_switch_action(action)?,
        }))
    }

    #[staticmethod]
    fn move_location(location: u32, unit: u16, player: i32, dest_location: u32) -> Self {
        Self(StockAction::MoveLocation {
            location,
            unit,
            player: decode_player(player),
            dest_location,
        })
    }

    #[staticmethod]
    fn remove_unit_at(count: u8, unit: u16, location: u32, player: i32) -> Self {
        Self(StockAction::RemoveUnitAt {
            count,
            unit,
            location,
            player: decode_player(player),
        })
    }

    #[staticmethod]
    fn kill_unit(unit: u16, player: i32) -> Self {
        Self(StockAction::KillUnit {
            unit,
            player: decode_player(player),
        })
    }

    #[staticmethod]
    fn move_unit(
        count: u8,
        unit: u16,
        player: i32,
        start_location: u32,
        dest_location: u32,
    ) -> Self {
        Self(StockAction::MoveUnit {
            count,
            unit,
            player: decode_player(player),
            start_location,
            dest_location,
        })
    }

    #[staticmethod]
    fn give_units(count: u8, unit: u16, player: i32, location: u32, new_player: i32) -> Self {
        Self(StockAction::GiveUnits {
            count,
            unit,
            player: decode_player(player),
            location,
            new_player: decode_player(new_player),
        })
    }

    #[staticmethod]
    fn order(
        unit: u16,
        player: i32,
        start_location: u32,
        order_type: u8,
        dest_location: u32,
    ) -> Self {
        Self(StockAction::Order {
            unit,
            player: decode_player(player),
            start_location,
            order_type,
            dest_location,
        })
    }

    #[staticmethod]
    fn play_wav(path: String) -> Self {
        Self(StockAction::PlayWAV { path })
    }

    #[staticmethod]
    fn comment(text: String) -> Self {
        Self(StockAction::Comment { text })
    }

    #[staticmethod]
    fn preserve_trigger() -> Self {
        Self(StockAction::PreserveTrigger)
    }

    #[staticmethod]
    fn victory() -> Self {
        Self(StockAction::Victory)
    }

    #[staticmethod]
    fn defeat() -> Self {
        Self(StockAction::Defeat)
    }

    fn __repr__(&self) -> String {
        format!("{}", self.0)
    }
}

/// The typed compilation result exposed to Python.
///
/// It wraps the core `CompilationReport` so the summary text and JSON wire
/// format have exactly one implementation, shared with the Rust CLI.
#[derive(Clone, Debug)]
#[pyclass(from_py_object, name = "CompilationReport", module = "eudkit._rust")]
pub struct PyCompilationReport {
    report: eudkit_core::CompilationReport,
}

#[pymethods]
impl PyCompilationReport {
    #[getter]
    fn trigger_count(&self) -> usize {
        self.report.trigger_count
    }

    #[getter]
    fn byte_buf_count(&self) -> usize {
        self.report.byte_buf_count
    }

    #[getter]
    fn vproc_count(&self) -> usize {
        self.report.vproc_count
    }

    #[getter]
    fn payload_offset(&self) -> usize {
        self.report.payload_offset
    }

    #[getter]
    fn payload_size(&self) -> usize {
        self.report.payload_size
    }

    #[getter]
    fn payload_entry_addr(&self) -> u32 {
        self.report.payload_entry_addr
    }

    #[getter]
    fn prt_count(&self) -> usize {
        self.report.prt_offsets.len()
    }

    #[getter]
    fn ort_count(&self) -> usize {
        self.report.ort_offsets.len()
    }

    #[getter]
    fn prt_offsets(&self) -> Vec<usize> {
        self.report.prt_offsets.clone()
    }

    #[getter]
    fn ort_offsets(&self) -> Vec<usize> {
        self.report.ort_offsets.clone()
    }

    /// Renders the same one-line summary as the Rust CLI.
    fn summary(&self, output_map: &str) -> String {
        self.report.summary(output_map)
    }

    /// Serializes the typed report as JSON for the Rust CLI.
    fn to_json(&self) -> PyResult<String> {
        self.report
            .to_json()
            .map_err(|e| PyRuntimeError::new_err(format!("failed to serialize build report: {e}")))
    }
}

#[pyclass(unsendable, name = "RawSession", module = "eudkit._rust")]
pub struct PyCompilationSession {
    session: CompilationSession,
    builder: HirBuilder,
    dirty: bool,
}

impl PyCompilationSession {
    fn sync_hir(&mut self) -> PyResult<()> {
        if self.builder.is_in_frame() {
            return Err(PyValueError::new_err(
                "cannot compile or dump IR while a function or control-flow block is still open",
            ));
        }
        if self.dirty {
            let module = self.builder.build().map_err(to_py_err)?;
            self.session.set_hir_module(module);
            self.dirty = false;
        }
        Ok(())
    }
}

#[pymethods]
impl PyCompilationSession {
    #[new]
    #[pyo3(signature = (map_path, obscurate=false, obscurate_seed=None, preserve_classic_triggers=true))]
    fn new(
        map_path: &str,
        obscurate: bool,
        obscurate_seed: Option<u64>,
        preserve_classic_triggers: bool,
    ) -> PyResult<Self> {
        let config = CompilationConfig {
            obscurate,
            obscurate_seed,
            preserve_classic_triggers,
        };
        let session = CompilationSession::from_map(map_path, config).map_err(to_py_err)?;
        Ok(Self {
            session,
            builder: HirBuilder::new("eudkit_module"),
            dirty: true,
        })
    }

    #[pyo3(signature = (obscurate, seed=None))]
    fn set_obscurate(&mut self, obscurate: bool, seed: Option<u64>) {
        self.session.set_obscurate(obscurate);
        if let Some(s) = seed {
            self.session.set_obscurate_seed(s);
        }
    }

    fn set_obscurate_seed(&mut self, seed: u64) {
        self.session.set_obscurate_seed(seed);
    }

    fn location_id(&self, name: &str) -> PyResult<u32> {
        self.session
            .map_context()
            .location_id(name)
            .ok_or_else(|| PyKeyError::new_err(format!("unknown location: {name:?}")))
    }

    fn unit_id(&self, name: &str) -> PyResult<u16> {
        self.session
            .map_context()
            .unit_id(name)
            .ok_or_else(|| PyKeyError::new_err(format!("unknown unit: {name:?}")))
    }

    fn switch_id(&self, name: &str) -> PyResult<u8> {
        self.session
            .map_context()
            .switch_id(name)
            .ok_or_else(|| PyKeyError::new_err(format!("unknown switch: {name:?}")))
    }

    fn get_or_insert_string(&mut self, text: &str) -> u32 {
        self.session
            .map_context_mut()
            .get_or_insert_string(text.as_bytes())
    }

    fn register_map_string(&mut self, text: &str) -> PyResult<PyMapString> {
        let ms = self.session.register_map_string(text).map_err(to_py_err)?;
        Ok(PyMapString {
            id: ms.id,
            offset: ms.offset,
            address: ms.address,
            epd: ms.epd,
        })
    }

    fn classic_trigger_count(&self) -> usize {
        self.session.map_context().triggers().len()
    }

    fn strx_byte_len(&self) -> usize {
        self.session
            .chk()
            .section(b"STRx")
            .map(|s| s.len())
            .unwrap_or(0)
    }

    fn locations_count(&self) -> usize {
        self.session.map_context().locations().len()
    }

    fn placed_units_count(&self) -> usize {
        self.session.map_context().placed_units().len()
    }

    fn add_var(&mut self, name: &str, init_val: i32) -> u32 {
        self.dirty = true;
        self.builder.add_var(name, init_val).0
    }

    fn add_byte_buf(&mut self, name: &str, data: &[u8]) -> u32 {
        self.dirty = true;
        self.builder
            .add_byte_buf(HirByteBuf::new(name, data.to_vec()))
            .0
    }

    #[pyo3(signature = (name, is_main_loop=true, params=None, return_count=0, inline=None))]
    fn begin_function(
        &mut self,
        name: &str,
        is_main_loop: bool,
        params: Option<Vec<u32>>,
        return_count: usize,
        inline: Option<bool>,
    ) {
        self.dirty = true;
        let params = params.unwrap_or_default().into_iter().map(VarId).collect();
        self.builder
            .begin_function(name, is_main_loop, params, return_count, inline);
    }

    fn end_function(&mut self) -> PyResult<u32> {
        self.dirty = true;
        let fn_id = self.builder.end_function().map_err(to_py_err)?;
        Ok(fn_id.0)
    }

    fn begin_if(&mut self, conditions: Vec<PyCondition>) {
        self.dirty = true;
        self.builder
            .begin_if(conditions.into_iter().map(|c| c.0).collect());
    }

    fn end_if(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.end_if().map_err(to_py_err)
    }

    fn begin_elif(&mut self, conditions: Vec<PyCondition>) -> PyResult<()> {
        self.dirty = true;
        self.builder
            .begin_elif(conditions.into_iter().map(|c| c.0).collect())
            .map_err(to_py_err)
    }

    fn end_elif(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.end_elif().map_err(to_py_err)
    }

    fn begin_else(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.begin_else().map_err(to_py_err)
    }

    fn end_else(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.end_else().map_err(to_py_err)
    }

    fn begin_while(&mut self, conditions: Vec<PyCondition>) {
        self.dirty = true;
        self.builder
            .begin_while(conditions.into_iter().map(|c| c.0).collect());
    }

    fn end_while(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.end_while().map_err(to_py_err)
    }

    fn begin_once(&mut self, conditions: Vec<PyCondition>) {
        self.dirty = true;
        self.builder
            .begin_once(conditions.into_iter().map(|c| c.0).collect());
    }

    fn begin_when(&mut self, conditions: Vec<PyCondition>) {
        self.dirty = true;
        self.builder
            .begin_when(conditions.into_iter().map(|c| c.0).collect());
    }

    fn begin_until(&mut self, conditions: Vec<PyCondition>) {
        self.dirty = true;
        self.builder
            .begin_until(conditions.into_iter().map(|c| c.0).collect());
    }

    fn begin_after(&mut self, delay_ticks: u32) {
        self.dirty = true;
        self.builder.begin_after(delay_ticks);
    }

    fn begin_every(&mut self, period_ticks: u32) {
        self.dirty = true;
        self.builder.begin_every(period_ticks);
    }

    fn end_reactive(&mut self) -> PyResult<()> {
        self.dirty = true;
        self.builder.end_reactive().map_err(to_py_err)
    }

    fn end_once(&mut self) -> PyResult<()> {
        self.end_reactive()
    }

    fn end_when(&mut self) -> PyResult<()> {
        self.end_reactive()
    }

    fn end_until(&mut self) -> PyResult<()> {
        self.end_reactive()
    }

    fn end_after(&mut self) -> PyResult<()> {
        self.end_reactive()
    }

    fn end_every(&mut self) -> PyResult<()> {
        self.end_reactive()
    }

    fn push_break(&mut self) {
        self.dirty = true;
        self.builder.push_break();
    }

    fn push_continue(&mut self) {
        self.dirty = true;
        self.builder.push_continue();
    }

    fn push_var_assign(&mut self, var_id: u32, value: i32) {
        self.dirty = true;
        self.builder.push_var_assign_const(VarId(var_id), value);
    }

    fn push_var_assign_expr(&mut self, var_id: u32, expr: &PyExpr) {
        self.dirty = true;
        self.builder
            .push_var_assign_expr(VarId(var_id), expr.0.clone());
    }

    fn push_var_add(&mut self, var_id: u32, value: i32) {
        self.dirty = true;
        self.builder.push_var_add_const(VarId(var_id), value);
    }

    fn push_var_sub(&mut self, var_id: u32, value: i32) {
        self.dirty = true;
        self.builder.push_var_sub_const(VarId(var_id), value);
    }

    fn push_var_set_mask(
        &mut self,
        var_id: u32,
        modifier: u8,
        value: i32,
        mask: u32,
    ) -> PyResult<()> {
        self.dirty = true;
        let mod_enum = decode_modifier(modifier)?;
        self.builder
            .push_var_set_mask(VarId(var_id), mod_enum, value, mask);
        Ok(())
    }

    fn push_action(&mut self, action: PyAction) {
        self.dirty = true;
        self.builder.push_action(action.0);
    }

    #[pyo3(signature = (conditions, actions, label=None, preserved=true))]
    fn push_trigger(
        &mut self,
        conditions: Vec<PyCondition>,
        actions: Vec<PyAction>,
        label: Option<String>,
        preserved: bool,
    ) {
        self.dirty = true;
        self.builder.push_trigger(
            conditions.into_iter().map(|c| c.0).collect(),
            actions.into_iter().map(|a| a.0).collect(),
            label,
            preserved,
        );
    }

    fn push_call(&mut self, callee: u32, args: Vec<PyExpr>, rets: Vec<u32>) {
        self.dirty = true;
        self.builder.push_call(
            FuncId(callee),
            args.into_iter().map(|e| e.0).collect(),
            rets.into_iter().map(VarId).collect(),
        );
    }

    fn push_return_values(&mut self, values: Vec<PyExpr>) {
        self.dirty = true;
        self.builder
            .push_return_values(values.into_iter().map(|e| e.0).collect());
    }

    #[pyo3(signature = (dst, epd, mask=0xFFFF_FFFF, shift=0, initial_val=0))]
    fn push_mem_read(&mut self, dst: u32, epd: &PyExpr, mask: u32, shift: i32, initial_val: i32) {
        self.dirty = true;
        self.builder
            .push_mem_read(VarId(dst), epd.0.clone(), mask, shift, initial_val);
    }

    #[pyo3(signature = (name, param_count=0, return_count=0, initial_target=None))]
    fn add_func_ptr(
        &mut self,
        name: &str,
        param_count: usize,
        return_count: usize,
        initial_target: Option<u32>,
    ) -> u32 {
        self.dirty = true;
        self.builder
            .add_func_ptr(
                name,
                (param_count, return_count),
                initial_target.map(FuncId),
            )
            .0
    }

    fn push_func_ptr_assign(&mut self, ptr: u32, target: u32) {
        self.dirty = true;
        self.builder
            .push_func_ptr_assign(FuncPtrId(ptr), FuncId(target));
    }

    fn push_call_indirect(&mut self, ptr: u32, args: Vec<PyExpr>, rets: Vec<u32>) {
        self.dirty = true;
        self.builder.push_call_indirect(
            FuncPtrId(ptr),
            args.into_iter().map(|e| e.0).collect(),
            rets.into_iter().map(VarId).collect(),
        );
    }

    fn dump_hir(&mut self) -> PyResult<String> {
        self.sync_hir()?;
        Ok(self.session.dump_hir())
    }

    fn dump_lir(&mut self) -> PyResult<String> {
        self.sync_hir()?;
        self.session.dump_lir().map_err(to_py_err)
    }

    fn compile(&mut self, output_path: &str) -> PyResult<PyCompilationReport> {
        self.sync_hir()?;
        let report = self.session.compile_map(output_path).map_err(to_py_err)?;
        Ok(PyCompilationReport { report })
    }
}

/// Resolves `eudkit.toml` and flag overrides into one build configuration.
///
/// Python Frontends call this instead of parsing `eudkit.toml` themselves, so
/// both CLIs share one alias and precedence rule.
#[pyfunction]
#[pyo3(signature = (config=None, input_map=None, output_map=None, entry=None, dts=None, obscurate=None, obscurate_seed=None, preserve_classic_triggers=None))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_build_request(
    py: Python<'_>,
    config: Option<PathBuf>,
    input_map: Option<PathBuf>,
    output_map: Option<PathBuf>,
    entry: Option<PathBuf>,
    dts: Option<PathBuf>,
    obscurate: Option<bool>,
    obscurate_seed: Option<u64>,
    preserve_classic_triggers: Option<bool>,
) -> PyResult<Bound<'_, PyDict>> {
    let request = BuildRequest {
        config,
        input_map,
        output_map,
        entry,
        dts,
        obscurate,
        obscurate_seed,
        preserve_classic_triggers,
        ..BuildRequest::default()
    };
    let resolved = request.resolve().map_err(to_py_err)?;
    let config = resolved.config;

    let result = PyDict::new(py);
    result.set_item("name", config.name)?;
    result.set_item("entry", path_string(config.entry.as_deref()))?;
    result.set_item("input_map", path_string(Some(config.input_map.as_path())))?;
    result.set_item("output_map", path_string(Some(config.output_map.as_path())))?;
    result.set_item("dts", path_string(config.dts.as_deref()))?;
    result.set_item("obscurate", config.obscurate)?;
    result.set_item("obscurate_seed", config.obscurate_seed)?;
    result.set_item(
        "preserve_classic_triggers",
        config.preserve_classic_triggers,
    )?;
    result.set_item("config_path", path_string(resolved.config_path.as_deref()))?;
    Ok(result)
}

fn path_string(path: Option<&Path>) -> Option<String> {
    let path = path?;
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path.to_string_lossy().into_owned())
    }
}

#[pymodule]
#[pyo3(name = "eudkit")]
pub(crate) mod eudkit_mod {
    #[pymodule_export]
    use super::{
        PyAction, PyCompilationReport, PyCompilationSession, PyCondition, PyExpr, PyMapString,
    };
}
