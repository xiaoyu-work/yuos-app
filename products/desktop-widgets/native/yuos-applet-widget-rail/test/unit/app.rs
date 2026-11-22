use super::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[test]
fn suggestions_use_only_reported_memory_and_storage() {
    let mut system = SystemSummary {
        memory: Some(Usage { used_mb: 85, total_mb: 100 }),
        storage: Some(Usage { used_mb: 90, total_mb: 100 }),
        ..Default::default()
    };
    assert_eq!(suggestions(&system), [fl!("suggestion-memory"), fl!("suggestion-storage")]);
    system.memory.as_mut().unwrap().used_mb = 84;
    system.storage.as_mut().unwrap().used_mb = 89;
    assert_eq!(suggestions(&system), [fl!("suggestion-balanced")]);
    system.storage = None;
    assert!(suggestions(&system).is_empty(), "incomplete data does not show a balanced status");
    assert_eq!(
        suggestions(&SystemSummary { memory: Some(Usage { used_mb: 85, total_mb: 100 }), ..Default::default() }),
        [fl!("suggestion-memory")],
    );
}

#[test]
fn formats_network_rates() {
    assert_eq!(format_rate(1536), "2 KB/s");
}

#[test]
fn telemetry_refresh_suppresses_overlapping_loads_and_recovers_after_denial() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&calls);
    let (mut rail, _) = <WidgetRail as cosmic::Application>::init(
        cosmic::app::Core::default(),
        Arc::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(SystemSummary::default()) })
        }),
    );
    let _ = cosmic::Application::update(&mut rail, Message::Refresh);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let _ = cosmic::Application::update(&mut rail, Message::SystemLoaded(Err("denied".into())));
    assert!(matches!(&rail.system, SystemState::Unavailable(error) if error == "denied"));
    let _ = cosmic::Application::update(&mut rail, Message::Refresh);
    let _ = cosmic::Application::update(&mut rail, Message::Refresh);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let _ = cosmic::Application::update(&mut rail, Message::SystemLoaded(Ok(SystemSummary::default())));
    assert!(matches!(rail.system, SystemState::Ready(_)));
}
