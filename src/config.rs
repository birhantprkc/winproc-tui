use std::{
    collections::BTreeMap,
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::app::{App, GraphSlotLayout, ProcessPanelHeight, ProcessViewMode};
use crate::model::{
    ColumnPreset, MetricColumn, ProcessColumnWidths, SortColumn, SortDirection, SortSpec,
};
use crate::samplers::SamplingOptions;

const CONFIG_FILE_NAME: &str = "winproc-tui.toml";
pub(crate) const EMPTY_TRACKED_LIST_NAME: &str = "Empty (default)";

pub(crate) fn is_empty_tracked_list_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case(EMPTY_TRACKED_LIST_NAME)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub(crate) general: GeneralConfig,
    #[serde(skip_serializing)]
    pub(crate) graphs: GraphConfig,
    pub(crate) process_table: ProcessTableConfig,
    pub(crate) recording: RecordingConfig,
    #[serde(skip_serializing)]
    pub(crate) tracking: TrackingConfig,
    #[serde(alias = "watch", alias = "process", skip_serializing)]
    pub(crate) tracked: Vec<TrackedConfig>,
    #[serde(skip_serializing)]
    pub(crate) tracked_lists: Vec<SavedTrackedList>,
    pub(crate) investigation: Option<InvestigationConfig>,
    pub(crate) investigation_profiles: Vec<SavedInvestigationProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct GeneralConfig {
    pub(crate) mouse: bool,
    pub(crate) theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            mouse: true,
            theme: "Green".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct GraphConfig {
    pub(crate) columns: u8,
    pub(crate) samples: bool,
    pub(crate) delta: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            columns: 0,
            samples: true,
            delta: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProcessTableConfig {
    #[serde(skip_serializing)]
    pub(crate) view: String,
    #[serde(skip_serializing)]
    pub(crate) preset: String,
    #[serde(skip_serializing)]
    pub(crate) columns: Vec<String>,
    #[serde(skip_serializing)]
    pub(crate) sort_by: String,
    #[serde(skip_serializing)]
    pub(crate) sort_order: String,
    #[serde(skip_serializing)]
    pub(crate) tracked_only: bool,
    pub(crate) body_rows: ProcessPanelHeightConfig,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) column_widths: BTreeMap<String, i64>,
}

impl Default for ProcessTableConfig {
    fn default() -> Self {
        Self {
            view: ProcessViewMode::Flat.label().to_string(),
            preset: ColumnPreset::Default.label().to_string(),
            columns: ColumnPreset::Default
                .columns()
                .iter()
                .map(|column| column.label().to_string())
                .collect(),
            sort_by: MetricColumn::WorksetPrivateBytes.label().to_string(),
            sort_order: SortDirection::Desc.label().to_string(),
            tracked_only: false,
            body_rows: ProcessPanelHeightConfig::default(),
            column_widths: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum ProcessPanelHeightConfig {
    Rows(i64),
    Mode(String),
}

impl Default for ProcessPanelHeightConfig {
    fn default() -> Self {
        Self::Mode("auto".to_string())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RecordingConfig {
    pub(crate) last_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TrackingConfig {
    pub(crate) startup: TrackedListStartup,
    pub(crate) active_list: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrackedListStartup {
    #[default]
    ResumeLast,
    ChooseList,
    StartEmpty,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvestigationStartup {
    #[default]
    ResumeLast,
    ChooseProfile,
    StartEmpty,
}

impl InvestigationStartup {
    pub(crate) const ALL: [Self; 3] = [Self::ResumeLast, Self::ChooseProfile, Self::StartEmpty];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ResumeLast => "Resume last",
            Self::ChooseProfile => "Choose Profile",
            Self::StartEmpty => "Start empty",
        }
    }

    pub(crate) const fn next(self) -> Self {
        match self {
            Self::ResumeLast => Self::ChooseProfile,
            Self::ChooseProfile => Self::StartEmpty,
            Self::StartEmpty => Self::ResumeLast,
        }
    }

    pub(crate) const fn previous(self) -> Self {
        match self {
            Self::ResumeLast => Self::StartEmpty,
            Self::ChooseProfile => Self::ResumeLast,
            Self::StartEmpty => Self::ChooseProfile,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrackedConfig {
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedTrackedList {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) processes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct InvestigationStateConfig {
    pub(crate) tracked_names: Vec<String>,
    pub(crate) tracked_only: bool,
    pub(crate) process_view: String,
    pub(crate) process_columns: Vec<String>,
    pub(crate) sort_by: String,
    pub(crate) sort_order: String,
    pub(crate) graphs: Vec<InvestigationGraphConfig>,
    pub(crate) graph_columns: u8,
    pub(crate) graph_time_span_seconds: u32,
    pub(crate) samples: bool,
    pub(crate) delta: bool,
    pub(crate) y_axis_zero_min: bool,
    pub(crate) recording_interval_seconds: u64,
}

impl Default for InvestigationStateConfig {
    fn default() -> Self {
        Self {
            tracked_names: Vec::new(),
            tracked_only: false,
            process_view: ProcessViewMode::Flat.label().to_string(),
            process_columns: ColumnPreset::Default
                .effective_columns()
                .iter()
                .map(|column| column.label().to_string())
                .collect(),
            sort_by: MetricColumn::WorksetPrivateBytes.label().to_string(),
            sort_order: SortDirection::Desc.label().to_string(),
            graphs: Vec::new(),
            graph_columns: 0,
            graph_time_span_seconds: 60,
            samples: true,
            delta: true,
            y_axis_zero_min: true,
            recording_interval_seconds: 1,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct InvestigationConfig {
    pub(crate) startup: InvestigationStartup,
    pub(crate) active_profile: Option<String>,
    #[serde(flatten)]
    pub(crate) last: InvestigationStateConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SavedInvestigationProfile {
    pub(crate) name: String,
    #[serde(flatten)]
    pub(crate) investigation: InvestigationStateConfig,
}

impl Deref for SavedInvestigationProfile {
    type Target = InvestigationStateConfig;

    fn deref(&self) -> &Self::Target {
        &self.investigation
    }
}

impl DerefMut for SavedInvestigationProfile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.investigation
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct InvestigationGraphConfig {
    pub(crate) kind: String,
    pub(crate) metric: String,
    pub(crate) display_mode: String,
    pub(crate) process_name: Option<String>,
    pub(crate) executable_path: Option<String>,
    pub(crate) gpu_adapter_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    pub(crate) mouse: bool,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) recording_last_dir: Option<PathBuf>,
    pub(crate) initial_theme: String,
    pub(crate) initial_graph_slot_layout: GraphSlotLayout,
    pub(crate) initial_graph_templates: Vec<InvestigationGraphConfig>,
    pub(crate) initial_graph_time_span_seconds: u32,
    pub(crate) initial_graph_y_axis_zero_min: bool,
    pub(crate) initial_show_samples_panel: bool,
    pub(crate) initial_show_sample_delta: bool,
    pub(crate) initial_recording_interval_seconds: u64,
    pub(crate) column_preset: ColumnPreset,
    pub(crate) process_columns: Vec<MetricColumn>,
    pub(crate) process_column_widths: ProcessColumnWidths,
    pub(crate) sort: SortSpec,
    pub(crate) initial_tracked_only: bool,
    pub(crate) initial_process_view_mode: ProcessViewMode,
    pub(crate) initial_process_panel_height: ProcessPanelHeight,
    pub(crate) process_filters: Vec<String>,
    pub(crate) investigation_startup: InvestigationStartup,
    pub(crate) active_investigation_profile: Option<String>,
    pub(crate) saved_investigation_profiles: Vec<SavedInvestigationProfile>,
    pub(crate) sampling_options: SamplingOptions,
}

pub(crate) fn resolve_config_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("failed to resolve executable path")?;
    let exe_dir = exe
        .parent()
        .context("failed to resolve executable directory")?;
    Ok(exe_dir.join(CONFIG_FILE_NAME))
}

pub(crate) fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    match toml::from_str::<AppConfig>(&raw) {
        Ok(config) => Ok(config),
        Err(error) => {
            eprintln!(
                "Config parse failed for {}: {error}. Falling back to defaults.",
                path.display()
            );
            Ok(AppConfig::default())
        }
    }
}

pub(crate) fn prepare_app_config(config: &mut AppConfig) {
    let legacy_state = legacy_investigation_state(config);
    let legacy_startup = match config.tracking.startup {
        TrackedListStartup::ResumeLast => InvestigationStartup::ResumeLast,
        TrackedListStartup::ChooseList => InvestigationStartup::ChooseProfile,
        TrackedListStartup::StartEmpty => InvestigationStartup::StartEmpty,
    };
    let legacy_active_list = config.tracking.active_list.clone();

    let mut profiles =
        normalize_saved_investigation_profiles(std::mem::take(&mut config.investigation_profiles));
    let mut migrated_names = Vec::<(String, String)>::new();
    for list in normalize_saved_tracked_lists(std::mem::take(&mut config.tracked_lists)) {
        let migrated_name = unique_migrated_profile_name(&list.name, &profiles);
        let mut investigation = legacy_state.clone();
        investigation.tracked_names = list.processes;
        profiles.push(SavedInvestigationProfile {
            name: migrated_name.clone(),
            investigation: normalize_investigation_state(investigation),
        });
        migrated_names.push((list.name, migrated_name));
    }

    let mut investigation = config.investigation.take().unwrap_or_else(|| {
        let active_profile = legacy_active_list.as_deref().and_then(|active| {
            migrated_names
                .iter()
                .find(|(legacy, _)| legacy.eq_ignore_ascii_case(active))
                .map(|(_, migrated)| migrated.clone())
        });
        InvestigationConfig {
            startup: legacy_startup,
            active_profile,
            last: if legacy_startup == InvestigationStartup::StartEmpty {
                InvestigationStateConfig::default()
            } else {
                legacy_state
            },
        }
    });
    investigation.last = normalize_investigation_state(investigation.last);
    investigation.active_profile = investigation
        .active_profile
        .map(|name| name.trim().to_string())
        .filter(|name| {
            profiles
                .iter()
                .any(|profile| profile.name.eq_ignore_ascii_case(name))
        });
    config.investigation = Some(investigation);
    config.investigation_profiles = profiles;
}

pub(crate) fn build_runtime_config(mut config: AppConfig) -> Result<RuntimeConfig> {
    prepare_app_config(&mut config);
    let mut investigation = config
        .investigation
        .take()
        .expect("prepared config must contain an investigation");
    let state = if investigation.startup == InvestigationStartup::StartEmpty {
        investigation.active_profile = None;
        InvestigationStateConfig::default()
    } else {
        normalize_investigation_state(investigation.last)
    };
    let process_columns = parse_columns(&state.process_columns)
        .unwrap_or_else(|| ColumnPreset::Default.effective_columns().to_vec());
    let column_preset = matching_column_preset(&process_columns);
    let process_column_widths =
        ProcessColumnWidths::from_overrides(config.process_table.column_widths.iter().filter_map(
            |(label, width)| {
                parse_width_column(label).map(|column| {
                    let width = (*width).clamp(
                        i64::from(column.min_width()),
                        i64::from(crate::model::columns::PROCESS_COLUMN_WIDTH_MAX),
                    ) as u16;
                    (column, width)
                })
            },
        ));
    Ok(RuntimeConfig {
        mouse: config.general.mouse,
        config_path: None,
        recording_last_dir: config.recording.last_dir,
        initial_theme: config.general.theme,
        initial_graph_slot_layout: match state.graph_columns {
            1 => GraphSlotLayout::OneColumn,
            2 => GraphSlotLayout::TwoColumns,
            3 => GraphSlotLayout::ThreeColumns,
            _ => GraphSlotLayout::Auto,
        },
        initial_graph_templates: state.graphs,
        initial_graph_time_span_seconds: state.graph_time_span_seconds,
        initial_graph_y_axis_zero_min: state.y_axis_zero_min,
        initial_show_samples_panel: state.samples,
        initial_show_sample_delta: state.delta,
        initial_recording_interval_seconds: state.recording_interval_seconds,
        column_preset,
        process_columns,
        process_column_widths,
        sort: SortSpec {
            column: state
                .sort_by
                .parse()
                .unwrap_or(SortColumn::Metric(MetricColumn::WorksetPrivateBytes)),
            direction: state.sort_order.parse().unwrap_or(SortDirection::Desc),
        },
        initial_tracked_only: state.tracked_only,
        initial_process_view_mode: state.process_view.parse().unwrap_or(ProcessViewMode::Flat),
        initial_process_panel_height: process_panel_height(config.process_table.body_rows),
        process_filters: state.tracked_names,
        investigation_startup: investigation.startup,
        active_investigation_profile: investigation.active_profile,
        saved_investigation_profiles: config.investigation_profiles,
        sampling_options: SamplingOptions {
            collect_gpu: true,
            collect_gui_resources: true,
        },
    })
}

pub(crate) fn write_app_config(path: &Path, app: &App) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let config = AppConfig {
        general: GeneralConfig {
            mouse: app.runtime.mouse,
            theme: app.theme().name.to_string(),
        },
        graphs: GraphConfig::default(),
        process_table: ProcessTableConfig {
            view: ProcessViewMode::Flat.label().to_string(),
            preset: ColumnPreset::Default.label().to_string(),
            columns: Vec::new(),
            sort_by: String::new(),
            sort_order: String::new(),
            tracked_only: false,
            body_rows: match app.process_panel_height {
                ProcessPanelHeight::Auto => ProcessPanelHeightConfig::default(),
                ProcessPanelHeight::Manual(rows) => {
                    ProcessPanelHeightConfig::Rows(i64::from(rows.max(1)))
                }
            },
            column_widths: app
                .process_column_widths
                .overrides()
                .map(|(column, width)| (column.label().to_string(), i64::from(width)))
                .collect(),
        },
        recording: RecordingConfig {
            last_dir: app.recording_last_dir.clone(),
        },
        tracking: TrackingConfig::default(),
        tracked: Vec::new(),
        tracked_lists: Vec::new(),
        investigation: Some(InvestigationConfig {
            startup: app.runtime.investigation_startup,
            active_profile: app.active_investigation_profile.clone(),
            last: app.capture_investigation_state(),
        }),
        investigation_profiles: normalize_saved_investigation_profiles(
            app.runtime.saved_investigation_profiles.clone(),
        ),
    };
    let content = toml::to_string_pretty(&config)?;
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn normalize_saved_tracked_lists(lists: Vec<SavedTrackedList>) -> Vec<SavedTrackedList> {
    let mut normalized = Vec::<SavedTrackedList>::new();
    for mut list in lists {
        list.name = list.name.trim().to_string();
        if list.name.is_empty()
            || is_empty_tracked_list_name(&list.name)
            || normalized
                .iter()
                .any(|saved| saved.name.eq_ignore_ascii_case(&list.name))
        {
            continue;
        }
        list.processes = dedupe_process_names(list.processes);
        normalized.push(list);
    }
    normalized
}

fn normalize_saved_investigation_profiles(
    profiles: Vec<SavedInvestigationProfile>,
) -> Vec<SavedInvestigationProfile> {
    let mut normalized = Vec::<SavedInvestigationProfile>::new();
    for mut profile in profiles {
        profile.name = profile.name.trim().to_string();
        if profile.name.is_empty()
            || normalized
                .iter()
                .any(|saved| saved.name.eq_ignore_ascii_case(&profile.name))
        {
            continue;
        }
        profile.investigation = normalize_investigation_state(profile.investigation);
        normalized.push(profile);
    }
    normalized
}

fn legacy_investigation_state(config: &AppConfig) -> InvestigationStateConfig {
    let column_preset = config
        .process_table
        .preset
        .parse()
        .unwrap_or(ColumnPreset::Default);
    let columns = parse_columns(&config.process_table.columns)
        .unwrap_or_else(|| column_preset.effective_columns().to_vec());
    normalize_investigation_state(InvestigationStateConfig {
        tracked_names: config
            .tracked
            .iter()
            .map(|tracked| tracked.name.clone())
            .collect(),
        tracked_only: config.process_table.tracked_only,
        process_view: config.process_table.view.clone(),
        process_columns: columns
            .iter()
            .map(|column| column.label().to_string())
            .collect(),
        sort_by: config.process_table.sort_by.clone(),
        sort_order: config.process_table.sort_order.clone(),
        graphs: Vec::new(),
        graph_columns: config.graphs.columns,
        graph_time_span_seconds: 60,
        samples: config.graphs.samples,
        delta: config.graphs.delta,
        y_axis_zero_min: true,
        recording_interval_seconds: 1,
    })
}

fn normalize_investigation_state(mut state: InvestigationStateConfig) -> InvestigationStateConfig {
    state.tracked_names = dedupe_process_names(state.tracked_names);
    state.process_view = state
        .process_view
        .parse::<ProcessViewMode>()
        .unwrap_or(ProcessViewMode::Flat)
        .label()
        .to_string();
    state.process_columns = parse_columns(&state.process_columns)
        .unwrap_or_else(|| ColumnPreset::Default.effective_columns().to_vec())
        .iter()
        .map(|column| column.label().to_string())
        .collect();
    state.sort_by = state
        .sort_by
        .parse::<SortColumn>()
        .ok()
        .and_then(|column| match column {
            SortColumn::Metric(metric) if !metric.is_selectable() => None,
            _ => Some(column.label().to_string()),
        })
        .unwrap_or_else(|| MetricColumn::WorksetPrivateBytes.label().to_string());
    state.sort_order = state
        .sort_order
        .parse::<SortDirection>()
        .unwrap_or(SortDirection::Desc)
        .label()
        .to_string();
    state.graph_columns = match state.graph_columns {
        1..=3 => state.graph_columns,
        _ => 0,
    };
    state.graph_time_span_seconds = state.graph_time_span_seconds.clamp(60, 7_200);
    if ![1, 2, 5, 10].contains(&state.recording_interval_seconds) {
        state.recording_interval_seconds = 1;
    }
    for graph in &mut state.graphs {
        graph.kind = graph.kind.trim().to_ascii_lowercase();
        graph.metric = graph.metric.trim().to_ascii_lowercase();
        graph.display_mode = match graph.display_mode.trim().to_ascii_lowercase().as_str() {
            "ma" | "ma5" | "moving_average_5" => "ma5".to_string(),
            _ => "raw".to_string(),
        };
        graph.process_name = trimmed_option(graph.process_name.take());
        graph.executable_path = trimmed_option(graph.executable_path.take());
        graph.gpu_adapter_name = trimmed_option(graph.gpu_adapter_name.take());
    }
    state
}

fn unique_migrated_profile_name(
    legacy_name: &str,
    profiles: &[SavedInvestigationProfile],
) -> String {
    if !profiles
        .iter()
        .any(|profile| profile.name.eq_ignore_ascii_case(legacy_name))
    {
        return legacy_name.to_string();
    }
    let base = format!("{legacy_name} (Tracking List)");
    if !profiles
        .iter()
        .any(|profile| profile.name.eq_ignore_ascii_case(&base))
    {
        return base;
    }
    for suffix in 2_u32.. {
        let candidate = format!("{legacy_name} (Tracking List {suffix})");
        if !profiles
            .iter()
            .any(|profile| profile.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    unreachable!("profile suffix space exhausted")
}

fn matching_column_preset(columns: &[MetricColumn]) -> ColumnPreset {
    [
        ColumnPreset::Default,
        ColumnPreset::Memory,
        ColumnPreset::Resources,
        ColumnPreset::DotNet,
        ColumnPreset::Gpu,
        ColumnPreset::Io,
    ]
    .into_iter()
    .find(|preset| preset.effective_columns() == columns)
    .unwrap_or(ColumnPreset::Custom)
}

fn trimmed_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn dedupe_process_names(names: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::<String>::new();
    for name in names {
        let name = name.trim().to_string();
        if !name.is_empty()
            && !deduped
                .iter()
                .any(|saved| saved.eq_ignore_ascii_case(&name))
        {
            deduped.push(name);
        }
    }
    deduped
}

fn parse_columns(columns: &[String]) -> Option<Vec<MetricColumn>> {
    let parsed = columns
        .iter()
        .filter_map(|column| column.parse().ok())
        .filter(|column: &MetricColumn| column.is_selectable())
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

fn parse_width_column(label: &str) -> Option<SortColumn> {
    let column = label.parse::<SortColumn>().ok()?;
    match column {
        SortColumn::Metric(metric) if !metric.is_selectable() => None,
        _ => Some(column),
    }
}

fn process_panel_height(config: ProcessPanelHeightConfig) -> ProcessPanelHeight {
    match config {
        ProcessPanelHeightConfig::Rows(rows) if (1..=i64::from(u16::MAX)).contains(&rows) => {
            ProcessPanelHeight::Manual(rows as u16)
        }
        ProcessPanelHeightConfig::Rows(_) | ProcessPanelHeightConfig::Mode(_) => {
            ProcessPanelHeight::Auto
        }
    }
}
