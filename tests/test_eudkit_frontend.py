import tempfile
import time
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TESTMAP_PATH = REPO_ROOT / "tests" / "unittests" / "testmap.scx"
BASEMAP_STRX_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


class TestEudkitPythonFrontend(unittest.TestCase):
    def test_isolated_sessions_and_map_context(self):
        import eudkit

        s1 = eudkit.Session(TESTMAP_PATH)
        s2 = eudkit.Session(BASEMAP_STRX_PATH)

        self.assertEqual(s1.map_context.location_id("Anywhere"), 64)
        self.assertEqual(s1.map_context.unit_id("Terran Marine"), 0)
        self.assertEqual(s1.map_context.unit_id("Zerg Zergling"), 37)
        self.assertEqual(s1.map_context.switch_id("Switch 1"), 0)

        v1 = s1.var(10, name="only_in_s1")
        v2 = s2.var(20, name="only_in_s2")

        @s1.fn
        def main_s1():
            v1 += 5

        @s2.fn
        def main_s2():
            v2 += 99
            eudkit.DisplayText("Hello s2")

        self.assertIn("only_in_s1", s1.dump_hir())
        self.assertNotIn("only_in_s2", s1.dump_hir())
        self.assertIn("only_in_s2", s2.dump_hir())
        self.assertNotIn("only_in_s1", s2.dump_hir())

    def test_eud_fn_control_flow_4b_var_lowering_and_action_coalescing(self):
        import eudkit
        from eudkit import (
            Add,
            AtLeast,
            DisplayText,
            Ore,
            Player1,
            SetDeaths,
            SetResources,
            SetTo,
            eud,
        )

        session = eudkit.Session(TESTMAP_PATH)
        counter = session.var(0, name="counter")
        phase = session.var(10, name="phase")
        flags = session.var(0x00FF, name="flags")

        @eud.fn
        def tick():
            # Compile-time unrolled loop: 68 unconditional actions -> coalesced into 64 + 4
            for i in range(68):
                counter += 1

            if counter >= 50 and (flags & 0xFF) == 0xFF:
                counter = 0
                SetResources(Player1, Add, 100, Ore)
            elif counter == 25 or counter == 26:
                SetDeaths(Player1, SetTo, 1, "Terran Marine")
            else:
                counter += 2

            while phase >= 1:
                phase -= 1
                if phase == 5:
                    counter += 10
                    continue
                if counter >= 500:
                    break
                counter += 1

            DisplayText("Tick done")

        session.add_fn(tick, main_loop=True)

        hir_text = session.dump_hir()
        self.assertIn("var @counter", hir_text)
        self.assertIn("var @phase", hir_text)
        self.assertIn("while", hir_text)

        lir_text = session.dump_lir()
        self.assertIn("ByteBuf @var_counter", lir_text)
        self.assertIn("ByteBuf @var_phase", lir_text)
        self.assertIn("ByteBuf @var_flags", lir_text)

        with tempfile.TemporaryDirectory() as tmpdir:
            out1 = Path(tmpdir) / "out1.scx"
            out2 = Path(tmpdir) / "out2.scx"

            report1 = session.compile(out1)

            # Second session with identical program to verify byte-for-byte determinism
            session2 = eudkit.Session(TESTMAP_PATH)
            c2 = session2.var(0, name="counter")
            p2 = session2.var(10, name="phase")
            f2 = session2.var(0x00FF, name="flags")
            session2.add_fn(tick, main_loop=True)
            report2 = session2.compile(out2)

            self.assertEqual(report1.vproc_count, 0)
            self.assertEqual(report1.byte_buf_count, 3)
            self.assertEqual(out1.read_bytes(), out2.read_bytes())
            self.assertEqual(report1.trigger_count, report2.trigger_count)
            self.assertEqual(report1.payload_size, report2.payload_size)

    def test_with_block_builder_api(self):
        import eudkit
        from eudkit import Add, CreateUnit, Gas, Ore, Player1, SetResources

        session = eudkit.Session(TESTMAP_PATH)
        step = session.var(0, name="step")
        limit = session.var(5, name="limit")

        with session.main_loop():
            for delta in range(1, 6):
                step += delta

            with session.if_(step >= 10):
                SetResources(Player1, Add, 200, Ore)
                CreateUnit(1, "Terran Marine", "Anywhere", Player1)
            with session.elif_(step == 5):
                SetResources(Player1, Add, 50, Gas)
            with session.else_():
                step += 1

            with session.while_(limit >= 1):
                limit -= 1
                with session.if_(limit == 2):
                    session.continue_()
                with session.if_(step >= 100):
                    session.break_()
                step += 2

        with tempfile.TemporaryDirectory() as tmpdir:
            out_map = Path(tmpdir) / "builder_out.scx"
            report = session.compile(out_map)
            self.assertEqual(report.vproc_count, 0)
            self.assertEqual(report.byte_buf_count, 2)
            self.assertTrue(out_map.exists())

    def test_v2_vs_v1_differential_budget_gate(self):
        import eudkit
        from eudkit import Add, Ore, Player1, SetDeaths, SetResources
        import eudplib as ep

        with tempfile.TemporaryDirectory() as tmpdir:
            v1_out = Path(tmpdir) / "v1_bench.scx"
            v2_out = Path(tmpdir) / "v2_bench.scx"

            # --- 1. Compile workload with legacy eudplib (v1) ---
            t0 = time.perf_counter()
            ep.LoadMap(str(BASEMAP_STRX_PATH))
            ep.CompressPayload(True)

            @ep.EUDFunc
            def v1_main():
                a = ep.EUDVariable(0)
                b = ep.EUDVariable(20)
                c = ep.EUDVariable(0)

                for i in range(40):
                    a += 1
                    ep.DoActions(ep.SetDeaths(ep.Player1, ep.Add, 1, "Terran Marine"))

                if ep.EUDIf()(a >= 30):
                    c << 100
                    ep.DoActions(ep.SetResources(ep.Player1, ep.Add, 500, ep.Ore))
                if ep.EUDElseIf()(a == 15):
                    c << 50
                if ep.EUDElse()():
                    c << 10
                ep.EUDEndIf()

                if ep.EUDWhile()(b >= 1):
                    b -= 1
                    if ep.EUDIf()(b == 5):
                        a += 10
                        ep.EUDContinue()
                    ep.EUDEndIf()
                    if ep.EUDIf()(a >= 200):
                        ep.EUDBreak()
                    ep.EUDEndIf()
                    a += 2
                ep.EUDEndWhile()

            ep.SaveMap(str(v1_out), v1_main)
            v1_compile_sec = time.perf_counter() - t0

            # Inspect v1 output CHK sections for trigger + payload size comparison
            v1_verify_session = eudkit.Session(v1_out)
            v1_base_session = eudkit.Session(BASEMAP_STRX_PATH)
            v1_strx_growth = v1_verify_session.strx_byte_len() - v1_base_session.strx_byte_len()
            v1_raw_triggers = sum(
                1
                for obj in ep.core.allocator.payload._found_objects_dict
                if isinstance(obj, ep.RawTrigger)
            )

            # --- 2. Compile equivalent workload with eudkit (v2) ---
            t1 = time.perf_counter()
            session = eudkit.Session(BASEMAP_STRX_PATH)
            a = session.var(0, name="a")
            b = session.var(20, name="b")
            c = session.var(0, name="c")

            @session.fn
            def v2_main():
                for i in range(40):
                    a += 1
                    SetDeaths(Player1, Add, 1, "Terran Marine")

                if a >= 30:
                    c = 100
                    SetResources(Player1, Add, 500, Ore)
                elif a == 15:
                    c = 50
                else:
                    c = 10

                while b >= 1:
                    b -= 1
                    if b == 5:
                        a += 10
                        continue
                    if a >= 200:
                        break
                    a += 2

            v2_report = session.compile(v2_out)
            v2_compile_sec = time.perf_counter() - t1

            self.assertEqual(v2_report.vproc_count, 0)
            self.assertLessEqual(
                v2_report.payload_size,
                v1_strx_growth,
                f"v2 payload ({v2_report.payload_size}B) must be <= v1 STRx payload growth ({v1_strx_growth}B)",
            )
            self.assertLessEqual(
                v2_report.trigger_count,
                v1_raw_triggers,
                f"v2 triggers ({v2_report.trigger_count}) must be <= v1 triggers ({v1_raw_triggers})",
            )
            self.assertLessEqual(
                v2_compile_sec,
                v1_compile_sec,
                f"v2 compile time ({v2_compile_sec:.4f}s) must be <= v1 compile time ({v1_compile_sec:.4f}s)",
            )

    def test_dynamic_vproc_storage_lowering_compound_exprs_and_field_refs(self):
        import eudkit
        from eudkit import (
            Add,
            Deaths,
            Exactly,
            Memory,
            Player1,
            SetDeaths,
            SetMemory,
            SetTo,
            eud,
        )

        session = eudkit.Session(TESTMAP_PATH)
        const_only = session.var(100, name="const_only")
        a = session.var(10, name="a")
        b = session.var(20, name="b")
        c = session.var(3, name="c")
        out = session.var(0, name="out")

        @session.fn
        def main_exprs():
            # Constant-only updates on `const_only` -> stays in 4-byte ByteBuf cell!
            const_only += 10 + 20 * 2

            # Direct variable copy: out = a
            out = a

            # Compound arithmetic & bitwise expression with constant folding and scratch reuse:
            # (a + b) + (a - c) + (b & 0xFF) + (c << 2)
            out = (a + b) + (a - c) + (b & 0xFF) + (c << 2)

            # Bitwise XOR, OR, NOT, Negation, Shift Right, Mul/Div/Rem by constants & vars:
            out = ((a ^ b) | (~c)) + (-a) + (b >> 1) + (a * 3) + (b // 5) + (b % 5)

            # Augmented assignments with expressions:
            out += a - c
            out -= b & 0x0F
            out &= a | 0xFF
            out |= c << 1
            out ^= a + 1

            # Dynamic variable comparisons (`==`, `!=`, `>=`, `<=`, `>`, `<`)
            if a >= b and a != c:
                const_only += 1
            elif a < b or a > c or a <= b or a == c:
                const_only += 2

            # Dynamic condition/action arguments patched via FieldRef!
            if Deaths(a, Exactly, b, "Terran Marine") and Memory(0x58A364, Exactly, c):
                SetDeaths(a, SetTo, b + 1, "Terran Marine")
                SetMemory(0x58A364, Add, c)

        lir_text = session.dump_lir()
        # `const_only` and `out` are never read as dynamic copy sources -> 4-byte ByteBuf cells!
        self.assertIn("ByteBuf @var_const_only", lir_text)
        self.assertIn("ByteBuf @var_out", lir_text)
        # `a`, `b`, `c` and reused scratch slots are in the unified VProcBuffer!
        self.assertIn("[VProcBuffer]", lir_text)

        with tempfile.TemporaryDirectory() as tmpdir:
            out_map = Path(tmpdir) / "vproc_expr_out.scx"
            report = session.compile(out_map)
            self.assertGreaterEqual(report.vproc_count, 3)
            self.assertLessEqual(
                report.vproc_count,
                5,
                f"Expected 3 dynamic vars + 1..2 reused scratch VProc slots, got {report.vproc_count}",
            )
            self.assertEqual(report.byte_buf_count, 2)
            self.assertTrue(out_map.exists())

    def test_v2_vs_v1_dynamic_vproc_and_compound_expressions_budget_gate(self):
        import eudkit
        from eudkit import Exactly, Player1, SetDeaths, SetTo
        import eudplib as ep

        with tempfile.TemporaryDirectory() as tmpdir:
            v1_out = Path(tmpdir) / "v1_expr_bench.scx"
            v2_out = Path(tmpdir) / "v2_expr_bench.scx"

            # --- 1. Compile dynamic expression workload with legacy eudplib (v1) ---
            ep.LoadMap(str(BASEMAP_STRX_PATH))
            ep.CompressPayload(True)

            @ep.EUDFunc
            def v1_expr_main():
                a = ep.EUDVariable(10)
                b = ep.EUDVariable(20)
                c = ep.EUDVariable(5)
                d = ep.EUDVariable(0)
                const_counter = ep.EUDVariable(0)

                const_counter += 15
                d << (a + b) + (a - c) + (b & 0xFF)
                d << (a ^ b) | (c & 0x0F)

                if ep.EUDIf()(a >= b):
                    d << a + c
                if ep.EUDElse()():
                    d << b - c
                ep.EUDEndIf()

                if ep.EUDIf()(ep.Deaths(Player1, ep.Exactly, a, "Terran Marine")):
                    ep.DoActions(ep.SetDeaths(Player1, ep.SetTo, d, "Terran Marine"))
                ep.EUDEndIf()

            ep.SaveMap(str(v1_out), v1_expr_main)

            v1_verify_session = eudkit.Session(v1_out)
            v1_base_session = eudkit.Session(BASEMAP_STRX_PATH)
            v1_strx_growth = (
                v1_verify_session.strx_byte_len() - v1_base_session.strx_byte_len()
            )
            v1_raw_triggers = sum(
                1
                for obj in ep.core.allocator.payload._found_objects_dict
                if isinstance(obj, ep.RawTrigger)
            )

            # --- 2. Compile equivalent workload with eudkit (v2) ---
            session = eudkit.Session(BASEMAP_STRX_PATH)
            a = session.var(10, name="a")
            b = session.var(20, name="b")
            c = session.var(5, name="c")
            d = session.var(0, name="d")
            const_counter = session.var(0, name="const_counter")

            @session.fn
            def v2_expr_main():
                const_counter += 15
                d = (a + b) + (a - c) + (b & 0xFF)
                d = (a ^ b) | (c & 0x0F)

                if a >= b:
                    d = a + c
                else:
                    d = b - c

                if eudkit.Deaths(Player1, Exactly, a, "Terran Marine"):
                    SetDeaths(Player1, SetTo, d, "Terran Marine")

            v2_report = session.compile(v2_out)

            self.assertLessEqual(
                v2_report.payload_size,
                v1_strx_growth,
                f"v2 payload ({v2_report.payload_size}B) must be <= v1 STRx payload growth ({v1_strx_growth}B)",
            )
            self.assertLessEqual(
                v2_report.trigger_count,
                v1_raw_triggers,
                f"v2 triggers ({v2_report.trigger_count}) must be <= v1 triggers ({v1_raw_triggers})",
            )


    def test_hir_function_inlining_zero_overhead(self):
        import eudkit

        session = eudkit.Session(BASEMAP_STRX_PATH)
        res = session.var(0, name="res")

        @session.fn(inline=True)
        def add_five(x):
            return x + 5

        @session.fn
        def main():
            res = add_five(10)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "inlining_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)
            hir_text = session.dump_hir()
            self.assertNotIn("call @add_five", hir_text)

    def test_non_inlined_function_direct_return_convention(self):
        import eudkit

        session = eudkit.Session(BASEMAP_STRX_PATH)
        out1 = session.var(0, name="out1")
        out2 = session.var(0, name="out2")

        @session.fn(inline=False)
        def calc_stats(x, y):
            return x * 2, y + 10

        @session.fn
        def main():
            out1, out2 = calc_stats(15, 25)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "direct_return_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)
            lir_text = session.dump_lir()
            self.assertIn("VProcBuffer", lir_text)
            self.assertIn("calc_stats_ret_0", lir_text)
            self.assertIn("calc_stats_ret_1", lir_text)

    def test_func_ptr_dynamic_dispatch_frontend(self):
        import eudkit

        session = eudkit.Session(BASEMAP_STRX_PATH)
        r1 = session.var(0, name="r1")
        r2 = session.var(0, name="r2")

        @session.fn(inline=False)
        def step_add(x):
            return x + 10

        @session.fn(inline=False)
        def step_mul(x):
            return x * 4

        fp = session.func_ptr(1, 1, init=step_add, name="op_step")

        @session.fn
        def main():
            r1 = fp(5)
            fp.set(step_mul)
            r2 = fp(5)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "func_ptr_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)
            lir_text = session.dump_lir()
            self.assertIn("fp_op_step_trampoline", lir_text)
            self.assertIn("fp_op_step_ret_0", lir_text)

    def test_v2_vs_v1_function_and_funcptr_differential_budget_benchmark(self):
        import eudkit
        import eudplib as ep

        with tempfile.TemporaryDirectory() as tmpdir:
            v1_out = Path(tmpdir) / "v1_fn_bench.scx"
            v2_out = Path(tmpdir) / "v2_fn_bench.scx"

            # --- 1. Compile workload with legacy eudplib (v1) ---
            ep.LoadMap(str(BASEMAP_STRX_PATH))
            ep.CompressPayload(True)

            @ep.EUDFunc
            def v1_worker(x):
                ep.EUDReturn(x + 7)

            @ep.EUDFunc
            def v1_mul(x):
                ep.EUDReturn(x * 3)

            v1_fptr = ep.EUDFuncPtr(1, 1)(v1_worker)

            def v1_fn_main():
                res1 = ep.EUDVariable(0)
                res2 = ep.EUDVariable(0)
                res1 << v1_fptr(10)
                v1_fptr << v1_mul
                res2 << v1_fptr(10)

            ep.SaveMap(str(v1_out), v1_fn_main)

            v1_verify_session = eudkit.Session(v1_out)
            v1_base_session = eudkit.Session(BASEMAP_STRX_PATH)
            v1_strx_growth = (
                v1_verify_session.strx_byte_len() - v1_base_session.strx_byte_len()
            )
            v1_raw_triggers = sum(
                1
                for obj in ep.core.allocator.payload._found_objects_dict
                if isinstance(obj, ep.RawTrigger)
            )

            # --- 2. Compile equivalent workload with eudkit (v2) ---
            session = eudkit.Session(BASEMAP_STRX_PATH)
            res1 = session.var(0, name="res1")
            res2 = session.var(0, name="res2")

            @session.fn(inline=False)
            def v2_worker(x):
                return x + 7

            @session.fn(inline=False)
            def v2_mul(x):
                return x * 3

            v2_fptr = session.func_ptr(1, 1, init=v2_worker, name="v2_fptr")

            @session.fn
            def v2_fn_main():
                res1 = v2_fptr(10)
                v2_fptr.set(v2_mul)
                res2 = v2_fptr(10)

            v2_report = session.compile(v2_out)

            self.assertLessEqual(
                v2_report.payload_size,
                v1_strx_growth,
                f"v2 payload ({v2_report.payload_size}B) must be <= v1 STRx payload growth ({v1_strx_growth}B)",
            )
            self.assertLessEqual(
                v2_report.trigger_count,
                v1_raw_triggers,
                f"v2 triggers ({v2_report.trigger_count}) must be <= v1 triggers ({v1_raw_triggers})",
            )

    def test_read_table_frontend_memio(self):
        import eudkit
        from eudkit import (
            EPD,
            bread_epd,
            dwread_epd,
            epdread_epd,
            wread_epd,
        )

        session = eudkit.Session(BASEMAP_STRX_PATH)
        val_dw = session.var(0, name="val_dw")
        val_epd = session.var(0, name="val_epd")
        val_w = session.var(0, name="val_w")
        val_b = session.var(0, name="val_b")

        @session.fn
        def memio_main():
            # Constant EPD reads
            val_dw = dwread_epd(EPD(0x59CCA8))
            val_epd = epdread_epd(EPD(0x59CCA8 + 0x00C))
            val_w = wread_epd(EPD(0x59CCA8 + 0x028), 0)
            val_b = bread_epd(EPD(0x59CCA8 + 0x020), 0)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "memio_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)
            lir_text = session.dump_lir()
            self.assertIn("ReadTable", lir_text)

    def test_cunit_and_csprite_frontend(self):
        import eudkit
        from eudkit import CSprite, CUnit

        session = eudkit.Session(BASEMAP_STRX_PATH)
        unit = CUnit(0, session=session)
        unit_hp = session.var(0, name="unit_hp")
        sprite_pos = session.var(0, name="sprite_pos")

        # 1. Compile-time rejection of unsupported SC:R fields
        with self.assertRaises(AttributeError) as ctx:
            _ = unit.order_queue_head
        self.assertIn("StarCraft: Remastered does not support", str(ctx.exception))

        with self.assertRaises(AttributeError) as ctx:
            _ = unit.path
        self.assertIn("StarCraft: Remastered does not support", str(ctx.exception))

        with self.assertRaises(AttributeError) as ctx:
            _ = unit.AI
        self.assertIn("StarCraft: Remastered does not support", str(ctx.exception))

        with self.assertRaises(AttributeError) as ctx:
            _ = unit.contour_bounds_lt
        self.assertIn("StarCraft: Remastered does not support", str(ctx.exception))

        # 2. Reading and writing supported fields inside @session.fn
        @session.fn
        def unit_ops():
            unit.hp = 10000
            unit.pos = 0x01000100
            unit.remove_collision()
            unit.set_invincible()
            unit_hp = unit.hp
            sprite = unit.sprite
            sprite_pos = sprite.pos

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "cunit_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)

    def test_array_and_varray_indexing(self):
        import eudkit
        from eudkit import Array, VArray

        session = eudkit.Session(BASEMAP_STRX_PATH)
        arr = Array(10, epd=0x58A364 // 4, session=session)
        varr = VArray(8, epd=0x58A364 // 4, session=session)
        idx = session.var(2, name="idx")
        read_val = session.var(0, name="read_val")

        @session.fn
        def array_main():
            # Constant index write and read
            arr[1] = 500
            varr[0] = 777

            # Dynamic index write and read
            arr[idx] = 1234
            varr[idx] = 5678
            read_val = arr[idx]
            read_val = varr[idx]

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "array_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)

    def test_v2_vs_v1_readtable_and_entities_budget_gate(self):
        import eudkit
        import eudplib as ep
        from eudkit import CUnit, dwread_epd, epdread_epd

        with tempfile.TemporaryDirectory() as tmpdir:
            v1_out = Path(tmpdir) / "v1_rt_bench.scx"
            v2_out = Path(tmpdir) / "v2_rt_bench.scx"

            # --- 1. Compile workload with legacy eudplib (v1) ---
            ep.LoadMap(str(BASEMAP_STRX_PATH))
            ep.CompressPayload(True)

            def v1_rt_main():
                cunit_epd = ep.EPD(0x59CCA8)
                v1 = ep.f_dwread_epd(cunit_epd + 2)
                v2 = ep.f_epdread_epd(cunit_epd + 3)
                v3 = ep.f_wread_epd(cunit_epd + 10, 0)
                v4 = ep.f_bread_epd(cunit_epd + 19, 1)
                ep.f_dwwrite_epd(cunit_epd + 2, 500)
                ep.f_dwwrite_epd(cunit_epd + 10, 0x02000200)

            ep.SaveMap(str(v1_out), v1_rt_main)

            v1_verify_session = eudkit.Session(v1_out)
            v1_base_session = eudkit.Session(BASEMAP_STRX_PATH)
            v1_strx_growth = (
                v1_verify_session.strx_byte_len() - v1_base_session.strx_byte_len()
            )
            v1_raw_triggers = sum(
                1
                for obj in ep.core.allocator.payload._found_objects_dict
                if isinstance(obj, ep.RawTrigger)
            )

            # --- 2. Compile equivalent workload with eudkit (v2) ---
            session = eudkit.Session(BASEMAP_STRX_PATH)
            unit = CUnit(0, session=session)
            v1 = session.var(0, name="v1")
            v2 = session.var(0, name="v2")
            v3 = session.var(0, name="v3")
            v4 = session.var(0, name="v4")

            @session.fn
            def v2_rt_main():
                v1 = unit.hp
                v2 = epdread_epd(unit.epd + 3)
                v3 = unit.posX
                v4 = unit.order
                unit.hp = 500
                unit.pos = 0x02000200

            v2_report = session.compile(v2_out)

            self.assertLessEqual(
                v2_report.payload_size,
                v1_strx_growth,
                f"v2 payload ({v2_report.payload_size}B) must be <= v1 STRx payload growth ({v1_strx_growth}B)",
            )
            self.assertLessEqual(
                v2_report.trigger_count,
                v1_raw_triggers,
                f"v2 triggers ({v2_report.trigger_count}) must be <= v1 triggers ({v1_raw_triggers})",
            )


if __name__ == "__main__":
    unittest.main()
