//! macOS Calendar through EventKit.
//!
//! `EKEventStore` is costly to create and isn't thread-safe, so one worker thread
//! owns it and everything else sends it jobs.

use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{Bool, NSObjectProtocol};
use objc2::sel;
use objc2_app_kit::{NSColor, NSColorSpace};
use objc2_event_kit::{
    EKAuthorizationStatus, EKCalendar, EKCalendarType, EKEntityType, EKEventStore,
    EKEventStoreChangedNotification,
};
use objc2_foundation::{NSArray, NSDate, NSError, NSNotification, NSNotificationCenter, NSOperationQueue};
use ruz_core::events::{AccessStatus, EventSource};
use tauri::{AppHandle, Emitter};

use super::{RawCalendar, RawEvent};

type Job = Box<dyn FnOnce(&mut Retained<EKEventStore>) + Send>;

fn worker() -> &'static Sender<Job> {
    static WORKER: OnceLock<Sender<Job>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("ruz-eventkit".into())
            .spawn(move || {
                let mut store = unsafe { EKEventStore::new() };
                for job in rx {
                    autoreleasepool(|_| job(&mut store));
                }
            })
            .expect("could not start the calendar thread");
        tx
    })
}

/// Runs `f` with the store on the worker thread and waits for its result.
fn on_store<T: Send + 'static>(
    f: impl FnOnce(&mut Retained<EKEventStore>) -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = mpsc::channel();
    worker()
        .send(Box::new(move |store| {
            let _ = tx.send(f(store));
        }))
        .map_err(|_| "the calendar thread stopped".to_string())?;
    rx.recv_timeout(Duration::from_secs(15)).map_err(|_| "the calendar didn't answer".to_string())
}

pub fn access() -> AccessStatus {
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    match status {
        EKAuthorizationStatus::NotDetermined => AccessStatus::NotDetermined,
        EKAuthorizationStatus::Restricted => AccessStatus::Restricted,
        EKAuthorizationStatus::FullAccess => AccessStatus::Authorized,
        // Write-only access can't read events, so it's as good as denied here.
        _ => AccessStatus::Denied,
    }
}

/// Shows the system prompt if the user hasn't answered yet, and waits for the answer.
pub fn request_access() -> AccessStatus {
    if access() != AccessStatus::NotDetermined {
        return access();
    }
    let (tx, rx) = mpsc::channel::<bool>();
    let asked = on_store(move |store| {
        let completion = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            let _ = tx.send(granted.as_bool());
        });
        // macOS 14 split calendar access into full and write-only; older systems
        // only have the general request.
        if store.respondsToSelector(sel!(requestFullAccessToEventsWithCompletion:)) {
            unsafe { store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&completion)) };
        } else {
            #[allow(deprecated)]
            unsafe {
                store.requestAccessToEntityType_completion(EKEntityType::Event, RcBlock::as_ptr(&completion))
            };
        }
    });
    if asked.is_err() {
        return access();
    }
    // The user may take a while to answer the prompt.
    if rx.recv_timeout(Duration::from_secs(600)).unwrap_or(false) {
        // A store created before access was granted doesn't see the calendars.
        let _ = on_store(|store| *store = unsafe { EKEventStore::new() });
    }
    access()
}

fn hex(color: &NSColor) -> String {
    color
        .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
        .map(|c| {
            let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!(
                "#{:02x}{:02x}{:02x}",
                byte(c.redComponent()),
                byte(c.greenComponent()),
                byte(c.blueComponent())
            )
        })
        .unwrap_or_else(|| "#8e8e93".into())
}

fn is_birthdays(calendar: &EKCalendar) -> bool {
    let kind = unsafe { calendar.r#type() };
    kind == EKCalendarType::Birthday
}

pub fn calendars() -> Result<Vec<RawCalendar>, String> {
    if access() != AccessStatus::Authorized {
        return Ok(Vec::new());
    }
    on_store(|store| {
        let calendars = unsafe { store.calendarsForEntityType(EKEntityType::Event) };
        calendars
            .iter()
            .map(|c| unsafe {
                RawCalendar {
                    id: c.calendarIdentifier().to_string(),
                    title: c.title().to_string(),
                    source: c.source().map(|s| s.title().to_string()).unwrap_or_default(),
                    color: hex(&c.color()),
                    birthdays: is_birthdays(&c),
                }
            })
            .collect()
    })
}

/// Events overlapping `[start, end)` (Unix seconds) in calendars not in `hidden`.
pub fn events(start: f64, end: f64, hidden: Vec<String>) -> Result<Vec<RawEvent>, String> {
    if access() != AccessStatus::Authorized {
        return Ok(Vec::new());
    }
    on_store(move |store| unsafe {
        let visible: Vec<Retained<EKCalendar>> = store
            .calendarsForEntityType(EKEntityType::Event)
            .iter()
            .filter(|c| !hidden.contains(&c.calendarIdentifier().to_string()))
            .collect();
        if visible.is_empty() {
            return Vec::new();
        }
        let predicate = store.predicateForEventsWithStartDate_endDate_calendars(
            &NSDate::dateWithTimeIntervalSince1970(start),
            &NSDate::dateWithTimeIntervalSince1970(end),
            Some(&NSArray::from_retained_slice(&visible)),
        );
        store
            .eventsMatchingPredicate(&predicate)
            .iter()
            .map(|e| {
                let calendar = e.calendar();
                let start = e.startDate().timeIntervalSince1970();
                RawEvent {
                    id: format!(
                        "{}@{start}",
                        e.eventIdentifier()
                            .map_or_else(|| e.calendarItemIdentifier().to_string(), |id| id.to_string())
                    ),
                    calendar_id: calendar
                        .as_ref()
                        .map(|c| c.calendarIdentifier().to_string())
                        .unwrap_or_default(),
                    title: e.title().to_string(),
                    color: calendar.as_ref().map_or_else(|| "#8e8e93".into(), |c| hex(&c.color())),
                    all_day: e.isAllDay(),
                    birthday: calendar.as_deref().is_some_and(is_birthdays),
                    start,
                    end: e.endDate().timeIntervalSince1970(),
                    location: e.location().map(|l| l.to_string()).filter(|l| !l.trim().is_empty()),
                    source: EventSource::System,
                    url: None,
                }
            })
            .collect()
    })
}

/// Emits `calendar-store-changed` whenever the user's calendars or events change.
pub fn watch(app: AppHandle) {
    let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        let _ = app.emit("calendar-store-changed", ());
    });
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(EKEventStoreChangedNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    // Watch for as long as the app runs.
    std::mem::forget(observer);
}
