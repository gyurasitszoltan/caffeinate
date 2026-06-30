//! IOKit Power Management assertion FFI.
//!
//! A gép alvását (és opcionálisan a kijelző alvását) `IOPMAssertionCreateWithName`
//! által létrehozott assertion-nel akadályozzuk meg. A megfelelő assertion
//! típusok:
//!   - `PreventUserIdleSystemSleep`  (fő)
//!   - `PreventUserIdleDisplaySleep` (opcionális)
//!
//! A kontroller idempotens: `start` előbb release-eli az esetleg meglévő
//! assertion-t, `stop` többször is hívható. `Drop` implementáció biztonsági
//! release kilépéskor, hogy sosem maradjon aktív assertion a processz után.

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use libc::c_void;
use std::fmt;

/// IOKit assertion azonosító (UInt32).
type IOPMAssertionID = u32;
/// IOKit return code (IOReturn).
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

/// `kIOPMAssertionLevelOn` érték (255). 0 = Off.
const ASSERTION_LEVEL_ON: u32 = 255;

/// Sikeres IOReturn: `kIOReturnSuccess = 0`.
const IORETURN_SUCCESS: IOReturn = 0;

const SYSTEM_TYPE: &str = "PreventUserIdleSystemSleep";
const DISPLAY_TYPE: &str = "PreventUserIdleDisplaySleep";
const ASSERTION_NAME: &str = "KeepAwake";

#[derive(Debug)]
#[allow(dead_code)]
pub enum AwakeError {
    CreateSystem(IOReturn),
    CreateDisplay(IOReturn),
    ReleaseSystem(IOReturn),
    ReleaseDisplay(IOReturn),
}

impl fmt::Display for AwakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AwakeError::CreateSystem(c) => {
                write!(f, "IOKit rendszer-sleep assertion létrehozása sikertelen (IOReturn={})", c)
            }
            AwakeError::CreateDisplay(c) => {
                write!(f, "IOKit kijelző-sleep assertion létrehozása sikertelen (IOReturn={})", c)
            }
            AwakeError::ReleaseSystem(c) => {
                write!(f, "IOKit rendszer-sleep assertion release hiba (IOReturn={})", c)
            }
            AwakeError::ReleaseDisplay(c) => {
                write!(f, "IOKit kijelző-sleep assertion release hiba (IOReturn={})", c)
            }
        }
    }
}

impl std::error::Error for AwakeError {}

/// Létrehoz egy IOKit assertion-t a megadott típusnévvel.
/// Visszaadja az assertion ID-t, vagy a nyers IOReturn hibakódot.
fn create_assertion(type_str: &str) -> Result<IOPMAssertionID, IOReturn> {
    let cf_type = CFString::new(type_str);
    let cf_name = CFString::new(ASSERTION_NAME);
    let mut id: IOPMAssertionID = 0;

    let result = unsafe {
        IOPMAssertionCreateWithName(
            cf_type.as_concrete_TypeRef() as *const c_void,
            ASSERTION_LEVEL_ON,
            cf_name.as_concrete_TypeRef() as *const c_void,
            &mut id,
        )
    };

    if result == IORETURN_SUCCESS {
        Ok(id)
    } else {
        Err(result)
    }
}

/// Release egy assertion-t. Visszaadja az IOReturn-öt (0 = ok).
fn release_assertion(id: IOPMAssertionID) -> IOReturn {
    unsafe { IOPMAssertionRelease(id) }
}

/// A sleep-blokkoló assertion-ek életciklusát vezérli.
#[allow(dead_code)]
pub struct AwakeController {
    system_assertion_id: Option<IOPMAssertionID>,
    display_assertion_id: Option<IOPMAssertionID>,
}

impl AwakeController {
    pub fn new() -> Self {
        Self {
            system_assertion_id: None,
            display_assertion_id: None,
        }
    }

    /// Elindítja a sleep blokkolást. Idempotens: előbb leállítja az esetleg
    /// már futó assertion-t, így dupla start nem hagy többszörös assertion-t.
    /// `keep_display_awake` true esetén külön display-sleep assertion is jön.
    pub fn start(&mut self, keep_display_awake: bool) -> Result<(), AwakeError> {
        self.stop();

        let sys_id = create_assertion(SYSTEM_TYPE).map_err(AwakeError::CreateSystem)?;
        self.system_assertion_id = Some(sys_id);

        if keep_display_awake {
            match create_assertion(DISPLAY_TYPE) {
                Ok(disp_id) => self.display_assertion_id = Some(disp_id),
                Err(code) => {
                    // rollback: release system assertion, majd hiba
                    self.stop();
                    return Err(AwakeError::CreateDisplay(code));
                }
            }
        }
        Ok(())
    }

    /// Leállítja a blokkolást. Idempotens: többször hívva se hibázzon.
    /// Release hiba csak logolásra kerül, nem abortál.
    pub fn stop(&mut self) {
        if let Some(id) = self.system_assertion_id.take() {
            let r = release_assertion(id);
            if r != IORETURN_SUCCESS {
                eprintln!("KeepAwake: rendszer assertion release hiba (IOReturn={})", r);
            }
        }
        if let Some(id) = self.display_assertion_id.take() {
            let r = release_assertion(id);
            if r != IORETURN_SUCCESS {
                eprintln!("KeepAwake: kijelző assertion release hiba (IOReturn={})", r);
            }
        }
    }

    /// Menüből (checkbox) menet közben változtatható: be/ki kapcsolja a
    /// kijelző-sleep assertion-t a futó session alatt, a rendszer
    /// assertion-t nem bántja. Ha épp nem aktív session, csak elmentődik
    /// a flag szándék, és a következő `start` alkalmazza.
    pub fn set_keep_display_awake(&mut self, keep: bool) -> Result<(), AwakeError> {
        let active = self.system_assertion_id.is_some();
        match (self.display_assertion_id.is_some(), keep, active) {
            (false, true, true) => match create_assertion(DISPLAY_TYPE) {
                Ok(disp_id) => {
                    self.display_assertion_id = Some(disp_id);
                    Ok(())
                }
                Err(code) => Err(AwakeError::CreateDisplay(code)),
            },
            (true, false, _) => {
                if let Some(id) = self.display_assertion_id.take() {
                    let r = release_assertion(id);
                    if r != IORETURN_SUCCESS {
                        return Err(AwakeError::ReleaseDisplay(r));
                    }
                }
                Ok(())
            }
            // nincs aktív session: semmi mutex változás most; flag a state-ben él.
            _ => Ok(()),
        }
    }

    /// True, ha épp van élő rendszer-sleep assertion.
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.system_assertion_id.is_some()
    }

    /// True, ha a kijelzõ-sleep assertion épp aktív.
    #[allow(dead_code)]
    pub fn is_display_awake(&self) -> bool {
        self.display_assertion_id.is_some()
    }
}

impl Default for AwakeController {
    fn default() -> Self {
        Self::new()
    }
}

/// Biztonsági release: ha valamiért nem hívódott `stop()`, a Drop is
/// felszabadítja az assertion-t, sosem marad aktív a processz után.
impl Drop for AwakeController {
    fn drop(&mut self) {
        self.stop();
    }
}
