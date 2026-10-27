import json
import tempfile
import time
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TESTMAP_PATH = REPO_ROOT / "tests" / "unittests" / "testmap.scx"
BASEMAP_STRX_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"
FIXTURES_PATH = REPO_ROOT / "tests" / "fixtures" / "corpus_baselines.json"

with open(FIXTURES_PATH, "r", encoding="utf-8") as f:
    BASELINES = json.load(f)


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
                session.do(
                    SetResources(Player1, Add, 200, Ore),
                    CreateUnit(1, "Terran Marine", "Anywhere", Player1),
                )
            with session.elif_(step == 5):
                session.do(SetResources(Player1, Add, 50, Gas))
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

        b = BASELINES["Kernel 2"]
        v1_raw_triggers = b["v1_triggers"]
        v1_strx_growth = b["v1_payload_bytes"]
        v1_compile_sec = b["v1_compile_ms"] / 1000.0

        with tempfile.TemporaryDirectory() as tmpdir:
            v2_out = Path(tmpdir) / "v2_bench.scx"

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

        b = BASELINES["Kernel 1"]
        v1_raw_triggers = b["v1_triggers"]
        v1_strx_growth = b["v1_payload_bytes"]

        with tempfile.TemporaryDirectory() as tmpdir:
            v2_out = Path(tmpdir) / "v2_expr_bench.scx"

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

        b = BASELINES["Kernel 3"]
        v1_raw_triggers = b["v1_triggers"]
        v1_strx_growth = b["v1_payload_bytes"]

        with tempfile.TemporaryDirectory() as tmpdir:
            v2_out = Path(tmpdir) / "v2_fn_bench.scx"

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
        from eudkit import CUnit, dwread_epd, epdread_epd

        b = BASELINES["Kernel 4"]
        v1_raw_triggers = b["v1_triggers"]
        v1_strx_growth = b["v1_payload_bytes"]

        with tempfile.TemporaryDirectory() as tmpdir:
            v2_out = Path(tmpdir) / "v2_rt_bench.scx"

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

    def test_map_string_frontend(self):
        import eudkit

        session = eudkit.Session(BASEMAP_STRX_PATH)
        ms1 = session.map_string("Ticket #7 MapString Test")
        ms2 = session.map_string("Ticket #7 MapString Test")
        ms3 = session.map_string("Another Distinct String")

        # Deduplication assertion
        self.assertEqual(ms1.id, ms2.id)
        self.assertEqual(ms1.offset, ms2.offset)
        self.assertEqual(ms1.address, ms2.address)
        self.assertEqual(ms1.epd, ms2.epd)

        # Distinct string assertion
        self.assertNotEqual(ms1.id, ms3.id)
        self.assertNotEqual(ms1.offset, ms3.offset)

        # Static STRx address and EPD invariant assertions
        self.assertGreaterEqual(ms1.address, 0x191943C8)
        self.assertEqual(ms1.address, 0x191943C8 + ms1.offset)
        self.assertEqual(ms1.epd, (ms1.address - 0x58A364) // 4)

        # String display and format actions
        @session.fn
        def main():
            ms1.display()
            ms3.format(12345)

        hir_text = session.dump_hir()
        self.assertIn("DisplayTextId", hir_text)
        self.assertIn("FormatPrint", hir_text)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "ms_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.payload_size, 0)

    def test_f_eprintln_and_print_at_formatting(self):
        import eudkit
        from eudkit import displayText, f_eprintln, print_at

        session = eudkit.Session(BASEMAP_STRX_PATH)
        score = session.var(250, name="score")
        timer = session.var(60, name="timer")

        @session.fn
        def main():
            f_eprintln("Score: %d, Time: 0x%08X", score, timer)
            f_eprintln("Score: ", score, " pts")
            print_at(3, "Fixed Line: %d", score)
            displayText("Static display text")

        hir_text = session.dump_hir()
        self.assertIn("FormatPrint", hir_text)
        self.assertIn("PrintAt", hir_text)

        lir_text = session.dump_lir()
        # Verify CPString residue stream SetDeaths writes to CurrentPlayer (13) and 0x6509B0
        self.assertIn("p1=13", lir_text)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "print_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.trigger_count, 0)

    def test_is_user_cp_local_display(self):
        import eudkit
        from eudkit import IsUserCP, f_eprintln

        session = eudkit.Session(BASEMAP_STRX_PATH)
        score = session.var(500, name="score")

        @session.fn
        def main():
            if IsUserCP():
                f_eprintln("Local user score: %d", score)

        with session.main_loop("secondary_loop"):
            with session.if_(IsUserCP()):
                session.f_eprintln("Secondary local text: %d", score)

        hir_text = session.dump_hir()
        self.assertIn("IsUserCP()", hir_text)

        with tempfile.TemporaryDirectory() as tmpdir:
            out = Path(tmpdir) / "user_cp_test.scx"
            report = session.compile(out)
            self.assertTrue(out.exists())
            self.assertGreater(report.trigger_count, 0)

    def test_v2_vs_v1_mapstring_and_cpstring_differential_budget_benchmark(self):
        import eudkit
        from eudkit import IsUserCP, f_eprintln

        b = BASELINES["Kernel 5"]
        v1_raw_triggers = b["v1_triggers"]
        v1_strx_growth = b["v1_payload_bytes"]
        v1_compile_sec = b["v1_compile_ms"] / 1000.0

        with tempfile.TemporaryDirectory() as tmpdir:
            v2_out = Path(tmpdir) / "v2_str_bench.scx"

            # --- 2. Compile equivalent workload with eudkit (v2) ---
            t1 = time.perf_counter()
            session = eudkit.Session(BASEMAP_STRX_PATH)
            score = session.var(100, name="score")

            @session.fn
            def v2_string_main():
                f_eprintln("Score: %d", score)
                if IsUserCP():
                    f_eprintln("Local user: %d", score)

            v2_report = session.compile(v2_out)
            v2_compile_sec = time.perf_counter() - t1

            # Substantially fewer triggers: v2 unified actions vs v1 CPString/DBString apparatus
            self.assertLess(
                v2_report.trigger_count,
                v1_raw_triggers,
                f"v2 triggers ({v2_report.trigger_count}) must be substantially fewer than v1 ({v1_raw_triggers})",
            )
            # Smaller payload: v2 avoids v1's bloated 150KB+ string tables and decryption loops
            self.assertLessEqual(
                v2_report.payload_size,
                v1_strx_growth,
                f"v2 payload ({v2_report.payload_size}B) must be <= v1 STRx payload growth ({v1_strx_growth}B)",
            )
            # Faster compile time
            self.assertLessEqual(
                v2_compile_sec,
                v1_compile_sec * 1.5,
                f"v2 compile time ({v2_compile_sec:.3f}s) must be faster or comparable to v1 ({v1_compile_sec:.3f}s)",
            )

    def test_obscurate_pass_configuration_and_reproducibility(self):
        import eudkit
        from eudkit import DisplayText

        with tempfile.TemporaryDirectory() as tmpdir:
            out_plain_1 = Path(tmpdir) / "plain_1.scx"
            out_plain_2 = Path(tmpdir) / "plain_2.scx"
            out_obs_1 = Path(tmpdir) / "obs_1.scx"
            out_obs_2 = Path(tmpdir) / "obs_2.scx"
            out_obs_diff = Path(tmpdir) / "obs_diff.scx"

            # 1. Plain builds: deterministic across runs
            s_plain_1 = eudkit.Session(TESTMAP_PATH, obscurate=False)
            v1 = s_plain_1.var(42)
            @s_plain_1.fn
            def main_plain_1():
                v1 += 1
                DisplayText("Plain Mission Text")
            s_plain_1.compile(out_plain_1)

            s_plain_2 = eudkit.Session(TESTMAP_PATH, obscurate=False)
            v2 = s_plain_2.var(42)
            @s_plain_2.fn
            def main_plain_2():
                v2 += 1
                DisplayText("Plain Mission Text")
            s_plain_2.compile(out_plain_2)

            self.assertEqual(out_plain_1.read_bytes(), out_plain_2.read_bytes(), "plain builds must be deterministic")

            # 2. Obscured builds with set_obscurate(True, seed=...)
            s_obs_1 = eudkit.Session(TESTMAP_PATH)
            s_obs_1.set_obscurate(True, seed=999888)
            v_obs1 = s_obs_1.var(42)
            @s_obs_1.fn
            def main_obs_1():
                v_obs1 += 1
                DisplayText("Plain Mission Text")
            s_obs_1.compile(out_obs_1)

            s_obs_2 = eudkit.Session(TESTMAP_PATH)
            s_obs_2.set_obscurate(True)
            s_obs_2.set_obscurate_seed(999888)
            v_obs2 = s_obs_2.var(42)
            @s_obs_2.fn
            def main_obs_2():
                v_obs2 += 1
                DisplayText("Plain Mission Text")
            s_obs_2.compile(out_obs_2)

            self.assertEqual(
                out_obs_1.read_bytes(),
                out_obs_2.read_bytes(),
                "seeded obscuration builds must be byte-for-byte reproducible",
            )
            self.assertNotEqual(
                out_obs_1.read_bytes(),
                out_plain_1.read_bytes(),
                "obscured build must differ from plain build",
            )

            # 3. Different seed diverges
            s_obs_diff = eudkit.Session(TESTMAP_PATH, obscurate=True, obscurate_seed=111222)
            v_diff = s_obs_diff.var(42)
            @s_obs_diff.fn
            def main_obs_diff():
                v_diff += 1
                DisplayText("Plain Mission Text")
            s_obs_diff.compile(out_obs_diff)

            self.assertNotEqual(
                out_obs_1.read_bytes(),
                out_obs_diff.read_bytes(),
                "different seeds must produce different obscured builds",
            )

            # 4. Integrity check via eudkit Session
            obs_session = eudkit.Session(out_obs_1)
            self.assertGreater(obs_session.strx_byte_len(), 0)
            self.assertGreater(out_obs_1.stat().st_size, 1000)

    def test_reactive_loop_primitives_frontend(self):
        import eudkit
        from eudkit import (
            Add,
            CreateUnit,
            DisplayText,
            Ore,
            Player1,
            SetResources,
            after,
            every,
            once,
            until,
            when,
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            out_ast = Path(tmpdir) / "out_ast.scx"
            out_builder = Path(tmpdir) / "out_builder.scx"

            # 1. AST lowering in @session.fn
            s_ast = eudkit.Session(TESTMAP_PATH)
            kills = s_ast.var(0, name="kills")
            timer = s_ast.var(10, name="timer")
            score = s_ast.var(50, name="score")

            @s_ast.fn
            def game_loop():
                with once:
                    DisplayText("Game Initialized (once)")
                    SetResources(Player1, Add, 50, Ore)

                with once(score >= 100):
                    DisplayText("Score milestone reached!")

                with when(kills >= 10):
                    CreateUnit(1, "Terran Marine", "Anywhere", Player1)

                with until(timer == 0):
                    DisplayText("Mission in progress...")

                with after(24):
                    DisplayText("24 trigger loops elapsed")
                    SetResources(Player1, Add, 500, Ore)

                with every(12):
                    DisplayText("Clock tick (every 12 loops)")

            rep_ast = s_ast.compile(out_ast)
            self.assertTrue(out_ast.exists())
            self.assertGreater(rep_ast.trigger_count, 0)
            self.assertGreater(rep_ast.payload_size, 0)

            # 2. Builder context managers on session
            s_bld = eudkit.Session(TESTMAP_PATH)
            b_kills = s_bld.var(0, name="b_kills")
            b_timer = s_bld.var(10, name="b_timer")

            with s_bld.main_loop():
                with s_bld.once():
                    DisplayText("Builder Init")

                with s_bld.when(b_kills >= 5):
                    DisplayText("5 kills")

                with s_bld.until(b_timer == 0):
                    DisplayText("Builder Timer active")

                with s_bld.after(30):
                    DisplayText("30 loops delayed")

                with s_bld.every(10):
                    DisplayText("Periodic 10 loops")

            rep_bld = s_bld.compile(out_builder)
            self.assertTrue(out_builder.exists())
            self.assertGreater(rep_bld.trigger_count, 0)
            self.assertGreater(rep_bld.payload_size, 0)

            # 3. Determinism check
            out_ast_2 = Path(tmpdir) / "out_ast_2.scx"
            s_ast_2 = eudkit.Session(TESTMAP_PATH)
            k2 = s_ast_2.var(0, name="kills")
            t2 = s_ast_2.var(10, name="timer")
            sc2 = s_ast_2.var(50, name="score")

            @s_ast_2.fn
            def game_loop_2():
                with once:
                    DisplayText("Game Initialized (once)")
                    SetResources(Player1, Add, 50, Ore)

                with once(sc2 >= 100):
                    DisplayText("Score milestone reached!")

                with when(k2 >= 10):
                    CreateUnit(1, "Terran Marine", "Anywhere", Player1)

                with until(t2 == 0):
                    DisplayText("Mission in progress...")

                with after(24):
                    DisplayText("24 trigger loops elapsed")
                    SetResources(Player1, Add, 500, Ore)

                with every(12):
                    DisplayText("Clock tick (every 12 loops)")

            s_ast_2.compile(out_ast_2)
            self.assertEqual(out_ast.read_bytes(), out_ast_2.read_bytes(), "reactive loop compilations must be byte-for-byte deterministic")

    def test_pure_actionspec_and_no_ambient_side_effects(self):
        import eudkit
        from eudkit import (
            ActionSpec,
            Add,
            Always,
            Ore,
            Player1,
            SetDeaths,
            SetResources,
            SetTo,
            eud,
        )

        # 1. ActionSpec instantiation must have zero ambient side-effects
        session = eudkit.Session(TESTMAP_PATH)

        with session.main_loop("main"):
            # Creating ActionSpecs inside a session scope must not emit actions
            act1 = SetResources(Player1, Add, 100, Ore)
            act2 = SetDeaths(Player1, SetTo, 5, "Terran Marine")
            self.assertIsInstance(act1, ActionSpec)
            self.assertIsInstance(act2, ActionSpec)

        # With no explicit session.do(...) or trigger, HIR has 0 actions
        hir_after_instantiation = session.dump_hir()
        self.assertNotIn("SetResources", hir_after_instantiation)
        self.assertNotIn("SetDeaths", hir_after_instantiation)

        # 2. session.trigger must cleanly resolve actions without double-emission
        session2 = eudkit.Session(TESTMAP_PATH)
        with session2.main_loop("main"):
            session2.trigger(
                Always(),
                [SetResources(Player1, Add, 100, Ore)],
            )

        hir_trigger = session2.dump_hir()
        # Verify exactly 1 SetResources appears in HIR
        self.assertEqual(hir_trigger.count("SetResources"), 1)

        # 3. session.do explicitly resolves and pushes actions
        session3 = eudkit.Session(TESTMAP_PATH)
        with session3.main_loop("main"):
            session3.do(
                SetResources(Player1, Add, 200, Ore),
                SetDeaths(Player1, SetTo, 10, "Terran Marine"),
            )

        hir_do = session3.dump_hir()
        self.assertEqual(hir_do.count("SetResources"), 1)
        self.assertEqual(hir_do.count("SetDeaths"), 1)

        # 4. Global _in_ast_lowering is eliminated
        import eudkit.types as types
        import eudkit.ast_lowering as ast_lowering
        self.assertFalse(hasattr(types, "_in_ast_lowering"))
        self.assertFalse(hasattr(ast_lowering, "_in_ast_lowering"))


if __name__ == "__main__":
    unittest.main()

