#!/usr/bin/python
# -*- coding: utf-8 -*-

'''
Copyright (c) 2014 trgk

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
'''

from ... import core as c
from ... import ctrlstru as cs
from ... import varfunc as vf


def bw_gen(cond):
    @vf.EUDFunc
    def f_bitsize_template(a, b):
        tmp = vf.EUDLightVariable()
        ret = vf.EUDVariable()

        ret << 0

        for i in range(31, -1, -1):
            c.Trigger(
                conditions=[
                    a.AtLeast(2 ** i)
                ],
                actions=[
                    tmp.AddNumber(1),
                    a.SubtractNumber(2 ** i)
                ]
            )

            c.Trigger(
                conditions=[
                    b.AtLeast(2 ** i)
                ],
                actions=[
                    tmp.AddNumber(1),
                    b.SubtractNumber(2 ** i)
                ]
            )

            c.Trigger(
                conditions=cond(tmp),
                actions=ret.AddNumber(2 ** i)
            )

            cs.DoActions(tmp.SetNumber(0))

        return ret

    return f_bitsize_template


f_bitand = bw_gen(lambda x: x.Exactly(2))
f_bitor = bw_gen(lambda x: x.AtLeast(1))
f_bitxor = bw_gen(lambda x: x.Exactly(1))

f_bitnand = bw_gen(lambda x: x.Exactly(0))
f_bitnor = bw_gen(lambda x: x.AtMost(1))


@vf.EUDFunc
def f_bitnxor(a, b):
    return f_bitnot(f_bitxor(a, b))


def f_bitnot(a):
    return 0xFFFFFFFF - a
