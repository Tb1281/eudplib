from __future__ import annotations

import argparse
import runpy
import sys
from pathlib import Path
from typing import Any

from .session import Session


def run_build(
    input_map: str | Path,
    output_map: str | Path | None = None,
    entry: str | Path | None = None,
    *,
    obscurate: bool = False,
    obscurate_seed: int | None = None,
    preserve_classic_triggers: bool = True,
    dump_hir: bool = False,
    dump_lir: bool = False,
) -> Any:
    input_path = Path(input_map).resolve()
    if not input_path.exists():
        raise FileNotFoundError(f"Input map not found: {input_path}")

    sess = Session(
        str(input_path),
        obscurate=obscurate,
        obscurate_seed=obscurate_seed,
        preserve_classic_triggers=preserve_classic_triggers,
    )

    with sess:
        if entry is not None:
            entry_path = Path(entry).resolve()
            if not entry_path.exists():
                raise FileNotFoundError(f"Entry script not found: {entry_path}")

            entry_dir = str(entry_path.parent)
            if entry_dir not in sys.path:
                sys.path.insert(0, entry_dir)

            runpy.run_path(str(entry_path), run_name="__main__")

        if dump_hir:
            print(sess.dump_hir())
            return None

        if dump_lir:
            print(sess.dump_lir())
            return None

        if output_map is None:
            raise ValueError("output_map must be specified when compiling")

        output_path = Path(output_map).resolve()
        if sess._compiled_report is not None:
            return sess._compiled_report

        return sess.compile(str(output_path))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="eudkit Python compilation runner",
        prog="python -m eudkit.runner",
    )
    parser.add_argument(
        "--input", "-i", required=True, help="Path to input basemap (.scx/.scm)"
    )
    parser.add_argument(
        "--output", "-o", help="Path to target output map (.scx)"
    )
    parser.add_argument(
        "--entry", "-e", required=True, help="Path to Python script (.py)"
    )
    parser.add_argument(
        "--obscurate", action="store_true", help="Enable bytecode obscuration"
    )
    parser.add_argument(
        "--seed", type=int, help="Deterministic seed for obscuration"
    )
    parser.add_argument(
        "--preserve-classic-triggers",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Preserve classic map triggers (default: true)",
    )
    parser.add_argument(
        "--report",
        metavar="PATH",
        help="Write the compilation report to this path for the Rust CLI",
    )
    parser.add_argument(
        "--dump-hir", action="store_true", help="Dump High-Level IR (HIR)"
    )
    parser.add_argument(
        "--dump-lir", action="store_true", help="Dump Low-Level IR (LIR)"
    )

    args = parser.parse_args()

    try:
        report = run_build(
            input_map=args.input,
            output_map=args.output,
            entry=args.entry,
            obscurate=args.obscurate,
            obscurate_seed=args.seed,
            preserve_classic_triggers=args.preserve_classic_triggers,
            dump_hir=args.dump_hir,
            dump_lir=args.dump_lir,
        )
        if report is not None:
            print(
                f"Built {args.output} (triggers={report.trigger_count}, "
                f"payload_bytes={report.payload_size}, entry={report.payload_entry_addr:#010X})"
            )
            if args.report:
                _write_report(Path(args.report), report)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def _write_report(path: Path, report: Any) -> None:
    """Writes the key=value report consumed by the Rust CLI."""
    lines = [
        f"trigger_count={report.trigger_count}",
        f"byte_buf_count={report.byte_buf_count}",
        f"vproc_count={report.vproc_count}",
        f"payload_offset={report.payload_offset}",
        f"payload_size={report.payload_size}",
        f"payload_entry_addr={report.payload_entry_addr}",
        f"prt_offsets={','.join(str(offset) for offset in report.prt_offsets)}",
        f"ort_offsets={','.join(str(offset) for offset in report.ort_offsets)}",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
