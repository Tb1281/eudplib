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


if __name__ == "__main__":
    unittest.main()
