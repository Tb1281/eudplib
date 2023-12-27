#!/usr/bin/python
# Copyright 2014 by trgk.
# All rights reserved.
# This file is part of EUD python library (eudplib),
# and is released under "MIT License Agreement". Please see the LICENSE
# file that should have been included as part of this package.

import builtins
import keyword
import types

__version__ = "0.76.13"

from .core import (
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    TBL,
    Accumulate,
    Action,
    Add,
    AddCurrentPlayer,
    All,
    AlliedVictory,
    Allies,
    AllPlayers,
    Ally,
    Always,
    AtLeast,
    AtMost,
    Attack,
    Bring,
    Buildings,
    CenterView,
    Clear,
    Cleared,
    Command,
    CommandLeast,
    CommandLeastAt,
    CommandMost,
    CommandMostAt,
    Comment,
    CompressPayload,
    Condition,
    ConstExpr,
    CountdownTimer,
    CreatePayload,
    CreateUnit,
    CreateUnitWithProperties,
    CurrentPlayer,
    Custom,
    Db,
    Deaths,
    DeathsX,
    Defeat,
    Disable,
    Disabled,
    DisplayText,
    Draw,
    ElapsedTime,
    Enable,
    EncodeAIScript,
    EncodeAllyStatus,
    EncodeComparison,
    EncodeCount,
    EncodeFlingy,
    EncodeIcon,
    EncodeImage,
    EncodeIscript,
    EncodeLocation,
    EncodeModifier,
    EncodeOrder,
    EncodePlayer,
    EncodePortrait,
    EncodeProperty,
    EncodePropState,
    EncodeResource,
    EncodeScore,
    EncodeSprite,
    EncodeString,
    EncodeSwitch,
    EncodeSwitchAction,
    EncodeSwitchState,
    EncodeTBL,
    EncodeTech,
    EncodeUnit,
    EncodeUnitOrder,
    EncodeUpgrade,
    EncodeWeapon,
    Enemy,
    EP_SetRValueStrictMode,
    EUDClearNamespace,
    EUDCreateVariables,
    EUDFunc,
    EUDFuncPtr,
    EUDLightBool,
    EUDLightVariable,
    EUDMethod,
    EUDObject,
    EUDRegistered,
    EUDRegisterObjectToNamespace,
    EUDReturn,
    EUDStruct,
    EUDTypedFunc,
    EUDTypedFuncPtr,
    EUDTypedMethod,
    EUDVariable,
    EUDVArray,
    EUDXVariable,
    Evaluate,
    Exactly,
    Flingy,
    Foes,
    Force1,
    Force2,
    Force3,
    Force4,
    Forward,
    Gas,
    GetChkTokenized,
    GetEUDNamespace,
    GetLocationIndex,
    GetObjectAddr,
    GetPlayerInfo,
    GetPropertyIndex,
    GetStringIndex,
    GetSwitchIndex,
    GetTriggerCounter,
    GetUnitIndex,
    GiveUnits,
    HighestScore,
    Icon,
    Image,
    IsConstExpr,
    Iscript,
    IsEUDVariable,
    IsMapdataInitialized,
    Kills,
    KillsAndRazings,
    KillUnit,
    KillUnitAt,
    LeaderBoardComputerPlayers,
    LeaderBoardControl,
    LeaderBoardControlAt,
    LeaderBoardGoalControl,
    LeaderBoardGoalControlAt,
    LeaderBoardGoalKills,
    LeaderBoardGoalResources,
    LeaderBoardGoalScore,
    LeaderBoardGreed,
    LeaderBoardKills,
    LeaderBoardResources,
    LeaderBoardScore,
    LeastKills,
    LeastResources,
    LowestScore,
    Memory,
    MemoryEPD,
    MemoryX,
    MemoryXEPD,
    MinimapPing,
    ModifyUnitEnergy,
    ModifyUnitHangarCount,
    ModifyUnitHitPoints,
    ModifyUnitResourceAmount,
    ModifyUnitShields,
    MostKills,
    MostResources,
    Move,
    MoveLocation,
    MoveUnit,
    MuteUnitSpeech,
    NeutralPlayers,
    Never,
    NextTrigger,
    NonAlliedVictoryPlayers,
    NonSeqCompute,
    Opponents,
    Order,
    Ore,
    OreAndGas,
    Patrol,
    PauseGame,
    PauseTimer,
    Player1,
    Player2,
    Player3,
    Player4,
    Player5,
    Player6,
    Player7,
    Player8,
    Player9,
    Player10,
    Player11,
    Player12,
    PlayWAV,
    PopTriggerScope,
    Portrait,
    PreserveTrigger,
    PushTriggerScope,
    Random,
    RawTrigger,
    Razings,
    RegisterCreatePayloadCallback,
    RemoveUnit,
    RemoveUnitAt,
    RlocInt,
    RlocInt_C,
    RunAIScript,
    RunAIScriptAt,
    Score,
    SeqCompute,
    Set,
    SetAllianceStatus,
    SetCountdownTimer,
    SetCurrentPlayer,
    SetDeaths,
    SetDeathsX,
    SetDoodadState,
    SetInvincibility,
    SetKills,
    SetMemory,
    SetMemoryEPD,
    SetMemoryX,
    SetMemoryXEPD,
    SetMissionObjectives,
    SetNextPtr,
    SetNextScenario,
    SetNextTrigger,
    SetResources,
    SetScore,
    SetSwitch,
    SetTo,
    SetVariables,
    ShufflePayload,
    Sprite,
    StatText,
    Subtract,
    Switch,
    TalkingPortrait,
    Tech,
    Toggle,
    Total,
    Transmission,
    TrgAIScript,
    TrgAllyStatus,
    TrgComparison,
    TrgCount,
    TrgLocation,
    TrgModifier,
    TrgOrder,
    TrgPlayer,
    TrgProperty,
    TrgPropState,
    TrgResource,
    TrgScore,
    TrgString,
    TrgSwitch,
    TrgSwitchAction,
    TrgSwitchState,
    TrgUnit,
    UnitOrder,
    UnitProperty,
    Units,
    UnitsAndBuildings,
    UnMuteUnitSpeech,
    UnpauseGame,
    UnpauseTimer,
    Upgrade,
    Victory,
    VProc,
    Wait,
    Weapon,
    f_bitand,
    f_bitlshift,
    f_bitnand,
    f_bitnor,
    f_bitnot,
    f_bitnxor,
    f_bitor,
    f_bitrshift,
    f_bitsplit,
    f_bitxor,
    f_div,
    f_mul,
    selftype,
    toRlocInt,
)
from .ctrlstru import *
from .epscript import (
    EPS_SetDebug,
    EPSLoader,
    IsSCDBMap,
    epsCompile,
)
from .eudlib import *
from .maprw import *
from .offsetmap import CSprite, CUnit, EPDCUnitMap
from .trigger import *
from .trigtrg.runtrigtrg import (
    GetFirstTrigTrigger,
    GetLastTrigTrigger,
    RunTrigTrigger,
    TrigTriggerBegin,
    TrigTriggerEnd,
)
from .utils import *

# remove modules from __all__
_old_globals = [
    "keyword",
    "__file__",
    "types",
    "__doc__",
    "__version__",
    "builtins",
    "__cached__",
    "__name__",
    "__loader__",
    "__spec__",
    "__path__",
    "__package__",
    "__builtins__",
]
_alllist = []
for _k, _v in dict(globals()).items():
    if _k in _old_globals:
        continue
    elif _k != "stocktrg" and isinstance(_v, types.ModuleType):
        continue
    elif _k[0] == "_":
        continue
    _alllist.append(_k)

__all__ = _alllist

del _k
del _v


def eudplibVersion():
    return __version__


_alllist.append("eudplibVersion")


from .epscript import epscompile

epscompile._set_eps_globals(_alllist)
epscompile._set_py_keywords(keyword.kwlist)
epscompile._set_py_builtins(dir(builtins))
