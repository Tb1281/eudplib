from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .runner import run_build


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
        input_map = args.input
        output_map = args.output
        entry = args.entry
        obscurate = args.obscurate
        seed = args.seed

        # Check for eudkit.toml if not all provided
        if args.config or (Path("eudkit.toml").exists() and (not input_map or not output_map or not entry)):
            config_file = Path(args.config) if args.config else Path("eudkit.toml")
            if config_file.exists():
                import tomllib
                try:
                    with open(config_file, "rb") as f:
                        data = tomllib.load(f)
                    proj = data.get("project", {})
                    bld = data.get("build", {})
                    if not input_map:
                        input_map = bld.get("input_map") or proj.get("input_map")
                    if not output_map:
                        output_map = bld.get("output_map") or proj.get("output_map")
                    if not entry:
                        entry = bld.get("entry") or proj.get("entry")
                    if not obscurate:
                        obscurate = bld.get("obscurate", False) or proj.get("obscurate", False)
                    if seed is None:
                        seed = bld.get("obscurate_seed") or proj.get("obscurate_seed")
                except Exception as e:
                    print(f"error: failed to parse config {config_file}: {e}", file=sys.stderr)
                    sys.exit(1)

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
                entry=entry,
                obscurate=obscurate,
                obscurate_seed=seed,
            )
            if report is not None:
                print(
                    f"Built {output_map} (triggers={report.trigger_count}, "
                    f"payload_bytes={report.payload_size}, entry={report.payload_entry_addr:#010X})"
                )
        except Exception as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.subcommand == "dump-ir":
        input_map = args.input
        entry = args.entry
        if not input_map:
            print("usage: eudkit dump-ir --map <map.scx> [--entry <script>]", file=sys.stderr)
            sys.exit(1)
        try:
            run_build(
                input_map=input_map,
                entry=entry,
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
