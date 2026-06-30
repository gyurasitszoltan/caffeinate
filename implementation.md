# KeepAwake macOS app – implementációs terv

## 1. Cél

Egy Rustban írt macOS utility app készítése, amely a `caffeinate` működéséhez hasonlóan ébren tartja a gépet.

A fő célok:

- a Mac ne menjen alvó módba, amíg a felhasználó szeretné;
- opcionálisan a kijelző se aludjon el;
- a felső macOS menüsorban legyen egy ikon;
- az ikonból nyíljon menü;
- lehessen időzített ébren tartást indítani, például 30 percre;
- az ikon vizuálisan jelezze a hátralévő időt, például egy kávéscsésze töltöttségével;
- kilépéskor minden rendszer-szintű sleep blokkolás szabályosan fel legyen oldva.

## 2. Platform és technológia

Célplatform:

- macOS

Nyelv:

- Rust

Fő natív macOS API:

- IOKit Power Management Assertion API

Fő Rust crate-ek:

```toml
[dependencies]
core-foundation = "0.10"
libc = "0.2"
ctrlc = "3"
tao = "0.34"
tray-icon = "0.21"
image = "0.25"
```

Megjegyzés: a pontos crate verziókat implementáláskor érdemes frissíteni és ellenőrizni, de az architektúra ettől nem változik.

## 3. Fő funkciók

### 3.1. Rendszer ébren tartása

A gép alvását az IOKit `IOPMAssertionCreateWithName(...)` függvényével kell megakadályozni.

Használt assertion típus:

```text
PreventUserIdleSystemSleep
```

Ez azt jelenti, hogy a gép normál idle állapotból nem fog elaludni, amíg az assertion aktív.

### 3.2. Kijelző ébren tartása

Opcionális funkcióként használható:

```text
PreventUserIdleDisplaySleep
```

Ez a kijelző idle sleep állapotát akadályozza meg.

Javasolt UI opció:

```text
Kijelzőt is ébren tartsa ✓
```

Ez egy checkbox menüpont legyen.

### 3.3. Időzített ébren tartás

Támogatandó időzítések:

- 15 perc
- 30 perc
- 1 óra
- 2 óra
- egyéni későbbi opcióként

A korábbi konkrét példa alapján a fontos use case:

```text
Ébren tartás 30 percig
```

Indítás után:

- létrejön az IOKit assertion;
- elindul egy timer;
- a tray tooltip mutatja a hátralévő időt;
- az ikon frame-je a hátralévő idő alapján változik;
- lejáratkor az assertion feloldódik;
- az ikon inaktív állapotra vált.

### 3.4. Manuális ébren tartás

Legyen olyan mód is, amikor időkorlát nélkül aktív:

```text
Ébren tartás bekapcsolása
```

Ez addig tartson, amíg a felhasználó ki nem kapcsolja vagy ki nem lép az appból.

### 3.5. Processzhez kötött mód későbbi bővítésként

Hasznos későbbi funkció:

```bash
keepawake --cmd "pi agent"
```

Ebben az esetben az app addig tartja ébren a gépet, amíg a megadott processz fut.

Másik lehetséges CLI mód:

```bash
keepawake --pid <PID>
```

Ez a `caffeinate -w <PID>` viselkedéséhez hasonló.

## 4. macOS tray / status bar ikon

macOS-en a felső jobb menüsorban megjelenő ikon technikailag status bar item.

Rust oldalon javasolt megoldás:

- `tray-icon` crate a tray/status ikonhoz;
- `tao` event loop az eseménykezeléshez.

Fontos szabály:

- macOS-en az event loop fusson a main threaden;
- a tray ikon létrehozása az event loop inicializációjához kötődjön;
- az ikon és menü eseményeit az event loopban kell feldolgozni.

## 5. Tray menü terv

Első verziós menü:

```text
KeepAwake: Inaktív / Aktív / Hátralévő idő
────────────────────────
Ébren tartás bekapcsolása
Ébren tartás 15 percig
Ébren tartás 30 percig
Ébren tartás 1 óráig
Ébren tartás 2 óráig
Leállítás
────────────────────────
Kijelzőt is ébren tartsa ✓
────────────────────────
Kilépés
```

Állapotok:

- `Inaktív`
- `Aktív`
- `30 perc: 24:13 van hátra`
- `Lejárt`

A menü első sora lehet disabled item, amely csak státuszt mutat.

## 6. Animált ikon koncepció

A tray ikon nem valódi GIF-ként animálódik, hanem több statikus PNG frame cserélgetésével.

A felhasználó által mutatott koncepció:

- üres kávéscsésze;
- kevés kávé;
- félig telt csésze;
- majdnem tele csésze;
- tele csésze;
- opcionális gőz frame-ek.

Javasolt asset lista:

```text
assets/cup_0.png  üres csésze / inaktív / lejárt
assets/cup_1.png  kevés idő van hátra
assets/cup_2.png  közepes idő van hátra
assets/cup_3.png  sok idő van hátra
assets/cup_4.png  teljes idő / frissen indított állapot
```

UX szempontból jobb, ha a csésze a hátralévő időt mutatja:

```text
tele csésze  = sok idő van hátra
üres csésze  = lejárt vagy inaktív
```

Ez érthetőbb, mint az eltelt idő alapú töltés.

## 7. Ikonfrissítési logika

A frame kiválasztása a hátralévő idő arányából történjen.

Példa:

```rust
fn pick_frame(remaining: Duration, total: Duration) -> &'static str {
    if total.as_secs() == 0 {
        return "assets/cup_0.png";
    }

    let ratio = remaining.as_secs_f64() / total.as_secs_f64();

    if ratio > 0.80 {
        "assets/cup_4.png"
    } else if ratio > 0.60 {
        "assets/cup_3.png"
    } else if ratio > 0.40 {
        "assets/cup_2.png"
    } else if ratio > 0.20 {
        "assets/cup_1.png"
    } else {
        "assets/cup_0.png"
    }
}
```

Ikoncsere:

```rust
tray.set_icon(Some(load_icon("assets/cup_3.png")))?;
```

Ne frissüljön az ikon minden loopban feleslegesen. Csak akkor legyen `set_icon(...)`, ha a frame ténylegesen változott.

## 8. Időzítő frissítési stratégia

Első verzióban elég másodpercenként frissíteni.

Event loop stratégia:

```rust
*control_flow = ControlFlow::WaitUntil(
    Instant::now() + Duration::from_secs(1)
);
```

Minden ticknél:

1. menüesemények feldolgozása;
2. timer állapot ellenőrzése;
3. hátralévő idő kiszámítása;
4. ikon frame kiválasztása;
5. tooltip frissítése;
6. lejárat esetén assertion release.

## 9. Tooltip terv

Inaktív állapot:

```text
KeepAwake: inaktív
```

Időzített aktív állapot:

```text
KeepAwake: 24:13 van hátra
```

Manuális aktív állapot:

```text
KeepAwake: aktív
```

Lejárt állapot:

```text
KeepAwake: lejárt
```

## 10. Moduláris kódfelépítés

Javasolt struktúra:

```text
src/
  main.rs
  awake.rs
  tray.rs
  timer.rs
  icons.rs
  app_state.rs
assets/
  cup_0.png
  cup_1.png
  cup_2.png
  cup_3.png
  cup_4.png
```

### 10.1. `awake.rs`

Feladata:

- IOKit FFI;
- system sleep assertion létrehozása;
- display sleep assertion létrehozása;
- assertion release;
- hibakezelés.

Javasolt API:

```rust
pub struct AwakeController {
    system_assertion_id: Option<u32>,
    display_assertion_id: Option<u32>,
}

impl AwakeController {
    pub fn new() -> Self;
    pub fn start(&mut self, keep_display_awake: bool) -> Result<(), AwakeError>;
    pub fn stop(&mut self);
    pub fn is_active(&self) -> bool;
}
```

Elvárás:

- `Drop` implementációval automatikusan release-eljen, ha a processz kilép;
- dupla start ne hozzon létre többszörös assertiont release nélkül;
- stop idempotens legyen, tehát többször hívva se hibázzon.

### 10.2. `timer.rs`

Feladata:

- időzített mód állapotának kezelése;
- hátralévő idő számítása;
- lejárat eldöntése.

Javasolt API:

```rust
pub struct TimerState {
    started_at: Instant,
    total: Duration,
}

impl TimerState {
    pub fn new(total: Duration) -> Self;
    pub fn elapsed(&self) -> Duration;
    pub fn remaining(&self) -> Duration;
    pub fn is_finished(&self) -> bool;
    pub fn progress_ratio(&self) -> f64;
}
```

### 10.3. `icons.rs`

Feladata:

- PNG ikonok betöltése;
- aktuális állapotból frame kiválasztása;
- ikon cache-elése, hogy ne kelljen minden ticknél újra fájlból tölteni.

Javasolt API:

```rust
pub enum IconFrame {
    Empty,
    Low,
    Mid,
    High,
    Full,
}

pub fn frame_for_remaining(remaining: Duration, total: Duration) -> IconFrame;
pub fn path_for_frame(frame: IconFrame) -> &'static str;
```

Későbbi optimalizáció:

- az összes ikon legyen startupkor memóriába töltve;
- `HashMap<IconFrame, Icon>` vagy egyszerű struct mezők használata.

### 10.4. `tray.rs`

Feladata:

- tray ikon létrehozása;
- menü létrehozása;
- menu item ID-k kezelése;
- tooltip és ikon frissítése.

Javasolt felelősség:

- csak UI eseményeket adjon vissza;
- ne tartalmazza közvetlenül az IOKit logikát.

Javasolt belső enum:

```rust
pub enum TrayCommand {
    StartIndefinite,
    StartTimed(Duration),
    Stop,
    ToggleDisplayAwake,
    Quit,
}
```

### 10.5. `app_state.rs`

Feladata:

- alkalmazás állapotának összefogása.

Javasolt modell:

```rust
pub enum AwakeMode {
    Inactive,
    Indefinite,
    Timed(TimerState),
}

pub struct AppState {
    awake_mode: AwakeMode,
    keep_display_awake: bool,
    current_icon_frame: IconFrame,
}
```

## 11. Alap működési folyamat

### 11.1. App indulás

1. `EventLoop::new()` létrejön.
2. Inicializálódik az `AppState`.
3. Inicializálódik az `AwakeController`.
4. Betöltődnek az ikonok.
5. Létrejön a tray ikon `cup_0.png` állapottal.
6. Tooltip: `KeepAwake: inaktív`.
7. Event loop várakozik.

### 11.2. 30 perces mód indítása

1. Felhasználó kiválasztja: `Ébren tartás 30 percig`.
2. `TrayCommand::StartTimed(Duration::from_secs(30 * 60))` keletkezik.
3. `AwakeController.start(keep_display_awake)` lefut.
4. `AppState.awake_mode = AwakeMode::Timed(...)`.
5. Ikon `cup_4.png`.
6. Tooltip hátralévő idővel frissül.

### 11.3. Timer tick

1. Timer remaining kiszámítása.
2. Ha lejárt:
   - `AwakeController.stop()`;
   - `AppState.awake_mode = AwakeMode::Inactive`;
   - ikon `cup_0.png`;
   - tooltip `KeepAwake: lejárt`.
3. Ha még fut:
   - frame kiválasztása;
   - ikon frissítése, ha változott;
   - tooltip frissítése.

### 11.4. Manuális leállítás

1. Felhasználó kiválasztja: `Leállítás`.
2. `AwakeController.stop()`.
3. Timer törlése.
4. Állapot inaktív.
5. Ikon `cup_0.png`.
6. Tooltip `KeepAwake: inaktív`.

### 11.5. Kilépés

1. Felhasználó kiválasztja: `Kilépés`.
2. `AwakeController.stop()`.
3. Event loop `ControlFlow::Exit`.
4. `Drop` biztonsági release.

## 12. IOKit implementációs vázlat

A szükséges FFI:

```rust
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use libc::c_void;

type IOPMAssertionID = u32;
type IOReturn = i32;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: *const c_void,
        assertion_level: u32,
        assertion_name: *const c_void,
        assertion_id: *mut IOPMAssertionID,
    ) -> IOReturn;

    fn IOPMAssertionRelease(assertion_id: IOPMAssertionID) -> IOReturn;
}
```

Assertion létrehozás:

```rust
let assertion_type = CFString::new("PreventUserIdleSystemSleep");
let assertion_name = CFString::new("KeepAwake Rust app");

let mut assertion_id: IOPMAssertionID = 0;

let result = unsafe {
    IOPMAssertionCreateWithName(
        assertion_type.as_concrete_TypeRef() as *const c_void,
        255,
        assertion_name.as_concrete_TypeRef() as *const c_void,
        &mut assertion_id,
    )
};
```

Sikeres létrehozás esetén `assertion_id` eltárolandó.

Release:

```rust
unsafe {
    IOPMAssertionRelease(assertion_id);
}
```

## 13. Tray ikon implementációs vázlat

Ikon betöltése:

```rust
fn load_icon(path: &str) -> Icon {
    let image = image::open(path)
        .expect("Nem sikerült betölteni az ikont")
        .into_rgba8();

    let (width, height) = image.dimensions();

    Icon::from_rgba(image.into_raw(), width, height)
        .expect("Nem sikerült ikon objektumot létrehozni")
}
```

Tray létrehozás:

```rust
let tray = TrayIconBuilder::new()
    .with_menu(Box::new(menu.clone()))
    .with_tooltip("KeepAwake: inaktív")
    .with_icon(load_icon("assets/cup_0.png"))
    .build()
    .expect("Nem sikerült létrehozni a tray ikont");
```

Ikon frissítés:

```rust
tray.set_icon(Some(load_icon("assets/cup_3.png")))?;
```

Tooltip frissítés:

```rust
tray.set_tooltip(Some("KeepAwake: 24:13 van hátra"))?;
```

## 14. CLI opciók későbbi bővítéshez

Későbbi CLI módok:

```bash
keepawake
```

Időkorlát nélküli ébren tartás.

```bash
keepawake --for 30m
```

30 perces ébren tartás.

```bash
keepawake --for 2h
```

2 órás ébren tartás.

```bash
keepawake --cmd "pi agent"
```

A gép addig marad ébren, amíg a megadott command fut.

```bash
keepawake --pid 12345
```

A gép addig marad ébren, amíg a megadott processz él.

Ezekhez később érdemes `clap` crate-et bevezetni.

## 15. Ikon asset követelmények

Javasolt:

- transparent PNG;
- egyszerű vonalrajz;
- kevés részlet;
- nagy kontraszt;
- 16x16 vagy 18x18 logikai méret;
- 32x32 retina változat;
- opcionálisan 64x64 forrásméretből skálázva.

A mutatott barna kávéscsésze koncepció jó, de macOS menüsorban valószínűleg jobb:

- monokróm vagy template jellegű ikon;
- vastagabb kontúr;
- egyszerűbb belső folyadékszintek;
- a gőz opcionális, mert kis méretben zajossá válhat.

## 16. Hibakezelés

Kezelendő hibák:

- IOKit assertion létrehozása sikertelen;
- assertion release hibája;
- ikonfájl hiányzik;
- ikonfájl nem olvasható;
- tray ikon létrehozása sikertelen;
- event loop alatt tray update hiba;
- dupla start / dupla stop.

Javasolt stratégia:

- kritikus startup hibánál kilépés;
- runtime tray update hibánál logolás;
- assertion release hibánál logolás, de kilépés folytatása;
- `stop()` legyen idempotens.

## 17. Biztonsági és stabilitási követelmények

Kritikus pont:

- az app semmilyen esetben ne hagyjon aktív assertiont kilépés után.

Megoldás:

- `AwakeController::stop()` meghívása menüből kilépéskor;
- `Drop` implementáció az `AwakeController`-en;
- Ctrl+C handler CLI módhoz;
- pánik esetére később `std::panic::set_hook` opcionális.

## 18. Tesztelési terv

### 18.1. Assertion teszt

Ellenőrizni kell, hogy aktív állapotban a macOS látja-e az assertiont.

Hasznos parancs:

```bash
pmset -g assertions
```

Elvárt:

- aktív módban látszik a `PreventUserIdleSystemSleep` assertion;
- display-awake bekapcsolva látszik a display assertion is;
- stop után eltűnnek.

### 18.2. Timer teszt

Teszt esetek:

- 30 másodperces fejlesztői timer;
- lejáratkor assertion release;
- lejáratkor ikon `cup_0.png`;
- tooltip `lejárt`;
- manuális stop timer közben.

### 18.3. Ikon teszt

Teszt esetek:

- induláskor üres csésze;
- timer indításkor tele csésze;
- hátralévő idő csökkenésével frame váltás;
- stop után üres csésze;
- új timer indításkor vissza tele csésze.

### 18.4. Menü teszt

Teszt esetek:

- `Ébren tartás bekapcsolása` működik;
- `Ébren tartás 30 percig` működik;
- `Leállítás` működik;
- `Kijelzőt is ébren tartsa` állapotot vált;
- `Kilépés` release-el és kilép.

## 19. Első implementációs milestone-ok

### Milestone 1 – CLI-only awake core

Cél:

- IOKit assertion működjön Rustból;
- Ctrl+C-re release;
- `pmset -g assertions` alatt ellenőrizhető legyen.

Tartalom:

- `awake.rs`;
- minimál `main.rs`;
- `PreventUserIdleSystemSleep`.

### Milestone 2 – Tray ikon és alap menü

Cél:

- app megjelenik a macOS menüsorban;
- van ikon;
- van menü;
- van `Kilépés`.

Tartalom:

- `tao` event loop;
- `tray-icon` integráció;
- `cup_0.png`.

### Milestone 3 – Manuális ébren tartás trayből

Cél:

- menüből bekapcsolható és kikapcsolható a sleep blokkolás.

Tartalom:

- `Ébren tartás bekapcsolása`;
- `Leállítás`;
- tooltip frissítés;
- assertion release kilépéskor.

### Milestone 4 – Időzített mód

Cél:

- 15 perc, 30 perc, 1 óra, 2 óra opciók;
- timer lejáratkor automatikus stop.

Tartalom:

- `timer.rs`;
- `AwakeMode::Timed`;
- tooltip hátralévő idővel.

### Milestone 5 – Animált/progress ikon

Cél:

- ikon frame-ek a hátralévő idő szerint változzanak.

Tartalom:

- `icons.rs`;
- `cup_0.png`–`cup_4.png`;
- frame cache;
- csak változáskor `set_icon(...)`.

### Milestone 6 – Display awake opció

Cél:

- checkbox menüből állítható legyen, hogy a kijelző is ébren maradjon.

Tartalom:

- `PreventUserIdleDisplaySleep`;
- checkbox menüpont;
- aktív session közben változtatás esetén assertion újraindítás vagy display assertion külön kezelése.

### Milestone 7 – Csomagolás macOS appként

Cél:

- `.app` bundle;
- saját ikon;
- indítható Finderből;
- később autostart opció.

Lehetséges eszközök:

- `cargo-bundle`;
- saját `.app` bundle script;
- később notarization, ha publikus terjesztés lesz.

## 20. Későbbi bővítések

Lehetséges további funkciók:

- autostart login után;
- saját idő megadása;
- command/process követés;
- menüből utolsó időzítés újraindítása;
- aktív processzlista figyelés;
- dark/light menüsorhoz optimalizált template ikon;
- app preferences ablak;
- Tauri alapú GUI, ha később komplexebb beállítófelület kell;
- logfájl;
- menüből `pmset assertions` státusz diagnosztika.

## 21. Döntések összefoglalása

Végleges tervezési döntések az eddigi beszélgetés alapján:

- Rust app készül macOS-re.
- A sleep blokkolás IOKit power assertionnel történik.
- A fő assertion: `PreventUserIdleSystemSleep`.
- Opcionális assertion: `PreventUserIdleDisplaySleep`.
- A felső macOS menüsorban tray/status ikon jelenik meg.
- A tray/status UI-hoz `tray-icon` + `tao` használható.
- Az ikon animálása statikus PNG frame-ek cseréjével történik.
- A kávéscsésze ikon a hátralévő időt mutatja.
- Tele csésze = sok idő van hátra.
- Üres csésze = inaktív vagy lejárt.
- Első fontos időzített opció: 30 perc.
- A logika modulárisan legyen szétválasztva: awake, tray, timer, icons, app state.
- Kilépéskor és stop esetén az assertion mindig release-elődjön.

