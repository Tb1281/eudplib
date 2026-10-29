# Replace C++ epScript with a TypeScript Frontend and Migration CLI

`eudkit` removes the C++ `libepScriptLib` Lemon parser, `pybind11` submodule, and Python bytecode line-table rewriter in favor of a first-class TypeScript Frontend (`@eudkit/sdk`) and a one-time `.eps` migration tool (`eudkit migrate`).

Maintaining a bespoke C++-to-Python transpiler for `.eps` files required a split CMake + Maturin build pipeline and left mapmakers without standard IDE language server support (autocomplete, type checking, refactoring, or `MapContext` schema validation). Standard TypeScript parsed via Rust (`oxc`) into `HIR` provides the exact C-family syntax `.eps` users want while giving them zero-cost access to the TypeScript type checker and IDE tooling.
