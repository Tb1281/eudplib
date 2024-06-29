## NOTE: THIS FILE IS GENERATED BY EPSCRIPT! DO NOT MODITY
from eudplib import *
from eudplib.core.eudfunc import EUDTraceLog, EUDTracedFunc, EUDTracedTypedFunc, EUDTracedMethod, EUDTracedTypedMethod
from eudplib.epscript.helper import  _RELIMP, _IGVA, _CGFW, _ARR, _VARR, _SRET, _SV, _ATTW, _ARRW, _ATTC, _ARRC, _L2V, _LVAR, _LSH, _ALL
# (Line 1) import eudplib.eudlib.stringf.tblprint;
from eudplib.eudlib.stringf import tblprint
# (Line 2) const inputData = py_bytes(1000);
inputData = _CGFW(lambda: [bytes(1000)], 1)[0]
# (Line 3) tblprint._AddStatText(inputData);
tblprint._AddStatText(inputData)
# (Line 5) const expected_result = py_str("abcdeArmo\xE2\x80\x89\0");
expected_result = _CGFW(lambda: [str("abcdeArmo\xE2\x80\x89\0")], 1)[0]
# (Line 6) function test_stattext() {
@EUDFunc
def f_test_stattext():
    # (Line 7) const armo = Db("Armo");
    armo = Db("Armo")
    # (Line 8) const stattext = GetTBLAddr(1);
    stattext = GetTBLAddr(1)
    # (Line 9) settblf(1, 0, "abcde{:s}", armo, encoding="UTF-8");
    f_settblf(1, 0, "abcde{:s}", armo, encoding="UTF-8")
    # (Line 10) const ret = py_list();
    ret = list()
    # (Line 11) const br = EUDByteReader();
    br = EUDByteReader()
    # (Line 12) br.seekoffset(stattext);
    br.seekoffset(stattext)
    # (Line 13) foreach(char : expected_result) {
    for char in expected_result:
        # (Line 14) ret.append(br.readbyte());
        ret.append(br.readbyte())
        # (Line 15) }
        # (Line 16) return List2Assignable(ret);

    EUDReturn(List2Assignable(ret))
    # (Line 17) }
