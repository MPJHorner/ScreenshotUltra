// Apple Vision OCR. Reads text out of an image file via the system's
// `VNRecognizeTextRequest` API — entirely on-device, no network calls.
//
// This is M5 v0: a single `extract_text(path)` entry point, used by the
// Quick Tray's "Text" button and by `screenshot-ultra --ocr <path>`.

#[cfg(not(target_os = "macos"))]
pub fn extract_text(_path: &std::path::Path) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub use mac::extract_text;

#[cfg(target_os = "macos")]
mod mac {
    use std::path::Path;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, AllocAnyThread, ClassType};
    use objc2_foundation::{NSArray, NSDictionary, NSString, NSURL};
    use objc2_vision::{
        VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation, VNRequest,
        VNRequestTextRecognitionLevel,
    };

    /// Run an OCR pass on `path` and return the recognised text joined by
    /// newlines. Returns None if the image can't be loaded, the request
    /// fails, or no text is found.
    pub fn extract_text(path: &Path) -> Option<String> {
        unsafe {
            let path_str = NSString::from_str(&path.to_string_lossy());
            let url = NSURL::fileURLWithPath(&path_str);

            // Empty options dict — we don't need orientation hints or a
            // custom CIContext for a still PNG/JPG from disk.
            let empty: Retained<NSDictionary<NSString, AnyObject>> =
                msg_send![class_for::<NSDictionary<NSString, AnyObject>>(), dictionary];
            let handler: Retained<VNImageRequestHandler> = msg_send![
                VNImageRequestHandler::alloc(),
                initWithURL: &*url,
                options: &*empty as *const NSDictionary<NSString, AnyObject>,
            ];

            let request: Retained<VNRecognizeTextRequest> =
                msg_send![VNRecognizeTextRequest::alloc(), init];
            request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
            // Apple's docs say usesLanguageCorrection=true is best for
            // freeform text; it's the default but pin it explicitly.
            request.setUsesLanguageCorrection(true);

            // perform requires NSArray<VNRequest>. The recognise-text
            // request is a subclass of VNRequest; wrap and upcast.
            let req_obj: *const VNRequest = (&*request as *const VNRecognizeTextRequest).cast();
            let req_array: Retained<NSArray<VNRequest>> = msg_send![
                class_for::<NSArray<VNRequest>>(),
                arrayWithObject: req_obj,
            ];

            if handler.performRequests_error(&req_array).is_err() {
                return None;
            }

            let observations: Retained<NSArray<VNRecognizedTextObservation>> = request.results()?;
            let mut out = String::new();
            let count = observations.count();
            for i in 0..count {
                let obs = observations.objectAtIndex(i);
                let candidates = obs.topCandidates(1);
                if candidates.count() == 0 {
                    continue;
                }
                let best = candidates.objectAtIndex(0);
                let s = best.string();
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&s.to_string());
            }
            if out.is_empty() {
                None
            } else {
                Some(out)
            }
        }
    }

    /// Helper to get the Objective-C class for a concrete generic type.
    /// objc2's typed `NSDictionary<K, V>` doesn't expose `class()` directly
    /// the way the non-generic types do; we want the raw NSDictionary
    /// class for `+dictionary` / `+arrayWithObject:`.
    fn class_for<T: ClassType>() -> &'static objc2::runtime::AnyClass {
        T::class()
    }
}
