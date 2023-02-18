## NOTE: THIS FILE IS GENERATED BY EPSCRIPT! DO NOT MODITY
from eudplib import *
from eudplib.epscript.helper import _RELIMP, _IGVA, _CGFW, _ARR, _VARR, _SRET, _SV, _ATTW, _ARRW, _ATTC, _ARRC, _L2V, _LVAR, _LSH
# (Line 1) import .BGM_eps_test;
# (Line 2) import .test_eps_stattext as stat;
BGM_eps_test = _RELIMP(".", "BGM_eps_test")
# (Line 3) var x = 1 << 4;
stat = _RELIMP(".", "test_eps_stattext")
x = _IGVA(1, lambda: [_LSH(1,4)])
# (Line 4) EUDOnStart(function () { x += x; });
@EUDFunc
def _lambda1():
    x.__iadd__(x)

EUDOnStart(_lambda1)
# (Line 5) function test_compatibility() {
@EUDFunc
def f_test_compatibility():
    # (Line 6) static var ret = 0;
    ret = EUDVariable(0)
    # (Line 7) const empty = Db(i2b4(0));
    empty = Db(i2b4(0))
    # (Line 8) const cond = Forward();
    cond = Forward()
    # (Line 10) py_exec("from helper import *\n\
    # (Line 19) ");
    exec("from helper import *\nwith expect_eperror():\n    Trigger(cond, ret.AddNumber(1 << 0))\nwith expect_eperror():\n    Trigger(empty, ret.AddNumber(1 << 1))\nwith expect_eperror():\n    Trigger(empty + 1, ret.AddNumber(1 << 2))\nwith expect_eperror():\n    SetVariables(ret, -1, EUDVariable(EncodeModifier(SetTo)))\n")
    # (Line 20) py_exec("from helper import *\n\
    # (Line 28) ");
    exec("from helper import *\nwith expect_eperror():\n    SetVariables(EUDVariable(), 1)\nwith expect_eperror():\n    SetVariables(f_dwread_epd(0), 1)\npv = PVariable()\nwith expect_eperror():\n    SetVariables(pv[0], 1)\n")
    # (Line 29) const tc = GetTriggerCounter();
    tc = GetTriggerCounter()
    # (Line 30) var x0 = EUDVariable(0);
    x0 = _LVAR([EUDVariable(0)])
    # (Line 31) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 33) var x1 = list(EUDVariable(0));
    x1 = _LVAR([FlattenList([EUDVariable(0)])])
    # (Line 34) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 36) var x2 = list(list(EUDVariable(0)));
    x2 = _LVAR([FlattenList([FlattenList([EUDVariable(0)])])])
    # (Line 37) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 39) var x3 = ExprProxy(EUDVariable(0));
    x3 = _LVAR([ExprProxy(EUDVariable(0))])
    # (Line 40) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 42) var x4 = ExprProxy(ExprProxy(EUDVariable(0)));
    x4 = _LVAR([ExprProxy(ExprProxy(EUDVariable(0)))])
    # (Line 43) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 45) var x5, y5 = EUDVariable(0), EUDVariable(0);
    x5, y5 = _LVAR([EUDVariable(0), EUDVariable(0)])
    # (Line 46) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 48) var x6, y6 = list(EUDVariable(0), EUDVariable(0));
    x6, y6 = _LVAR([FlattenList([EUDVariable(0), EUDVariable(0)])])
    # (Line 49) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 51) var x7, y7, z7 = EUDCreateVariables(3);
    x7, y7, z7 = _LVAR([EUDCreateVariables(3)])
    # (Line 52) ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy");
    ep_assert(tc == GetTriggerCounter(), "Fail to elide var copy")
    # (Line 54) const vlist = EUDCreateVariables(3);
    vlist = EUDCreateVariables(3)
    # (Line 55) var x8, y8, z8 = vlist;
    x8, y8, z8 = _LVAR([vlist])
    # (Line 56) ep_assert(tc != GetTriggerCounter(), "Wrongly elide var copy");
    ep_assert(EUDNot(tc == GetTriggerCounter()), "Wrongly elide var copy")
    # (Line 58) const tc2 = GetTriggerCounter();
    tc2 = GetTriggerCounter()
    # (Line 59) var x9, y9, z9 = list(vlist[2], vlist[1], vlist[0]);
    x9, y9, z9 = _LVAR([FlattenList([vlist[2], vlist[1], vlist[0]])])
    # (Line 60) ep_assert(tc2 != GetTriggerCounter(), "Wrongly elide var copy");
    ep_assert(EUDNot(tc2 == GetTriggerCounter()), "Wrongly elide var copy")
    # (Line 62) cond.__lshift__(Memory(empty, AtLeast, 1));
    cond.__lshift__(Memory(empty, AtLeast, 1))
    # (Line 63) if (cond) { ret += 1 << 3; }
    if EUDIf()(cond):
        ret.__iadd__(_LSH(1,3))
        # (Line 64) ret += x;
    EUDEndIf()
    ret.__iadd__(x)
    # (Line 65) if(Is64BitWireframe()) {}
    if EUDIf()(Is64BitWireframe()):
        # (Line 66) var z = EUDVariable();
        pass
    EUDEndIf()
    z = _LVAR([EUDVariable()])
    # (Line 67) return ret;
    EUDReturn(ret)
    # (Line 68) }
    # (Line 69) class ParticleBag extends EUDStruct {

# (Line 70) object {
class ParticleBag(EUDStruct):
    # (Line 71) var a;
    _subobject_ = [
        # (Line 72) const b;
        'a',
        # (Line 73) };
        ('b', None, 'const'),
        # (Line 74) var a;

    ]
    # (Line 75) var b;
    # (Line 76) }
    _fields_ = [
        'a',
        'b',
    ]
