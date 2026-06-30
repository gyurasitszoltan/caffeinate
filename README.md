# KeepAwake

macOS menüsor utility Rustban. IOKit power assertion-nel ébren tartja a
gépet (és opcionálisan a kijelzőt), tray ikonnal, ami egy kávéscsésze
töltöttségével mutatja a hátralévõ idõt.

## Build & futtatás

```bash
cargo run --release
```

Az ikonok az `assets/` mappában vannak (`cup_0@2x.png` … `cup_4@2x.png`).
`cargo run` a `CARGO_MANIFEST_DIR`-bõl találja meg õket.

## Menü

```
KeepAwake: <állapot>          (disabled státuszsor)
─────────────────────────
Ébren tartás bekapcsolása    (idõkorlát nélkül)
Ébren tartás 15 percig
Ébren tartás 30 percig
Ébren tartás 1 óráig
Ébren tartás 2 óráig
Leállítás
─────────────────────────
Kijelzõt is ébren tartsa ✓   (checkbox)
─────────────────────────
Kilépés
```

## Állapotok / ikon frame-ek

A csésze a *hátralévõ* idõt mutatja:

| frame         | jelentés                  |
|---------------|---------------------------|
| `cup_0` (üres)  | inaktív / lejárt          |
| `cup_1`         | kevés idõ van hátra      |
| `cup_2`         | közepes idõ van hátra    |
| `cup_3`         | sok idõ van hátra        |
| `cup_4` (tele)  | frissen indított / manuális mód |

A frame a `remaining / total` arányból választódik ki (`icons.rs`).

## Ellenõrzés

Az aktív IOKit assertion látható:

```bash
pmset -g assertions
```

Aktív módban látni kell a `PreventUserIdleSystemSleep` (és kijelzõ
módnál a `PreventUserIdleDisplaySleep`) assertion-t; stop / kilépés
után eltûnik. Kilépéskor a `Drop` garantálja a release-t.

## Modulok

- `awake.rs`      — IOKit FFI + assertion életciklus (idempotens `start`/`stop`, `Drop`)
- `timer.rs`      — idõzített mód állapota
- `icons.rs`      — PNG frame cache + frame kiválasztás hátralévõbõl
- `app_state.rs`  — alkalmazás állapot (`Inactive`/`Indefinite`/`Timed`/`Expired`)
- `tray.rs`       — menü felépítés + ID -> parancs
- `main.rs`       — `tao` event loop, másodperces tick, UI frissítés

## Jegyzet

Ez a Milestone 1–6 implementációja (CLI/process-követés és `.app`
csomagolás a terv 19/20. szakaszában késõbb következhet).
