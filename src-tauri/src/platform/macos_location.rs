//! "Use my location" through CoreLocation. The manager and its delegate live on the
//! main thread, where CoreLocation delivers its callbacks.

use std::cell::{Cell, RefCell};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_location::{
    CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
    kCLLocationAccuracyKilometer,
};
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};
use tauri::AppHandle;

type Reply = Sender<Result<(f64, f64), String>>;

pub struct DelegateState {
    reply: RefCell<Option<Reply>>,
    requested: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RuzLocationDelegate"]
    #[ivars = DelegateState]
    struct LocationDelegate;

    unsafe impl NSObjectProtocol for LocationDelegate {}

    unsafe impl CLLocationManagerDelegate for LocationDelegate {
        #[unsafe(method(locationManager:didUpdateLocations:))]
        fn did_update_locations(&self, _manager: &CLLocationManager, locations: &NSArray<CLLocation>) {
            if let Some(location) = locations.lastObject() {
                let c = unsafe { location.coordinate() };
                self.finish(Ok((c.latitude, c.longitude)));
            }
        }

        #[unsafe(method(locationManager:didFailWithError:))]
        fn did_fail(&self, _manager: &CLLocationManager, error: &NSError) {
            self.finish(Err(error.localizedDescription().to_string()));
        }

        #[unsafe(method(locationManagerDidChangeAuthorization:))]
        fn did_change_authorization(&self, manager: &CLLocationManager) {
            self.follow_authorization(manager);
        }
    }
);

impl LocationDelegate {
    fn new(mtm: MainThreadMarker, reply: Reply) -> Retained<Self> {
        let this = Self::alloc(mtm)
            .set_ivars(DelegateState { reply: RefCell::new(Some(reply)), requested: Cell::new(false) });
        unsafe { msg_send![super(this), init] }
    }

    fn finish(&self, result: Result<(f64, f64), String>) {
        if let Some(reply) = self.ivars().reply.borrow_mut().take() {
            let _ = reply.send(result);
        }
    }

    /// Asks for one location once allowed; reports a refusal.
    fn follow_authorization(&self, manager: &CLLocationManager) {
        let status = unsafe { manager.authorizationStatus() };
        if status == CLAuthorizationStatus::AuthorizedAlways
            || status == CLAuthorizationStatus::AuthorizedWhenInUse
        {
            if !self.ivars().requested.replace(true) {
                unsafe { manager.requestLocation() };
            }
        } else if status == CLAuthorizationStatus::Denied || status == CLAuthorizationStatus::Restricted {
            self.finish(Err("denied".into()));
        }
        // NotDetermined: the system prompt is showing; wait for the next callback.
    }
}

thread_local! {
    /// Keeps the latest request's manager and delegate alive until it's answered.
    static ACTIVE: RefCell<Option<(Retained<CLLocationManager>, Retained<LocationDelegate>)>> = const { RefCell::new(None) };
}

/// The device's location as (latitude, longitude). Blocks, so call it off the main thread.
pub fn locate(app: &AppHandle) -> Result<(f64, f64), String> {
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let mtm = MainThreadMarker::new().expect("run_on_main_thread runs on the main thread");
        let manager = unsafe { CLLocationManager::new() };
        let delegate = LocationDelegate::new(mtm, tx);
        unsafe {
            manager.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            manager.setDesiredAccuracy(kCLLocationAccuracyKilometer);
            if manager.authorizationStatus() == CLAuthorizationStatus::NotDetermined {
                manager.requestWhenInUseAuthorization();
            } else {
                delegate.follow_authorization(&manager);
            }
        }
        ACTIVE.with(|active| *active.borrow_mut() = Some((manager, delegate)));
    })
    .map_err(|e| e.to_string())?;
    // Long enough for the user to answer the permission prompt.
    rx.recv_timeout(Duration::from_secs(120)).map_err(|_| "timed out".to_string())?
}
