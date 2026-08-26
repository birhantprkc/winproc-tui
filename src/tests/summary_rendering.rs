use crate::model::SystemCounterSample;
use crate::samplers::memory::map_memory_counters;
use crate::ui::{
    SummaryInfoStyle, THEMES, optional_value_color, render_summary_info_line,
    render_summary_info_value_spans, render_summary_line,
};

#[test]
fn map_memory_counters_uses_real_commit_values() {
    let mapped = map_memory_counters(
        32_000,
        12_000,
        Ok(Some(SystemCounterSample {
            available_memory: 10_000,
            committed_memory: 9_000,
            commit_limit: 24_000,
            cache_bytes: Some(1_000),
            modified_page_list_bytes: Some(750),
            standby_cache_bytes: Some(2_000),
            free_zeroed_bytes: Some(500),
            pages_input_per_sec: Some(25),
            pages_output_per_sec: Some(15),
            disk_read_bytes_per_sec: Some(3_000),
            disk_write_bytes_per_sec: Some(4_000),
            disk_queue_length: Some(1.5),
            network_received_bytes_per_sec: Some(5_000),
            network_sent_bytes_per_sec: Some(6_000),
            cpu_frequencies_mhz: Vec::new(),
            cpu_total_usage_percent: None,
            cpu_user_usage_percent: None,
            cpu_kernel_usage_percent: None,
        })),
    );

    assert_eq!(mapped.available_memory, 10_000);
    assert_eq!(mapped.committed_memory, Some(9_000));
    assert_eq!(mapped.commit_limit, Some(24_000));
    assert_eq!(mapped.cache_bytes, Some(1_000));
    assert_eq!(mapped.modified_page_list_bytes, Some(750));
    assert_eq!(mapped.standby_cache_bytes, Some(2_000));
    assert_eq!(mapped.disk_read_bytes_per_sec, Some(3_000));
    assert_eq!(mapped.disk_write_bytes_per_sec, Some(4_000));
    assert_eq!(mapped.disk_queue_length, Some(1.5));
    assert_eq!(mapped.network_received_bytes_per_sec, Some(5_000));
    assert_eq!(mapped.network_sent_bytes_per_sec, Some(6_000));
    assert_eq!(mapped.warning, None);
}

#[test]
fn map_memory_counters_drops_commit_fields_on_failure() {
    let mapped = map_memory_counters(32_000, 12_000, Err(anyhow::anyhow!("pdh failed")));

    assert_eq!(mapped.available_memory, 12_000);
    assert_eq!(mapped.committed_memory, None);
    assert_eq!(mapped.commit_limit, None);
    assert_eq!(mapped.cache_bytes, None);
    assert_eq!(mapped.modified_page_list_bytes, None);
    assert_eq!(mapped.standby_cache_bytes, None);
    assert_eq!(mapped.disk_read_bytes_per_sec, None);
    assert_eq!(mapped.disk_write_bytes_per_sec, None);
    assert_eq!(mapped.disk_queue_length, None);
    assert_eq!(mapped.network_received_bytes_per_sec, None);
    assert_eq!(mapped.network_sent_bytes_per_sec, None);
    assert!(
        mapped
            .warning
            .unwrap()
            .contains("commit counters unavailable")
    );
}

#[test]
fn optional_value_color_uses_presence_not_magnitude() {
    assert_eq!(optional_value_color(Some(0), THEMES[0]), THEMES[0].text);
    assert_eq!(optional_value_color(Some(999), THEMES[0]), THEMES[0].text);
    assert_eq!(optional_value_color(None, THEMES[0]), THEMES[0].muted);
}

#[test]
fn render_summary_info_value_spans_separates_numbers_from_units() {
    let spans = render_summary_info_value_spans("2.11 GHz / 930.43 GiB (97%)", THEMES[0]);
    let rendered = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec!["2.11", " GHz / ", "930.43", " GiB (", "97", "%)"]
    );
    assert_eq!(spans[0].style.fg, Some(THEMES[0].text));
    assert_eq!(spans[1].style.fg, Some(THEMES[0].muted));
}

#[test]
fn render_summary_info_value_spans_keeps_comma_numbers_together() {
    let spans = render_summary_info_value_spans("C: 861/999 GB, X: 400/2,000 GB", THEMES[0]);
    let rendered = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec![
            "C: ", "861", "/", "999", " GB, X: ", "400", "/", "2,000", " GB"
        ]
    );
    assert_eq!(spans[7].style.fg, Some(THEMES[0].text));
}

#[test]
fn render_summary_info_value_spans_keeps_cache_labels_as_text() {
    let spans =
        render_summary_info_value_spans("L1 1.00 MiB  L2 12.00 MiB  L3 25.00 MiB", THEMES[0]);
    let rendered = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec![
            "L1 ",
            "1.00",
            " MiB  L2 ",
            "12.00",
            " MiB  L3 ",
            "25.00",
            " MiB"
        ]
    );
    assert_eq!(spans[0].style.fg, Some(THEMES[0].muted));
    assert_eq!(spans[1].style.fg, Some(THEMES[0].text));
}

#[test]
fn render_summary_line_formats_percent_in_parentheses() {
    let line = render_summary_line(
        "Physical Memory",
        Some(12_345_600_000),
        Some(24_691_200_000),
        None,
        THEMES[0],
    );
    let rendered = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>();
    let joined = rendered.join("");

    assert!(joined.contains("12,346 MB / 24,691 MB"));
    assert!(joined.contains("( 50%)"));
    assert_eq!(line.spans[0].style.fg, Some(THEMES[0].muted));
}

#[test]
fn render_summary_info_line_keeps_identity_values_plain() {
    let line = render_summary_info_line(
        "GPU",
        "NVIDIA GeForce RTX 3070 Ti",
        SummaryInfoStyle::Plain,
        THEMES[0],
    );
    let rendered = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>();

    assert_eq!(rendered, vec!["GPU     ", "NVIDIA GeForce RTX 3070 Ti"]);
    assert_eq!(line.spans[0].style.fg, Some(THEMES[0].muted));
    assert_eq!(line.spans[1].style.fg, Some(THEMES[0].text));
}
