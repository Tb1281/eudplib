#!/usr/bin/python
# -*- coding: utf-8 -*-
# Copyright 2014 by trgk.
# All rights reserved.
# This file is part of EUD python library (eudplib), and is released under "MIT License Agreement".
# Please see the LICENSE file that should have been included as part of this package.

from abc import ABCMeta, abstractmethod
from typing import TYPE_CHECKING, Literal, overload, TypeVar

from eudplib import utils as ut
from eudplib.localize import _

from .. import rawtrigger as bt
from ..allocator import ConstExpr, IsConstExpr

if TYPE_CHECKING:
    from ..rawtrigger.constenc import Dword

Self = TypeVar("Self", bound="VariableBase")


class VariableBase(metaclass=ABCMeta):
    @abstractmethod
    def __init__(self) -> None:
        pass

    @abstractmethod
    def getValueAddr(self) -> ConstExpr:
        raise NotImplementedError()

    # -------

    def AtLeast(self, value: "Dword") -> bt.Condition:
        return bt.Memory(self.getValueAddr(), bt.AtLeast, value)

    def AtMost(self, value: "Dword") -> bt.Condition:
        return bt.Memory(self.getValueAddr(), bt.AtMost, value)

    def Exactly(self, value: "Dword") -> bt.Condition:
        return bt.Memory(self.getValueAddr(), bt.Exactly, value)

    # -------

    def SetNumber(self, value: "Dword") -> bt.Action:
        return bt.SetMemory(self.getValueAddr(), bt.SetTo, value)

    def AddNumber(self, value: "Dword") -> bt.Action:
        return bt.SetMemory(self.getValueAddr(), bt.Add, value)

    def SubtractNumber(self, value: "Dword") -> bt.Action:
        return bt.SetMemory(self.getValueAddr(), bt.Subtract, value)

    # -------

    def AtLeastX(self, value: "Dword", mask: "Dword") -> bt.Condition:
        return bt.MemoryX(self.getValueAddr(), bt.AtLeast, value, mask)

    def AtMostX(self, value: "Dword", mask: "Dword") -> bt.Condition:
        return bt.MemoryX(self.getValueAddr(), bt.AtMost, value, mask)

    def ExactlyX(self, value: "Dword", mask: "Dword") -> bt.Condition:
        return bt.MemoryX(self.getValueAddr(), bt.Exactly, value, mask)

    # -------

    def SetNumberX(self, value: "Dword", mask: "Dword") -> bt.Action:
        return bt.SetMemoryX(self.getValueAddr(), bt.SetTo, value, mask)

    def AddNumberX(self, value: "Dword", mask: "Dword") -> bt.Action:
        return bt.SetMemoryX(self.getValueAddr(), bt.Add, value, mask)

    def SubtractNumberX(self, value: "Dword", mask: "Dword") -> bt.Action:
        return bt.SetMemoryX(self.getValueAddr(), bt.Subtract, value, mask)

    # -------

    def Assign(self, value: "Dword") -> None:
        bt.RawTrigger(actions=bt.SetMemory(self.getValueAddr(), bt.SetTo, value))

    def __lshift__(self, value: "Dword") -> None:
        self.Assign(value)

    def __iadd__(self: Self, value: "Dword") -> Self:
        bt.RawTrigger(actions=bt.SetMemory(self.getValueAddr(), bt.Add, value))
        return self

    def __isub__(self: Self, value: "Dword") -> Self:
        bt.RawTrigger(actions=bt.SetMemory(self.getValueAddr(), bt.Subtract, value))
        return self

    # -------

    # See: https://github.com/heinermann/llvm-bw/wiki/Instruction-Implementation

    def __ior__(self: Self, value: "Dword") -> Self:
        bt.RawTrigger(actions=self.SetNumberX(0xFFFFFFFF, value))
        return self

    def __iand__(self: Self, value: int) -> Self:
        bt.RawTrigger(actions=self.SetNumberX(0, ~value))
        return self

    def __ixor__(self: Self, value: "Dword") -> Self:
        bt.RawTrigger(
            actions=[
                self.AddNumberX(value, 0x55555555),  # 5 = 0b0101
                self.AddNumberX(value, 0xAAAAAAAA),  # A = 0b1010
            ]
        )
        return self

    def iinvert(self: Self) -> Self:
        "In-place invert (x << ~x)"
        return self.__ixor__(0xFFFFFFFF)

    @overload
    def ineg(self: Self, *, action: Literal[False] = False) -> Self:
        ...

    @overload
    def ineg(self: Self, *, action: Literal[True]) -> list[bt.Action]:
        ...

    def ineg(self: Self, *, action: bool = False) -> Self | list[bt.Action]:
        "In-place negate (x << -x)"
        actions = [
            self.AddNumberX(0xFFFFFFFF, 0x55555555),
            self.AddNumberX(0xFFFFFFFF, 0xAAAAAAAA),
            self.AddNumber(1),
        ]
        if action:
            return actions
        else:
            bt.RawTrigger(actions=actions)
            return self

    def iabs(self: Self) -> Self:
        bt.RawTrigger(
            conditions=self >= 0x80000000,
            actions=self.ineg(action=True),
        )
        return self

    def __ilshift__(self: Self, n: int) -> Self:  # type: ignore[misc]
        mask = (1 << (n + 1)) - 1
        bt.RawTrigger(
            actions=[
                [
                    self.SetNumberX(0, (mask >> 1) << (t + 1)),
                    self.AddNumberX((mask >> 1) << t, mask << t),
                ]
                for t in reversed(range(32 - n))
            ]
            + [self.SetNumberX(0, mask >> 1)]  # lowest n bits
        )
        return self

    def __irshift__(self: Self, n: int) -> Self:
        mask = (1 << (n + 1)) - 1
        bt.RawTrigger(
            actions=[self.SetNumberX(0, mask >> 1)]  # lowest n bits
            + [self.SubtractNumberX((mask >> 1) << t, mask << t) for t in range(32 - n)]
        )
        return self

    # -------

    def __eq__(self, other) -> bt.Condition:  # type: ignore[override]
        return self.Exactly(other)

    def __ne__(self, other) -> bt.Condition:  # type: ignore[override]
        if isinstance(other, int):
            if other & 0xFFFFFFFF == 0:
                return self.AtLeast(1)
            if other & 0xFFFFFFFF == 0xFFFFFFFF:
                return self.AtMost(0xFFFFFFFE)
        return NotImplemented

    def __le__(self, other: "Dword") -> bt.Condition:
        return self.AtMost(other)

    def __lt__(self, other) -> bt.Condition:
        if IsConstExpr(other):
            return self.AtMost(other - 1)
        return NotImplemented

    def __ge__(self, other: "Dword") -> bt.Condition:
        return self.AtLeast(other)

    def __gt__(self, other) -> bt.Condition:
        if IsConstExpr(other):
            return self.AtLeast(other + 1)
        return NotImplemented
