# Scenario 13: In-Game Chat Detection — Architecture Comparison

## 1. Scenario Intent
Monitors engine chat buffers (`0x58D900` for sender ID, `0x58D904` for message head) every trigger cycle:
- Checks if a chat message has been submitted (`msg_head != 0`).
- Captures the sender ID into `last_sender` and increments `chat_count`.
- Evaluates whether Player 1 (`id == 0`) sent the message, toggling `p1_lamp`.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Unconditionally reads both sender and message slots every single frame
sender = f_dwread_epd(CHAT_SENDER_EPD)
msg_head = f_dwread_epd(CHAT_MSG_EPD)

if EUDIf()(msg_head != 0):
    last_sender << sender
    chat_count << chat_count + 1

    if EUDIf()(sender == 0):
        p1_lamp << 1
    if EUDElse()():
        p1_lamp << 0
    EUDEndIf()
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Lazy reading: evaluates message arrival first, avoiding unnecessary reads
if dwread_epd(CHAT_MSG_EPD) != 0:
    sender = dwread_epd(CHAT_SENDER_EPD)
    last_sender = sender
    chat_count += 1

    if sender == 0:
        p1_lamp = 1
    else:
        p1_lamp = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 11 T (call sites only) | 104 T (standalone) | **9.45x** | Isolated call sites vs full engine |
| **Idle Runtime Reads** | 2 reads / frame (always) | **1 read / frame** | **0.50x** | **50% runtime read reduction** |
| **VProc Allocation** | 4 VProcs (288 B) | 1 VProc (72 B) | **0.25x** | **-75% VProcs** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | — | Zero hidden engine dependencies |

---

## 4. Architectural Mechanics

### Lazy Evaluation & Locality
- **Legacy v1 (Eager Inefficiency)**: Legacy code eagerly executes two `f_dwread_epd` calls on *every frame*, regardless of whether anyone is chatting. This burns CPU cycles inside StarCraft's trigger engine even during idle gameplay.
- **Modern v2 (Lazy Guarding)**: Modern `eudkit` guards the sender read inside `if dwread_epd(CHAT_MSG_EPD) != 0:`. When no chat is being dispatched (which is true for 99.9% of game frames), the engine skips the sender read entirely.

### Storage Lowering
- Variables `chat_count`, `last_sender`, and `p1_lamp` are optimized through ByteBuf lowering, leaving only 1 scratch VProc for dynamic conditional comparisons.
