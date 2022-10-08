from ..core import (
    EUDVariable,
    SeqCompute,
    SetVariables,
    EUDVArray,
    IsEUDVariable,
    f_bitlshift,
    SetTo,
)
from ..maprw import EUDOnStart
from ..utils import ExprProxy, FlattenList, List2Assignable
from ..eudlib import EUDArray
from ..ctrlstru import EUDIf, EUDElse, EUDEndIf
from .epsimp import EPSLoader
from ..core.variable.eudv import IsRValue


def _RELIMP(path, mod_name):  # relative path import
    import inspect, pathlib, importlib.util

    p = pathlib.Path(inspect.getabsfile(inspect.currentframe())).parent
    for s in path.split("."):
        if s == "":
            p = p.parent
        else:
            p = p / s
    try:
        spec = importlib.util.spec_from_file_location(mod_name, p / (mod_name + ".py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
    except FileNotFoundError:
        loader = EPSLoader(mod_name, str(p / (mod_name + ".eps")))
        spec = importlib.util.spec_from_loader(mod_name, loader)
        module = loader.create_module(spec)
        loader.exec_module(module)
    return module


def _IGVA(vList, exprListGen):
    def _():
        exprList = exprListGen()
        SetVariables(vList, exprList)

    EUDOnStart(_)


def _CGFW(exprf, retn):
    rets = [ExprProxy(None) for _ in range(retn)]

    def _():
        vals = exprf()
        for ret, val in zip(rets, vals):
            ret._value = val

    EUDOnStart(_)
    return rets


def _ARR(items):  # EUDArray initialization
    k = EUDArray(len(items))
    for i, item in enumerate(items):
        k[i] = item
    return k


def _VARR(items):  # EUDVArray initialization
    k = EUDVArray(len(items))()
    for i, item in enumerate(items):
        k[i] = item
    return k


def _SRET(v, klist):
    return List2Assignable([v[k] for k in klist])


def _SV(dL, sL):
    [d << s for d, s in zip(FlattenList(dL), FlattenList(sL))]


class _ATTW:  # attribute write
    def __init__(self, obj, attrName):
        self.obj = obj
        self.attrName = attrName

    def __lshift__(self, r):
        setattr(self.obj, self.attrName, r)

    def __iadd__(self, v):
        try:
            self.obj.iaddattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov += v
            setattr(self.obj, self.attrName, ov)

    def __isub__(self, v):
        try:
            self.obj.isubattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov -= v
            setattr(self.obj, self.attrName, ov)

    def __imul__(self, v):
        try:
            self.obj.imulattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov *= v
            setattr(self.obj, self.attrName, ov)

    def __ifloordiv__(self, v):
        try:
            self.obj.ifloordivattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov //= v
            setattr(self.obj, self.attrName, ov)

    def __imod__(self, v):
        try:
            self.obj.imodattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov %= v
            setattr(self.obj, self.attrName, ov)

    def __ilshift__(self, v):
        try:
            self.obj.ilshiftattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov <<= v
            setattr(self.obj, self.attrName, ov)

    def __irshift__(self, v):
        try:
            self.obj.irshiftattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov >>= v
            setattr(self.obj, self.attrName, ov)

    def __ipow__(self, v):
        try:
            self.obj.ipowattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov **= v
            setattr(self.obj, self.attrName, ov)

    def __iand__(self, v):
        try:
            self.obj.iandattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov &= v
            setattr(self.obj, self.attrName, ov)

    def __ior__(self, v):
        try:
            self.obj.iorattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov |= v
            setattr(self.obj, self.attrName, ov)

    def __ixor__(self, v):
        try:
            self.obj.ixorattr(self.attrName, v)
        except AttributeError:
            ov = getattr(self.obj, self.attrName)
            ov ^= v
            setattr(self.obj, self.attrName, ov)


class _ARRW:  # array write
    def __init__(self, obj, index):
        self.obj = obj
        self.index = index

    def __lshift__(self, r):
        self.obj[self.index] = r

    def __iadd__(self, v):
        try:
            self.obj.iadditem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov += v
            self.obj[self.index] = ov

    def __isub__(self, v):
        try:
            self.obj.isubitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov -= v
            self.obj[self.index] = ov

    def __imul__(self, v):
        try:
            self.obj.imulitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov *= v
            self.obj[self.index] = ov

    def __ifloordiv__(self, v):
        try:
            self.obj.ifloordivitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov //= v
            self.obj[self.index] = ov

    def __imod__(self, v):
        try:
            self.obj.imoditem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov %= v
            self.obj[self.index] = ov

    def __ilshift__(self, v):
        try:
            self.obj.ilshiftitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov <<= v
            self.obj[self.index] = ov

    def __irshift__(self, v):
        try:
            self.obj.irshiftitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov >>= v
            self.obj[self.index] = ov

    def __ipow__(self, v):
        try:
            self.obj.ipowitem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov **= v
            self.obj[self.index] = ov

    def __iand__(self, v):
        try:
            self.obj.ianditem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov &= v
            self.obj[self.index] = ov

    def __ior__(self, v):
        try:
            self.obj.ioritem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov |= v
            self.obj[self.index] = ov

    def __ixor__(self, v):
        try:
            self.obj.ixoritem(self.index, v)
        except AttributeError:
            ov = self.obj[self.index]
            ov ^= v
            self.obj[self.index] = ov


def _L2V(l):  # logic to value
    ret = EUDVariable()
    if EUDIf()(l):
        ret << 1
    if EUDElse()():
        ret << 0
    EUDEndIf()
    return ret


def _LVAR(vs):
    ret, ops = [], []
    for v in FlattenList(vs):
        if IsEUDVariable(v) and IsRValue(v):
            ret.append(v.makeL())
        else:
            nv = EUDVariable()
            ret.append(nv)
            ops.append((nv, SetTo, v))
    if ops:
        SeqCompute(ops)
    return List2Assignable(ret)


def _LSH(l, r):
    if IsEUDVariable(l):
        return f_bitlshift(l, r)
    else:
        return l << r
