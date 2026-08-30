use crate::{
    app::{
        App, AppActivity, FocusedPanel, GraphDisplayMode, GraphId, GraphSlot, GraphSlotLayout,
        ProcessViewMode,
        export::RECORDING_INTERVAL_OPTIONS_SECONDS,
        state::{GRAPH_LIMIT, GraphEntry, PendingTrackedListSwitch},
    },
    config::{
        InvestigationGraphConfig, InvestigationStartup, InvestigationStateConfig,
        SavedInvestigationProfile,
    },
    model::{ColumnPreset, MetricColumn, ProcessIdentity, SortColumn, SortDirection, SystemMetric},
    ui::widgets::scrollable_modal::ScrollableModalState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileNameInputPurpose {
    SaveAs,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingInvestigationProfileLoad {
    pub(crate) profile: SavedInvestigationProfile,
    pub(crate) graph_sources: Vec<ResolvedProfileGraph>,
    pub(crate) unresolved: Vec<String>,
    pub(crate) tracking_switch: PendingTrackedListSwitch,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedProfileGraph {
    source: GraphSlot,
    display_mode: GraphDisplayMode,
}

#[derive(Debug, Clone)]
pub(crate) enum InvestigationProfilesView {
    Browse,
    Startup {
        selected: InvestigationStartup,
    },
    NameInput {
        purpose: ProfileNameInputPurpose,
        draft: String,
        cursor: usize,
        error: Option<String>,
    },
    ConfirmDelete {
        name: String,
    },
    ConfirmLoad {
        pending: Box<PendingInvestigationProfileLoad>,
    },
    LoadReport {
        name: String,
        loaded_graph_count: usize,
        unresolved: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct InvestigationProfilesDialog {
    pub(crate) index: usize,
    pub(crate) scroll: ScrollableModalState,
    pub(crate) view: InvestigationProfilesView,
}

impl App {
    pub(crate) fn open_investigation_profiles(&mut self) {
        let index = self
            .active_investigation_profile
            .as_deref()
            .and_then(|active| {
                self.runtime
                    .saved_investigation_profiles
                    .iter()
                    .position(|profile| profile.name.eq_ignore_ascii_case(active))
            })
            .unwrap_or(0)
            .min(
                self.runtime
                    .saved_investigation_profiles
                    .len()
                    .saturating_sub(1),
            );
        self.investigation_profiles_dialog = Some(InvestigationProfilesDialog {
            index,
            scroll: ScrollableModalState {
                page_size: 8,
                ..ScrollableModalState::default()
            },
            view: InvestigationProfilesView::Browse,
        });
        self.ensure_investigation_profile_selection_visible();
        self.status = "Investigation Profiles".to_string();
    }

    pub(crate) fn close_investigation_profiles(&mut self) {
        self.investigation_profiles_dialog = None;
        self.status = "Ready".to_string();
    }

    pub(crate) fn investigation_profiles_view(&self) -> Option<&InvestigationProfilesView> {
        self.investigation_profiles_dialog
            .as_ref()
            .map(|dialog| &dialog.view)
    }

    pub(crate) fn investigation_profiles_index(&self) -> usize {
        self.investigation_profiles_dialog
            .as_ref()
            .map(|dialog| dialog.index)
            .unwrap_or(0)
    }

    pub(crate) fn investigation_profiles_scroll_offset(&self) -> usize {
        self.investigation_profiles_dialog
            .as_ref()
            .map(|dialog| dialog.scroll.offset)
            .unwrap_or(0)
    }

    pub(crate) fn investigation_profiles_entry_count(&self) -> usize {
        self.runtime.saved_investigation_profiles.len()
    }

    pub(crate) fn open_investigation_startup(&mut self) {
        self.investigation_profiles_dialog = Some(InvestigationProfilesDialog {
            index: 0,
            scroll: ScrollableModalState::default(),
            view: InvestigationProfilesView::Startup {
                selected: self.runtime.investigation_startup,
            },
        });
        self.status = "Startup behavior".to_string();
    }

    pub(crate) fn select_next_investigation_startup(&mut self) {
        if let Some(InvestigationProfilesView::Startup { selected }) = self
            .investigation_profiles_dialog
            .as_mut()
            .map(|dialog| &mut dialog.view)
        {
            *selected = selected.next();
        }
    }

    pub(crate) fn select_previous_investigation_startup(&mut self) {
        if let Some(InvestigationProfilesView::Startup { selected }) = self
            .investigation_profiles_dialog
            .as_mut()
            .map(|dialog| &mut dialog.view)
        {
            *selected = selected.previous();
        }
    }

    pub(crate) fn select_investigation_startup(&mut self, startup: InvestigationStartup) {
        if let Some(InvestigationProfilesView::Startup { selected }) = self
            .investigation_profiles_dialog
            .as_mut()
            .map(|dialog| &mut dialog.view)
        {
            *selected = startup;
        }
    }

    pub(crate) fn apply_selected_investigation_startup(&mut self) {
        let Some(InvestigationProfilesView::Startup { selected }) =
            self.investigation_profiles_view().cloned()
        else {
            return;
        };
        if self.set_investigation_startup(selected) {
            self.investigation_profiles_dialog = None;
            self.status = format!("Startup behavior: {}", selected.label());
        }
    }

    fn set_investigation_startup(&mut self, startup: InvestigationStartup) -> bool {
        let previous = self.runtime.investigation_startup;
        self.runtime.investigation_startup = startup;
        if self.persist_investigation_profile_changes() {
            true
        } else {
            self.runtime.investigation_startup = previous;
            false
        }
    }

    pub(crate) fn set_investigation_profiles_page_size(&mut self, page_size: usize) {
        let total = match self.investigation_profiles_view() {
            Some(InvestigationProfilesView::LoadReport { unresolved, .. }) => unresolved.len(),
            _ => self.investigation_profiles_entry_count(),
        };
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.scroll.set_page_size(page_size.max(1), total.max(1));
        }
        self.ensure_investigation_profile_selection_visible();
    }

    pub(crate) fn move_investigation_profile_selection_up(&mut self, amount: usize) {
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.index = dialog.index.saturating_sub(amount);
        }
        self.ensure_investigation_profile_selection_visible();
    }

    pub(crate) fn move_investigation_profile_selection_down(&mut self, amount: usize) {
        let last = self.investigation_profiles_entry_count().saturating_sub(1);
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.index = dialog.index.saturating_add(amount).min(last);
        }
        self.ensure_investigation_profile_selection_visible();
    }

    pub(crate) fn move_investigation_profile_selection_home(&mut self) {
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.index = 0;
        }
        self.ensure_investigation_profile_selection_visible();
    }

    pub(crate) fn move_investigation_profile_selection_end(&mut self) {
        let last = self.investigation_profiles_entry_count().saturating_sub(1);
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.index = last;
        }
        self.ensure_investigation_profile_selection_visible();
    }

    pub(crate) fn select_investigation_profile_index(&mut self, index: usize) {
        let last = self.investigation_profiles_entry_count().saturating_sub(1);
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.index = index.min(last);
        }
        self.ensure_investigation_profile_selection_visible();
    }

    fn ensure_investigation_profile_selection_visible(&mut self) {
        let total = self.investigation_profiles_entry_count();
        let Some(dialog) = self.investigation_profiles_dialog.as_mut() else {
            return;
        };
        if matches!(dialog.view, InvestigationProfilesView::LoadReport { .. }) {
            return;
        }
        dialog.index = dialog.index.min(total.saturating_sub(1));
        dialog.scroll.ensure_visible(dialog.index, total.max(1));
    }

    pub(crate) fn scroll_investigation_profile_report_up(&mut self, amount: usize) {
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.scroll.scroll_up(amount);
        }
    }

    pub(crate) fn scroll_investigation_profile_report_down(&mut self, amount: usize) {
        let total = match self.investigation_profiles_view() {
            Some(InvestigationProfilesView::LoadReport { unresolved, .. }) => unresolved.len(),
            _ => 0,
        };
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.scroll.scroll_down(amount, total);
        }
    }

    pub(crate) fn selected_investigation_profile(&self) -> Option<&SavedInvestigationProfile> {
        self.runtime
            .saved_investigation_profiles
            .get(self.investigation_profiles_index())
    }

    pub(crate) fn active_investigation_profile_dirty(&self) -> bool {
        let Some(active) = self.active_investigation_profile.as_deref() else {
            return true;
        };
        let Some(saved) = self
            .runtime
            .saved_investigation_profiles
            .iter()
            .find(|profile| profile.name.eq_ignore_ascii_case(active))
        else {
            return true;
        };
        !profiles_equivalent(
            saved,
            &self.capture_investigation_profile(saved.name.clone()),
        )
    }

    pub(crate) fn begin_save_investigation_profile_as(&mut self) {
        if !self.profile_workspace_available() {
            return;
        }
        self.investigation_profiles_dialog = Some(InvestigationProfilesDialog {
            index: 0,
            scroll: ScrollableModalState::default(),
            view: InvestigationProfilesView::NameInput {
                purpose: ProfileNameInputPurpose::SaveAs,
                draft: String::new(),
                cursor: 0,
                error: None,
            },
        });
        self.status = "Save Investigation Profile As".to_string();
    }

    pub(crate) fn save_active_investigation_profile(&mut self) {
        if !self.profile_workspace_available() {
            return;
        }
        let Some(active) = self.active_investigation_profile.clone() else {
            self.begin_save_investigation_profile_as();
            return;
        };
        let Some(index) = self
            .runtime
            .saved_investigation_profiles
            .iter()
            .position(|profile| profile.name.eq_ignore_ascii_case(&active))
        else {
            self.active_investigation_profile = None;
            self.begin_save_investigation_profile_as();
            return;
        };

        let previous = self.runtime.saved_investigation_profiles[index].clone();
        self.runtime.saved_investigation_profiles[index] =
            self.capture_investigation_profile(previous.name.clone());
        if self.persist_investigation_profile_changes() {
            self.status = format!("Saved Investigation Profile: {}", previous.name);
        } else {
            self.runtime.saved_investigation_profiles[index] = previous;
        }
    }

    pub(crate) fn cancel_investigation_profile_subdialog(&mut self) {
        let close_dialog = matches!(
            self.investigation_profiles_view(),
            Some(InvestigationProfilesView::Startup { .. })
                | Some(InvestigationProfilesView::NameInput { .. })
        );
        if close_dialog {
            self.close_investigation_profiles();
            return;
        }
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.view = InvestigationProfilesView::Browse;
            dialog.scroll.offset = 0;
        }
        self.ensure_investigation_profile_selection_visible();
        self.status = "Investigation Profiles".to_string();
    }

    pub(crate) fn push_investigation_profile_name_char(&mut self, ch: char) {
        if let Some((draft, cursor, error)) = self.profile_name_input_mut() {
            *error = None;
            *cursor = (*cursor).min(draft.len());
            draft.insert(*cursor, ch);
            *cursor += ch.len_utf8();
        }
    }

    pub(crate) fn pop_investigation_profile_name_char(&mut self) {
        let Some((draft, cursor, error)) = self.profile_name_input_mut() else {
            return;
        };
        if *cursor == 0 {
            return;
        }
        *error = None;
        let previous = draft[..*cursor]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0);
        draft.drain(previous..*cursor);
        *cursor = previous;
    }

    pub(crate) fn delete_investigation_profile_name_char(&mut self) {
        let Some((draft, cursor, error)) = self.profile_name_input_mut() else {
            return;
        };
        if *cursor >= draft.len() {
            return;
        }
        *error = None;
        let next = draft[*cursor..]
            .chars()
            .next()
            .map(|ch| *cursor + ch.len_utf8())
            .unwrap_or(draft.len());
        draft.drain(*cursor..next);
    }

    pub(crate) fn move_investigation_profile_name_cursor_left(&mut self) {
        let Some((draft, cursor, _)) = self.profile_name_input_mut() else {
            return;
        };
        if *cursor > 0 {
            *cursor = draft[..*cursor]
                .char_indices()
                .last()
                .map(|(index, _)| index)
                .unwrap_or(0);
        }
    }

    pub(crate) fn move_investigation_profile_name_cursor_right(&mut self) {
        let Some((draft, cursor, _)) = self.profile_name_input_mut() else {
            return;
        };
        if *cursor < draft.len() {
            *cursor = draft[*cursor..]
                .chars()
                .next()
                .map(|ch| *cursor + ch.len_utf8())
                .unwrap_or(draft.len());
        }
    }

    pub(crate) fn move_investigation_profile_name_cursor_home(&mut self) {
        if let Some((_, cursor, _)) = self.profile_name_input_mut() {
            *cursor = 0;
        }
    }

    pub(crate) fn move_investigation_profile_name_cursor_end(&mut self) {
        if let Some((draft, cursor, _)) = self.profile_name_input_mut() {
            *cursor = draft.len();
        }
    }

    fn profile_name_input_mut(&mut self) -> Option<(&mut String, &mut usize, &mut Option<String>)> {
        let dialog = self.investigation_profiles_dialog.as_mut()?;
        let InvestigationProfilesView::NameInput {
            draft,
            cursor,
            error,
            ..
        } = &mut dialog.view
        else {
            return None;
        };
        Some((draft, cursor, error))
    }

    pub(crate) fn commit_investigation_profile_name_input(&mut self) {
        let Some((purpose, name)) =
            self.investigation_profiles_dialog
                .as_ref()
                .and_then(|dialog| {
                    let InvestigationProfilesView::NameInput { purpose, draft, .. } = &dialog.view
                    else {
                        return None;
                    };
                    Some((*purpose, draft.trim().to_string()))
                })
        else {
            return;
        };
        if name.is_empty() {
            self.set_investigation_profile_name_error("Name is required.");
            return;
        }

        match purpose {
            ProfileNameInputPurpose::SaveAs => self.save_investigation_profile_as(name),
        }
    }

    fn set_investigation_profile_name_error(&mut self, message: &str) {
        if let Some((_, _, error)) = self.profile_name_input_mut() {
            *error = Some(message.to_string());
        }
        self.status = message.to_string();
    }

    fn save_investigation_profile_as(&mut self, name: String) {
        if self
            .runtime
            .saved_investigation_profiles
            .iter()
            .any(|profile| profile.name.eq_ignore_ascii_case(&name))
        {
            self.set_investigation_profile_name_error(
                "An Investigation Profile with that name already exists.",
            );
            return;
        }
        let previous_active = self.active_investigation_profile.clone();
        self.runtime
            .saved_investigation_profiles
            .push(self.capture_investigation_profile(name.clone()));
        self.active_investigation_profile = Some(name.clone());
        if self.persist_investigation_profile_changes() {
            self.investigation_profiles_dialog = None;
            self.status = format!("Saved Investigation Profile: {name}");
        } else {
            self.runtime.saved_investigation_profiles.pop();
            self.active_investigation_profile = previous_active;
            self.set_investigation_profile_name_error("Save failed.");
        }
    }

    pub(crate) fn request_delete_selected_investigation_profile(&mut self) {
        let Some(profile) = self.selected_investigation_profile() else {
            self.status = "No Investigation Profile selected".to_string();
            return;
        };
        let name = profile.name.clone();
        if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
            dialog.view = InvestigationProfilesView::ConfirmDelete { name };
        }
    }

    pub(crate) fn confirm_investigation_profile_action(&mut self) {
        let view = self
            .investigation_profiles_dialog
            .as_ref()
            .map(|dialog| dialog.view.clone());
        match view {
            Some(InvestigationProfilesView::ConfirmDelete { name }) => {
                self.delete_investigation_profile(name)
            }
            Some(InvestigationProfilesView::ConfirmLoad { pending }) => {
                self.apply_investigation_profile_load(*pending)
            }
            _ => {}
        }
    }

    fn delete_investigation_profile(&mut self, name: String) {
        let previous_profiles = self.runtime.saved_investigation_profiles.clone();
        let previous_active = self.active_investigation_profile.clone();
        self.runtime
            .saved_investigation_profiles
            .retain(|profile| !profile.name.eq_ignore_ascii_case(&name));
        if self
            .active_investigation_profile
            .as_deref()
            .is_some_and(|active| active.eq_ignore_ascii_case(&name))
        {
            self.active_investigation_profile = None;
        }
        if self.persist_investigation_profile_changes() {
            let last = self.investigation_profiles_entry_count().saturating_sub(1);
            if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
                dialog.index = dialog.index.min(last);
                dialog.view = InvestigationProfilesView::Browse;
            }
            self.ensure_investigation_profile_selection_visible();
            self.status = format!("Deleted Investigation Profile: {name}");
        } else {
            self.runtime.saved_investigation_profiles = previous_profiles;
            self.active_investigation_profile = previous_active;
        }
    }

    pub(crate) fn load_selected_investigation_profile(&mut self) {
        if !self.profile_workspace_available() {
            return;
        }
        let Some(profile) = self.selected_investigation_profile().cloned() else {
            self.status = "No Investigation Profile selected".to_string();
            return;
        };
        let (graph_sources, unresolved) = self.resolve_profile_graphs(&profile);
        let tracking_switch = self.prepare_tracked_list_switch(profile.tracked_names.clone());
        let pending = PendingInvestigationProfileLoad {
            profile,
            graph_sources,
            unresolved,
            tracking_switch,
        };
        if pending.tracking_switch.discarded_sample_count > 0 {
            if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
                dialog.view = InvestigationProfilesView::ConfirmLoad {
                    pending: Box::new(pending),
                };
            }
            self.status = "Loading this profile will discard older samples".to_string();
        } else {
            self.apply_investigation_profile_load(pending);
        }
    }

    pub(crate) fn restore_initial_investigation_graphs(
        &mut self,
        templates: Vec<InvestigationGraphConfig>,
    ) {
        if templates.is_empty() {
            return;
        }
        let name = self
            .active_investigation_profile
            .clone()
            .unwrap_or_else(|| "Last investigation".to_string());
        let mut profile = SavedInvestigationProfile {
            name: name.clone(),
            ..SavedInvestigationProfile::default()
        };
        profile.graphs = templates;
        let (graphs, unresolved) = self.resolve_profile_graphs(&profile);
        self.replace_profile_graphs(graphs);
        self.select_details_sample_latest();
        self.sync_graph_layout_visibility();
        self.reveal_active_graph();
        if !unresolved.is_empty() {
            let index = self
                .runtime
                .saved_investigation_profiles
                .iter()
                .position(|profile| profile.name.eq_ignore_ascii_case(&name))
                .unwrap_or(0);
            let unresolved_count = unresolved.len();
            self.investigation_profiles_dialog = Some(InvestigationProfilesDialog {
                index,
                scroll: ScrollableModalState::default(),
                view: InvestigationProfilesView::LoadReport {
                    name,
                    loaded_graph_count: self.graph_entries.len(),
                    unresolved,
                },
            });
            self.status =
                format!("Investigation restored with {unresolved_count} unresolved Graphs");
        }
    }

    fn profile_workspace_available(&mut self) -> bool {
        match self.activity() {
            AppActivity::Live => true,
            AppActivity::Recording => {
                self.status =
                    "Investigation Profiles cannot save or load during Recording".to_string();
                false
            }
            AppActivity::LogView => {
                self.status = "Investigation Profiles cannot save or load in Log view".to_string();
                false
            }
        }
    }

    fn apply_investigation_profile_load(&mut self, pending: PendingInvestigationProfileLoad) {
        if !self.profile_workspace_available() {
            return;
        }
        let PendingInvestigationProfileLoad {
            profile,
            graph_sources,
            unresolved,
            tracking_switch,
        } = pending;
        self.apply_tracked_list_switch(tracking_switch);
        self.watch_enabled = profile.tracked_only;

        let columns = profile
            .process_columns
            .iter()
            .filter_map(|column| column.parse::<MetricColumn>().ok())
            .filter(|column| column.is_selectable())
            .collect::<Vec<_>>();
        self.process_columns = if columns.is_empty() {
            ColumnPreset::Default.effective_columns().to_vec()
        } else {
            columns
        };
        self.column_preset = ColumnPreset::Custom;
        self.process_view_mode = profile
            .process_view
            .parse::<ProcessViewMode>()
            .unwrap_or(ProcessViewMode::Flat);
        self.sort = crate::model::SortSpec {
            column: profile
                .sort_by
                .parse::<SortColumn>()
                .unwrap_or(SortColumn::Metric(MetricColumn::WorksetPrivateBytes)),
            direction: profile
                .sort_order
                .parse::<SortDirection>()
                .unwrap_or(SortDirection::Desc),
        };
        self.ensure_sort_column_visible();
        self.clamp_selected_process_column();
        self.refresh_process_order();

        self.replace_profile_graphs(graph_sources);
        self.graph_slot_layout = match profile.graph_columns {
            1 => GraphSlotLayout::OneColumn,
            2 => GraphSlotLayout::TwoColumns,
            3 => GraphSlotLayout::ThreeColumns,
            _ => GraphSlotLayout::Auto,
        };
        self.graph_time_span_seconds = profile.graph_time_span_seconds.clamp(60, 7_200);
        self.graph_time_offset_seconds = 0;
        self.graph_time_window_right_at = None;
        self.graph_show_all_samples = false;
        self.graph_y_axis_zero_min = profile.y_axis_zero_min;
        self.show_samples_panel = profile.samples;
        self.samples_temporarily_collapsed = false;
        self.show_sample_delta = profile.delta;
        self.ab_comparison = None;
        self.details_live = true;
        self.graph_scroll_row = 0;
        self.show_details = !self.graph_entries.is_empty();
        self.recording_interval_index = RECORDING_INTERVAL_OPTIONS_SECONDS
            .iter()
            .position(|seconds| *seconds == profile.recording_interval_seconds)
            .unwrap_or(0);
        self.select_details_sample_latest();
        self.sync_graph_layout_visibility();
        self.reveal_active_graph();
        self.ensure_visible_panel_focus();
        if self.graph_entries.is_empty()
            && matches!(
                self.focused_panel,
                FocusedPanel::DetailsGraph | FocusedPanel::DetailsSamples
            )
        {
            self.focused_panel = FocusedPanel::Processes;
        }
        self.active_investigation_profile = Some(profile.name.clone());
        if !self.persist_investigation_profile_changes() {
            return;
        }

        let loaded_graph_count = self.graph_entries.len();
        if unresolved.is_empty() {
            self.investigation_profiles_dialog = None;
            self.status = format!(
                "Loaded Investigation Profile: {} ({} Graph{})",
                profile.name,
                loaded_graph_count,
                if loaded_graph_count == 1 { "" } else { "s" }
            );
        } else {
            let unresolved_count = unresolved.len();
            if let Some(dialog) = self.investigation_profiles_dialog.as_mut() {
                dialog.scroll.offset = 0;
                dialog.view = InvestigationProfilesView::LoadReport {
                    name: profile.name.clone(),
                    loaded_graph_count,
                    unresolved,
                };
            }
            self.status = format!(
                "Loaded Investigation Profile: {} ({} unresolved)",
                profile.name, unresolved_count
            );
        }
    }

    fn replace_profile_graphs(&mut self, graphs: Vec<ResolvedProfileGraph>) {
        self.graph_entries.clear();
        self.active_graph_id = None;
        for graph in graphs {
            let id = GraphId(self.next_graph_id);
            self.next_graph_id = self
                .next_graph_id
                .checked_add(1)
                .expect("GraphId space exhausted");
            self.graph_entries.push(GraphEntry {
                id,
                source: graph.source,
                display_mode: graph.display_mode,
            });
            if self.active_graph_id.is_none() {
                self.active_graph_id = Some(id);
            }
        }
        self.show_details = !self.graph_entries.is_empty();
    }

    fn resolve_profile_graphs(
        &self,
        profile: &SavedInvestigationProfile,
    ) -> (Vec<ResolvedProfileGraph>, Vec<String>) {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();
        for (index, template) in profile.graphs.iter().enumerate() {
            let label = graph_template_label(template);
            if resolved.len() >= GRAPH_LIMIT {
                unresolved.push(format!("{}: Graph limit reached · {label}", index + 1));
                continue;
            }
            match self.resolve_profile_graph(template) {
                Ok(source)
                    if resolved
                        .iter()
                        .any(|resolved: &ResolvedProfileGraph| resolved.source == source) =>
                {
                    unresolved.push(format!("{}: Duplicate source · {label}", index + 1));
                }
                Ok(source) => resolved.push(ResolvedProfileGraph {
                    source,
                    display_mode: graph_display_mode(&template.display_mode),
                }),
                Err(reason) => unresolved.push(format!("{}: {reason} · {label}", index + 1)),
            }
        }
        (resolved, unresolved)
    }

    fn resolve_profile_graph(
        &self,
        template: &InvestigationGraphConfig,
    ) -> Result<GraphSlot, String> {
        match template.kind.as_str() {
            "process" => {
                let metric = parse_process_metric(&template.metric)
                    .ok_or_else(|| "Invalid process metric".to_string())?;
                let name = template
                    .process_name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .ok_or_else(|| "Missing process name".to_string())?;
                let candidates = self
                    .snapshot
                    .processes
                    .iter()
                    .filter(|process| process.name.eq_ignore_ascii_case(name))
                    .filter(|process| {
                        template.executable_path.as_deref().is_none_or(|expected| {
                            process
                                .executable_path
                                .as_deref()
                                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                        })
                    })
                    .collect::<Vec<_>>();
                match candidates.as_slice() {
                    [process] => Ok(GraphSlot::process(
                        ProcessIdentity::from_row(process),
                        metric,
                    )),
                    [] => Err("Process target unavailable".to_string()),
                    _ => Err("Process target ambiguous".to_string()),
                }
            }
            "system" => {
                let metric = parse_system_metric(&template.metric)
                    .filter(|metric| !is_gpu_metric(*metric))
                    .ok_or_else(|| "Invalid system metric".to_string())?;
                Ok(GraphSlot::system(metric))
            }
            "gpu" => {
                let metric = parse_system_metric(&template.metric)
                    .filter(|metric| is_gpu_metric(*metric))
                    .ok_or_else(|| "Invalid GPU metric".to_string())?;
                let name = template
                    .gpu_adapter_name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .ok_or_else(|| "Missing GPU adapter name".to_string())?;
                let candidates = self
                    .snapshot
                    .gpu_adapters
                    .iter()
                    .filter(|adapter| {
                        adapter
                            .name
                            .as_deref()
                            .is_some_and(|actual| actual.eq_ignore_ascii_case(name))
                    })
                    .collect::<Vec<_>>();
                match candidates.as_slice() {
                    [adapter] => Ok(GraphSlot::gpu(adapter.id, name, metric)),
                    [] => Err("GPU adapter unavailable".to_string()),
                    _ => Err("GPU adapter ambiguous".to_string()),
                }
            }
            _ => Err("Invalid Graph kind".to_string()),
        }
    }

    fn capture_investigation_profile(&self, name: String) -> SavedInvestigationProfile {
        SavedInvestigationProfile {
            name,
            investigation: self.capture_investigation_state(),
        }
    }

    pub(crate) fn capture_investigation_state(&self) -> InvestigationStateConfig {
        InvestigationStateConfig {
            tracked_names: self.watch_list.clone(),
            tracked_only: self.watch_enabled,
            process_view: self.process_view_mode.label().to_string(),
            process_columns: self
                .process_columns
                .iter()
                .map(|column| column.label().to_string())
                .collect(),
            sort_by: self.sort.column.label().to_string(),
            sort_order: self.sort.direction.label().to_string(),
            graphs: self
                .graph_entries
                .iter()
                .map(|entry| self.capture_graph_template(entry))
                .collect(),
            graph_columns: self.graph_slot_layout.columns(),
            graph_time_span_seconds: self.graph_time_span_seconds.clamp(60, 7_200),
            samples: self.show_samples_panel,
            delta: self.show_sample_delta,
            y_axis_zero_min: self.graph_y_axis_zero_min,
            recording_interval_seconds: self.selected_recording_interval_seconds(),
        }
    }

    fn capture_graph_template(&self, entry: &GraphEntry) -> InvestigationGraphConfig {
        let display_mode = graph_display_mode_id(entry.display_mode).to_string();
        match &entry.source {
            GraphSlot::Process { identity, metric } => InvestigationGraphConfig {
                kind: "process".to_string(),
                metric: process_metric_id(*metric).to_string(),
                display_mode,
                process_name: Some(identity.name.clone()),
                executable_path: self
                    .snapshot
                    .processes
                    .iter()
                    .find(|process| ProcessIdentity::from_row(process) == *identity)
                    .and_then(|process| process.executable_path.clone()),
                gpu_adapter_name: None,
            },
            GraphSlot::System { metric } => InvestigationGraphConfig {
                kind: "system".to_string(),
                metric: system_metric_id(*metric).to_string(),
                display_mode,
                process_name: None,
                executable_path: None,
                gpu_adapter_name: None,
            },
            GraphSlot::Gpu {
                adapter_name,
                metric,
                ..
            } => InvestigationGraphConfig {
                kind: "gpu".to_string(),
                metric: system_metric_id(*metric).to_string(),
                display_mode,
                process_name: None,
                executable_path: None,
                gpu_adapter_name: Some(adapter_name.clone()),
            },
        }
    }

    fn persist_investigation_profile_changes(&mut self) -> bool {
        let Some(path) = self.runtime.config_path.clone() else {
            return true;
        };
        match crate::config::write_app_config(&path, self) {
            Ok(()) => true,
            Err(error) => {
                self.status = format!("Failed to save Investigation Profiles: {error}");
                false
            }
        }
    }
}

fn graph_template_label(template: &InvestigationGraphConfig) -> String {
    match template.kind.as_str() {
        "process" => format!(
            "{} · {}",
            template.process_name.as_deref().unwrap_or("<process>"),
            template.metric
        ),
        "gpu" => format!(
            "{} · {}",
            template.gpu_adapter_name.as_deref().unwrap_or("<GPU>"),
            template.metric
        ),
        _ => format!("{} · {}", template.kind, template.metric),
    }
}

fn profiles_equivalent(
    left: &SavedInvestigationProfile,
    right: &SavedInvestigationProfile,
) -> bool {
    left.name.eq_ignore_ascii_case(&right.name)
        && strings_equal_case_insensitively(&left.tracked_names, &right.tracked_names)
        && left.tracked_only == right.tracked_only
        && left.process_view.eq_ignore_ascii_case(&right.process_view)
        && left.process_columns == right.process_columns
        && left.sort_by.eq_ignore_ascii_case(&right.sort_by)
        && left.sort_order.eq_ignore_ascii_case(&right.sort_order)
        && left.graphs.len() == right.graphs.len()
        && left
            .graphs
            .iter()
            .zip(&right.graphs)
            .all(|(left, right)| graph_templates_equivalent(left, right))
        && left.graph_columns == right.graph_columns
        && left.graph_time_span_seconds == right.graph_time_span_seconds
        && left.samples == right.samples
        && left.delta == right.delta
        && left.y_axis_zero_min == right.y_axis_zero_min
        && left.recording_interval_seconds == right.recording_interval_seconds
}

fn strings_equal_case_insensitively(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn graph_templates_equivalent(
    left: &InvestigationGraphConfig,
    right: &InvestigationGraphConfig,
) -> bool {
    left.kind.eq_ignore_ascii_case(&right.kind)
        && left.metric.eq_ignore_ascii_case(&right.metric)
        && graph_display_mode(&left.display_mode) == graph_display_mode(&right.display_mode)
        && optional_strings_equal_case_insensitively(
            left.process_name.as_deref(),
            right.process_name.as_deref(),
        )
        && left.executable_path.as_deref().is_none_or(|expected| {
            right
                .executable_path
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
        })
        && optional_strings_equal_case_insensitively(
            left.gpu_adapter_name.as_deref(),
            right.gpu_adapter_name.as_deref(),
        )
}

fn graph_display_mode(value: &str) -> GraphDisplayMode {
    match value.trim().to_ascii_lowercase().as_str() {
        "ma" | "ma5" | "moving_average_5" => GraphDisplayMode::MovingAverage5,
        _ => GraphDisplayMode::Raw,
    }
}

fn graph_display_mode_id(mode: GraphDisplayMode) -> &'static str {
    match mode {
        GraphDisplayMode::Raw => "raw",
        GraphDisplayMode::MovingAverage5 => "ma5",
    }
}

fn optional_strings_equal_case_insensitively(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        (None, None) => true,
        _ => false,
    }
}

fn process_metric_id(metric: crate::app::DetailsMetric) -> &'static str {
    use crate::app::DetailsMetric::*;
    match metric {
        CpuPercent => "cpu_percent",
        Private => "private_bytes",
        Workset => "workset_bytes",
        WorksetPrivate => "workset_private_bytes",
        WorksetShareable => "workset_shareable_bytes",
        ThreadCount => "thread_count",
        HandleCount => "handle_count",
        UserObjectCount => "user_object_count",
        GdiObjectCount => "gdi_object_count",
        GpuPercent => "gpu_percent",
        DotNetHeap => "dotnet_heap_bytes",
        DotNetGcGen0Heap => "dotnet_gc_gen0_heap_bytes",
        DotNetGcGen1Heap => "dotnet_gc_gen1_heap_bytes",
        DotNetGcGen2Heap => "dotnet_gc_gen2_heap_bytes",
        DotNetGcLoh => "dotnet_gc_loh_bytes",
        DotNetGcPoh => "dotnet_gc_poh_bytes",
        DotNetGcCommitted => "dotnet_gc_committed_bytes",
        DotNetGcFragmentation => "dotnet_gc_fragmentation_bytes",
        DotNetAllocation => "dotnet_allocation_bytes_per_sec",
        GpuDedicated => "gpu_dedicated_bytes",
        GpuShared => "gpu_shared_bytes",
        IoRead => "io_read_bytes_per_sec",
        IoWrite => "io_write_bytes_per_sec",
    }
}

fn parse_process_metric(value: &str) -> Option<crate::app::DetailsMetric> {
    use crate::app::DetailsMetric::*;
    match value {
        "cpu_percent" => Some(CpuPercent),
        "private_bytes" => Some(Private),
        "workset_bytes" => Some(Workset),
        "workset_private_bytes" => Some(WorksetPrivate),
        "workset_shareable_bytes" => Some(WorksetShareable),
        "thread_count" => Some(ThreadCount),
        "handle_count" => Some(HandleCount),
        "user_object_count" => Some(UserObjectCount),
        "gdi_object_count" => Some(GdiObjectCount),
        "gpu_percent" => Some(GpuPercent),
        "dotnet_heap_bytes" => Some(DotNetHeap),
        "dotnet_gc_gen0_heap_bytes" => Some(DotNetGcGen0Heap),
        "dotnet_gc_gen1_heap_bytes" => Some(DotNetGcGen1Heap),
        "dotnet_gc_gen2_heap_bytes" => Some(DotNetGcGen2Heap),
        "dotnet_gc_loh_bytes" => Some(DotNetGcLoh),
        "dotnet_gc_poh_bytes" => Some(DotNetGcPoh),
        "dotnet_gc_committed_bytes" => Some(DotNetGcCommitted),
        "dotnet_gc_fragmentation_bytes" => Some(DotNetGcFragmentation),
        "dotnet_allocation_bytes_per_sec" => Some(DotNetAllocation),
        "gpu_dedicated_bytes" => Some(GpuDedicated),
        "gpu_shared_bytes" => Some(GpuShared),
        "io_read_bytes_per_sec" => Some(IoRead),
        "io_write_bytes_per_sec" => Some(IoWrite),
        _ => None,
    }
}

fn system_metric_id(metric: SystemMetric) -> &'static str {
    match metric {
        SystemMetric::CpuAverage => "cpu_average",
        SystemMetric::PhysicalMemory => "physical_memory",
        SystemMetric::ModifiedMemory => "modified_memory",
        SystemMetric::StandbyMemory => "standby_memory",
        SystemMetric::FreeZeroedMemory => "free_zeroed_memory",
        SystemMetric::Committed => "committed_memory",
        SystemMetric::PagedPool => "paged_pool",
        SystemMetric::NonpagedPool => "nonpaged_pool",
        SystemMetric::PagesInput => "pages_input_per_sec",
        SystemMetric::PagesOutput => "pages_output_per_sec",
        SystemMetric::ThreadCount => "thread_count",
        SystemMetric::ProcessCount => "process_count",
        SystemMetric::GpuUtilization => "gpu_utilization",
        SystemMetric::GpuEncode => "gpu_encode",
        SystemMetric::GpuDecode => "gpu_decode",
        SystemMetric::GpuDedicated => "gpu_dedicated",
        SystemMetric::GpuShared => "gpu_shared",
        SystemMetric::NetworkReceived => "network_received",
        SystemMetric::NetworkSent => "network_sent",
        SystemMetric::DiskRead => "disk_read",
        SystemMetric::DiskWrite => "disk_write",
        SystemMetric::DiskQueueLength => "disk_queue_length",
    }
}

fn parse_system_metric(value: &str) -> Option<SystemMetric> {
    match value {
        "cpu_average" => Some(SystemMetric::CpuAverage),
        "physical_memory" => Some(SystemMetric::PhysicalMemory),
        "modified_memory" => Some(SystemMetric::ModifiedMemory),
        "standby_memory" => Some(SystemMetric::StandbyMemory),
        "free_zeroed_memory" => Some(SystemMetric::FreeZeroedMemory),
        "committed_memory" => Some(SystemMetric::Committed),
        "paged_pool" => Some(SystemMetric::PagedPool),
        "nonpaged_pool" => Some(SystemMetric::NonpagedPool),
        "pages_input_per_sec" => Some(SystemMetric::PagesInput),
        "pages_output_per_sec" => Some(SystemMetric::PagesOutput),
        "thread_count" => Some(SystemMetric::ThreadCount),
        "process_count" => Some(SystemMetric::ProcessCount),
        "gpu_utilization" => Some(SystemMetric::GpuUtilization),
        "gpu_encode" => Some(SystemMetric::GpuEncode),
        "gpu_decode" => Some(SystemMetric::GpuDecode),
        "gpu_dedicated" => Some(SystemMetric::GpuDedicated),
        "gpu_shared" => Some(SystemMetric::GpuShared),
        "network_received" => Some(SystemMetric::NetworkReceived),
        "network_sent" => Some(SystemMetric::NetworkSent),
        "disk_read" => Some(SystemMetric::DiskRead),
        "disk_write" => Some(SystemMetric::DiskWrite),
        "disk_queue_length" => Some(SystemMetric::DiskQueueLength),
        _ => None,
    }
}

fn is_gpu_metric(metric: SystemMetric) -> bool {
    matches!(
        metric,
        SystemMetric::GpuUtilization
            | SystemMetric::GpuEncode
            | SystemMetric::GpuDecode
            | SystemMetric::GpuDedicated
            | SystemMetric::GpuShared
    )
}
