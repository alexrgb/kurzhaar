use kurzhaar_core::detect::{CachedDetector, Detection, Detector, UaparserDetector};
use kurzhaar_core::detect::{DeviceClass, Signals};
use serde::Deserialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct In {
    user_agent: String,
    #[serde(default)]
    brands: Vec<String>,
    mobile: Option<bool>,
    platform: Option<String>,
    platform_major: Option<u32>,
    touch_points: Option<u32>,
    coarse_pointer: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Out {
    browser: String,
    browser_major: Option<u32>,
    os: String,
    os_major: Option<u32>,
    device: DeviceClass,
    bot: bool,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    signals: In,
    expected: Out,
}

fn signals(i: In) -> Signals {
    Signals {
        user_agent: i.user_agent,
        brands: i.brands,
        mobile: i.mobile,
        platform: i.platform,
        platform_major: i.platform_major,
        touch_points: i.touch_points,
        coarse_pointer: i.coarse_pointer,
    }
}

fn cases() -> Vec<Case> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/detect/vectors.json"
    );
    let text = std::fs::read_to_string(path).expect("vectors");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
    serde_json::from_value(v["vectors"].clone()).expect("cases")
}

// verifies: REQ-COLLECT-X6WF.S1 (real User-Agents and client signals give the expected browser, OS, device class and bot flag)
// verifies: REQ-COLLECT-QZRJ.S1 (names and major versions only, for every vector)
#[test]
fn vectors_give_the_expected_detection() {
    let d = UaparserDetector::builtin().expect("rules");
    let mut failures = Vec::new();
    for c in cases() {
        let got = d.detect(&signals(c.signals));
        let want = Detection {
            browser: c.expected.browser,
            browser_major: c.expected.browser_major,
            os: c.expected.os,
            os_major: c.expected.os_major,
            device: c.expected.device,
            bot: c.expected.bot,
        };
        if got != want {
            failures.push(format!("{}: got {got:?}, want {want:?}", c.name));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

struct Counting(Arc<AtomicUsize>);

impl Detector for Counting {
    fn detect(&self, _: &Signals) -> Detection {
        self.0.fetch_add(1, Ordering::SeqCst);
        Detection {
            browser: "B".into(),
            browser_major: Some(1),
            os: "O".into(),
            os_major: None,
            device: DeviceClass::Desktop,
            bot: false,
        }
    }
}

// verifies: REQ-COLLECT-4THR.S1 (a thousand events with one tuple call the detector once)
#[test]
fn a_seen_tuple_never_reaches_the_detector_again() {
    let calls = Arc::new(AtomicUsize::new(0));
    let d = CachedDetector::new(Counting(calls.clone()), 100, Duration::from_secs(3600));
    let s = Signals {
        user_agent: "same".into(),
        ..Signals::default()
    };
    for _ in 0..1000 {
        d.detect(&s);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    d.detect(&Signals {
        user_agent: "other".into(),
        ..Signals::default()
    });
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn the_cache_is_bounded_and_expires() {
    let calls = Arc::new(AtomicUsize::new(0));
    let d = CachedDetector::new(Counting(calls.clone()), 3, Duration::from_millis(0));
    let s = Signals {
        user_agent: "x".into(),
        ..Signals::default()
    };
    d.detect(&s);
    d.detect(&s);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "expired entries are asked again"
    );
    let d = CachedDetector::new(Counting(calls), 3, Duration::from_secs(60));
    for i in 0..10 {
        d.detect(&Signals {
            user_agent: format!("ua{i}"),
            ..Signals::default()
        });
    }
    assert!(d.len() <= 3);
}
