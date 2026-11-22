use super::*;

#[test]
fn preserves_presented_telemetry_fields() {
    let summary = system_summary(applet::SystemSummary {
        cpu_percent: Some(42.0),
        memory: Some(applet::Usage {
            used_mb: 2,
            total_mb: 4,
        }),
        storage: Some(applet::Usage {
            used_mb: 3,
            total_mb: 5,
        }),
        network_down_bps: Some(123),
        network_up_bps: Some(456),
        fallback: true,
    });
    assert_eq!(summary.cpu_percent, Some(42.0));
    assert_eq!(
        summary.memory.unwrap(),
        Usage {
            used_mb: 2,
            total_mb: 4
        }
    );
    assert_eq!(
        summary.storage.unwrap(),
        Usage {
            used_mb: 3,
            total_mb: 5
        }
    );
    assert_eq!(summary.network_down_bps, Some(123));
    assert_eq!(summary.network_up_bps, Some(456));
    assert!(summary.fallback);
}
