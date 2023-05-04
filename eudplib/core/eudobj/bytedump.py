#!/usr/bin/python
# Copyright 2014 by trgk.
# All rights reserved.
# This file is part of EUD python library (eudplib),
# and is released under "MIT License Agreement". Please see the LICENSE
# file that should have been included as part of this package.

from .eudobj import EUDObject
from ..allocator.payload import _PayloadBuffer


class Db(EUDObject):
    """Class for raw data object"""

    def __init__(self, b: bytes | int | str) -> None:
        super().__init__()
        if isinstance(b, str):
            b = b.encode("UTF-8") + b"\0"
        self.content: bytes = bytes(b)

    def GetDataSize(self) -> int:
        return len(self.content)

    def WritePayload(self, pbuffer: _PayloadBuffer) -> None:
        pbuffer.WriteBytes(self.content)
