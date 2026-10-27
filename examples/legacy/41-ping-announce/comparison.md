# 41 - Minimap Ping + Warning with Cooldown Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Ambient Cooldown State & Monolithic Nested Conditions
In legacy eudplib (`legacy.py`), building an intruder alert system across three defensive zones suffered from classic state scattering:
- **Global Mutable Counter Variables**: Three global variables (`watch_cd`, `gate_cd`, `rear_cd`) were declared at module scope, exposing countdown state globally.
- **Nested Conditional Boilerplate**: Every zone check required manually nesting two `EUDIf` blocks: `if Bring: if cd == 0: cd << 168`, generating separate conditional evaluation triggers.
- **Fragile Action Sequences**: In legacy, actions were queued via explicit `DoActions([MinimapPing(...), DisplayText(...)])` calls, lacking type checking or static map symbol validation.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with First-Class MinimapPing and Scoped State
In modern eudkit (`v2.py`), the security monitoring loop is self-contained:
- **First-Class `MinimapPing` Action**: `MinimapPing(zone)` is a typed stock trigger action that emits action opcode 28 with zero intermediate VProcs.
- **Scoped Countdown Management**: `watch_cd`, `gate_cd`, and `rear_cd` are allocated directly in the compilation session and lowered to constant-only storage cells with **0 VProcs**.
- **Compact Trigger Footprint**: The entire 3-zone intrusion and cooldown system compiles to just 13 triggers and 4,024 bytes payload.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | 16 | **13** | **-18.8%** |
| **VProcs** | 3 | **0** | **-100.0%** |
| **Payload Size** | ~5,600 bytes | **4,024 bytes** | **-28.1%** |
| **State Locality** | 3 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (13) <= v1_triggers (16)`
- `v2_payload_bytes (4,024) <= v1_payload_bytes (5,600)`
