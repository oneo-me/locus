use async_channel::Sender;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, extern_protocol, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};

use crate::UpdateEvent;

extern_protocol!(
    /// Sparkle's dynamically loaded updater delegate protocol.
    ///
    /// # Safety
    /// Implementations must match SPUUpdaterDelegate.h and run on the main thread.
    /// The Sparkle framework must be loaded before registering an implementing class.
    #[allow(clippy::missing_safety_doc)] // extern_protocol! does not retain the safety docs for Clippy.
    unsafe trait SPUUpdaterDelegate: NSObjectProtocol {}
);

struct DelegateIvars {
    events: Sender<UpdateEvent>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. All callbacks and ivars
    // are used on the main thread; signatures match SPUUpdaterDelegate.h.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "LocusSparkleDelegate"]
    #[ivars = DelegateIvars]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}
    unsafe impl SPUUpdaterDelegate for Delegate {}

    impl Delegate {
        #[unsafe(method(updater:mayPerformUpdateCheck:error:))]
        fn may_check(&self, _updater: &AnyObject, _check: isize, _error: *mut *mut NSError) -> Bool {
            // Observe automatic checks too, without changing Sparkle's scheduling policy.
            let _ = self.ivars().events.try_send(UpdateEvent::Started);
            Bool::YES
        }

        #[unsafe(method(updater:didFindValidUpdate:))]
        fn found(&self, _updater: &AnyObject, item: &AnyObject) {
            // SAFETY: Sparkle passes an SUAppcastItem with this NSString getter.
            let version: Retained<NSString> = unsafe { msg_send![item, displayVersionString] };
            let _ = self.ivars().events.try_send(UpdateEvent::Found(version.to_string()));
        }

        #[unsafe(method(updaterDidNotFindUpdate:error:))]
        fn no_update(&self, _updater: &AnyObject, error: &NSError) {
            // The recovery text distinguishes an up-to-date app from an update
            // that requires a newer macOS; the generic error title may not.
            let reason = error.localizedRecoverySuggestion()
                .or_else(|| error.localizedFailureReason())
                .unwrap_or_else(|| error.localizedDescription());
            let _ = self.ivars().events.try_send(UpdateEvent::NoUpdate(reason.to_string()));
        }

        #[unsafe(method(updater:didFinishUpdateCycleForUpdateCheck:error:))]
        fn finished(&self, _updater: &AnyObject, _check: isize, error: Option<&NSError>) {
            // Sparkle's SUErrors.h: no update (1001) and user cancellation (4007)
            // are normal completion; the no-update callback retains its reason.
            let error = error.filter(|error| !matches!(error.code(), 1001 | 4007))
                .map(|error| error.localizedDescription().to_string());
            let _ = self.ivars().events.try_send(UpdateEvent::Finished(error));
        }
    }
);

pub(super) struct Backend {
    controller: Retained<AnyObject>,
    // Sparkle's updater delegate is weak; retain it alongside the controller.
    _delegate: Retained<Delegate>,
}

impl Backend {
    pub(super) fn new(events: Sender<UpdateEvent>) -> Result<Option<Self>, String> {
        let Some(main_thread) = MainThreadMarker::new() else {
            return Ok(None);
        };
        let Some(directory) = NSBundle::mainBundle().privateFrameworksPath() else {
            return Ok(None);
        };
        let path = NSString::from_str(&format!("{directory}/Sparkle.framework"));
        let Some(bundle) = NSBundle::bundleWithPath(&path) else {
            return Ok(None);
        };
        // SAFETY: The release pipeline embeds the pinned framework. All loading
        // and Objective-C calls stay on the AppKit main thread.
        unsafe { bundle.loadAndReturnError() }
            .map_err(|error| error.localizedDescription().to_string())?;
        let class = AnyClass::get(c"SPUStandardUpdaterController")
            .ok_or_else(|| "Sparkle has no standard updater controller".to_owned())?;
        let allocated = Delegate::alloc(main_thread).set_ivars(DelegateIvars { events });
        // SAFETY: Standard NSObject initialization and Sparkle header signatures.
        let delegate: Retained<Delegate> = unsafe { msg_send![super(allocated), init] };
        let controller: Retained<AnyObject> = unsafe {
            let allocated: objc2::rc::Allocated<AnyObject> = msg_send![class, alloc];
            msg_send![allocated, initWithStartingUpdater: false,
                updaterDelegate: &*delegate,
                userDriverDelegate: std::ptr::null::<AnyObject>()]
        };
        let backend = Self {
            controller,
            _delegate: delegate,
        };
        let updater = backend.updater();
        let mut error: *mut NSError = std::ptr::null_mut();
        // SAFETY: startUpdater: returns BOOL and writes an autoreleased NSError
        // only on failure; it is read before leaving this autorelease scope.
        let started: bool = unsafe { msg_send![&*updater, startUpdater: &mut error] };
        if !started {
            let reason = unsafe { error.as_ref() }
                .map(|error| error.localizedDescription().to_string())
                .unwrap_or_else(|| "Sparkle could not start the updater".to_owned());
            return Err(reason);
        }
        Ok(Some(backend))
    }

    fn updater(&self) -> Retained<AnyObject> {
        // SAFETY: SPUStandardUpdaterController exposes its retained SPUUpdater.
        unsafe { msg_send![&*self.controller, updater] }
    }

    pub(super) fn is_check_enabled(&self) -> bool {
        // SAFETY: SPUUpdater's canCheckForUpdates is a main-thread BOOL getter.
        unsafe { msg_send![&*self.updater(), canCheckForUpdates] }
    }

    pub(super) fn check(&self) {
        // SAFETY: Header-declared action with a nullable sender. Native UI owns
        // cancellation, downloading, installation and restart.
        unsafe {
            let _: () =
                msg_send![&*self.controller, checkForUpdates: std::ptr::null::<AnyObject>()];
        }
    }
}
