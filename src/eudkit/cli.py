from __future__ import annotations

import argparse
import sys

from . import _rust as _rust_eudkit
from .runner import run_build


def _resolve_request(**overrides: object) -> dict[str, object]:
    """Resolves flags and ``eudkit.toml`` through the shared BuildRequest."""
    return _rust_eudkit.resolve_build_request(**overrides)


def main(argv: list[str] | None = None) -> None:
    if argv is None:
        argv = sys.argv[1:]

    parser = argparse.ArgumentParser(
        prog="eudkit",
        description="eudkit StarCraft EUD compilation framework",
    )
    subparsers = parser.add_subparsers(dest="subcommand", help="Subcommand to execute")

    build_parser = subparsers.add_parser("build", help="Build a StarCraft map from source")
    build_parser.add_argument("--config", "-c", help="Path to eudkit.toml configuration file")
    build_parser.add_argument("--input", "-i", help="Path to input basemap (.scx/.scm)")
    build_parser.add_argument("--output", "-o", help="Path to target output map (.scx)")
    build_parser.add_argument("--entry", "-e", help="Path to entry script (.py/.ts)")
    build_parser.add_argument("--obscurate", action="store_true", help="Enable bytecode obscuration")
    build_parser.add_argument("--seed", type=int, help="Deterministic seed for obscuration")

    dump_parser = subparsers.add_parser("dump-ir", help="Dump Intermediate Representation (HIR/LIR)")
    dump_parser.add_argument("--config", "-c", help="Path to eudkit.toml configuration file")
    dump_parser.add_argument("--map", "-m", "--input", "-i", dest="input", help="Path to input basemap")
    dump_parser.add_argument("--entry", "-e", help="Path to entry script")
    dump_parser.add_argument("--hir-only", action="store_true", help="Dump HIR only")
    dump_parser.add_argument("--lir-only", action="store_true", help="Dump LIR only")

    args = parser.parse_args(argv)

    if args.subcommand == "build":
        try:
            resolved = _resolve_request(
                config=args.config,
                input_map=args.input,
                output_map=args.output,
                entry=args.entry,
                obscurate=True if args.obscurate else None,
                obscurate_seed=args.seed,
            )
        except Exception as e:
            print(f"error: failed to resolve build configuration: {e}", file=sys.stderr)
            sys.exit(1)

        input_map = resolved["input_map"]
        output_map = resolved["output_map"]
        if not input_map or not output_map:
            print(
                "usage: eudkit build [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <script>]",
                file=sys.stderr,
            )
            sys.exit(1)

        try:
            report = run_build(
                input_map=input_map,
                output_map=output_map,
                entry=resolved["entry"],
                obscurate=resolved["obscurate"],
                obscurate_seed=resolved["obscurate_seed"],
                preserve_classic_triggers=resolved["preserve_classic_triggers"],
            )
            if report is not None:
                print(report.summary(str(output_map)))
        except Exception as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.subcommand == "dump-ir":
        try:
            resolved = _resolve_request(
                config=args.config,
                input_map=args.input,
                entry=args.entry,
            )
        except Exception as e:
            print(f"error: failed to resolve build configuration: {e}", file=sys.stderr)
            sys.exit(1)

        input_map = resolved["input_map"]
        if not input_map:
            print("usage: eudkit dump-ir --map <map.scx> [--entry <script>]", file=sys.stderr)
            sys.exit(1)
        try:
            run_build(
                input_map=input_map,
                entry=resolved["entry"],
                obscurate=resolved["obscurate"],
                obscurate_seed=resolved["obscurate_seed"],
                preserve_classic_triggers=resolved["preserve_classic_triggers"],
                dump_hir=not args.lir_only,
                dump_lir=not args.hir_only,
            )
        except Exception as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
