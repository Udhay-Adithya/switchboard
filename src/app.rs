use crate::model::{short_uuid, Account};
use crate::ops::{self, CopyOutcome};
use crate::scan;
use crate::theme;
use eframe::egui;
use egui::{CollapsingHeader, Frame, RichText, Rounding};
use std::collections::BTreeMap;
use std::time::Instant;

const COLUMN_WIDTH: f32 = 360.0;

#[derive(Clone, Debug)]
struct DragPayload {
    account_id: String,
    file_name: String,
}

#[derive(Clone)]
struct SessionRow {
    account_id: String,
    org_id: String,
    file_name: String,
    title: String,
    cwd: String,
    last_activity_at: i64,
}

pub struct AccountManagerApp {
    accounts: Vec<Account>,
    status: Option<(String, Instant, bool)>, // message, when, is_error
    demo: bool,
}

impl AccountManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, demo: bool) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = Self {
            accounts: Vec::new(),
            status: None,
            demo,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.accounts = if self.demo {
            crate::demo::demo_accounts()
        } else {
            scan::scan_accounts()
        };
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), Instant::now(), is_error));
    }

    fn rows_for_account(account: &Account) -> Vec<SessionRow> {
        // Flatten across all org folders, de-duplicating by file_name and
        // preferring the copy that lives in the currently-authenticated org.
        let mut by_name: BTreeMap<String, (bool, SessionRow)> = BTreeMap::new();
        for org in &account.orgs {
            let is_live = account.live_org_id.as_deref() == Some(org.org_id.as_str());
            for s in &org.sessions {
                let row = SessionRow {
                    account_id: account.account_id.clone(),
                    org_id: org.org_id.clone(),
                    file_name: s.file_name.clone(),
                    title: s.title.clone(),
                    cwd: s.cwd.clone(),
                    last_activity_at: s.last_activity_at,
                };
                match by_name.get(&s.file_name) {
                    Some((existing_live, _)) if *existing_live && !is_live => {}
                    _ => {
                        by_name.insert(s.file_name.clone(), (is_live, row));
                    }
                }
            }
        }
        let mut rows: Vec<SessionRow> = by_name.into_values().map(|(_, r)| r).collect();
        rows.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
        rows
    }

    fn group_by_leaf(rows: &[SessionRow]) -> BTreeMap<String, Vec<usize>> {
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (idx, row) in rows.iter().enumerate() {
            let leaf = ops::path_leaf_label(&row.cwd);
            groups.entry(leaf).or_default().push(idx);
        }
        groups
    }

    fn render_account_column(
        &self,
        ui: &mut egui::Ui,
        account: &Account,
        rows: &[SessionRow],
    ) -> Option<DragPayload> {
        let mut dropped_here: Option<DragPayload> = None;

        ui.allocate_ui_with_layout(
            egui::vec2(COLUMN_WIDTH, ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                Frame::group(ui.style())
                    .rounding(Rounding::same(theme::RADIUS_CARD))
                    .inner_margin(theme::SPACE_LG)
                    .show(ui, |ui| {
                    ui.set_width(COLUMN_WIDTH - 16.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(short_uuid(&account.account_id)).heading());
                        if account.is_current {
                            ui.add_space(theme::SPACE_SM);
                            let (dot_rect, _) =
                                ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 3.0, theme::ACCENT);
                            ui.label(RichText::new("signed in").color(theme::ACCENT).small());
                        }
                    });

                    if account.live_org_id.is_none() {
                        ui.colored_label(
                            theme::NEGATIVE,
                            "no authenticated org, sign into this account in Claude Desktop once",
                        );
                    }

                    ui.add_space(theme::SPACE_SM);
                    ui.separator();
                    ui.add_space(theme::SPACE_XS);

                    let frame = Frame::default().inner_margin(2.0);
                    let (_resp, dropped) = ui.dnd_drop_zone::<DragPayload, ()>(frame, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(("scroll", &account.account_id))
                            .max_height(ui.available_height())
                            .show(ui, |ui| {
                                if rows.is_empty() {
                                    ui.weak("No chats found");
                                    return;
                                }
                                let groups = Self::group_by_leaf(rows);
                                for (folder, indices) in &groups {
                                    CollapsingHeader::new(
                                        RichText::new(truncate(folder, 34)).strong(),
                                    )
                                    .id_salt(("folder", &account.account_id, folder))
                                    .default_open(groups.len() <= 3)
                                    .show(ui, |ui| {
                                        for &idx in indices {
                                            Self::render_session_row(ui, &rows[idx]);
                                        }
                                    });
                                }
                            });
                    });

                    if let Some(payload) = dropped {
                        dropped_here = Some((*payload).clone());
                    }
                });
            },
        );

        dropped_here
    }

    fn render_session_row(ui: &mut egui::Ui, row: &SessionRow) {
        let drag_id = egui::Id::new(("session", &row.account_id, &row.org_id, &row.file_name));
        let payload = DragPayload {
            account_id: row.account_id.clone(),
            file_name: row.file_name.clone(),
        };

        ui.dnd_drag_source(drag_id, payload, |ui| {
            let frame = Frame::group(ui.style())
                .fill(theme::RAISED)
                .rounding(Rounding::same(theme::RADIUS_ROW))
                .inner_margin(egui::Margin::symmetric(theme::SPACE_SM, theme::SPACE_XS));
            frame.show(ui, |ui| {
                ui.set_width(COLUMN_WIDTH - 40.0);
                ui.vertical(|ui| {
                    ui.label(truncate(&row.title, 42));
                    ui.label(
                        RichText::new(ops::format_relative_time(row.last_activity_at))
                            .small()
                            .color(theme::TEXT_MUTED),
                    );
                });
            });
        });
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        t.push_str("...");
        t
    }
}

impl eframe::App for AccountManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut do_refresh = false;
        let mut pending_copy: Option<(DragPayload, String)> = None; // payload, dest_account_id

        egui::TopBottomPanel::top("top")
            .frame(
                Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(theme::SPACE_LG, theme::SPACE_MD)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("switchboard").heading().strong());
                    ui.add_space(theme::SPACE_SM);
                    if ui.button("Refresh").clicked() {
                        do_refresh = true;
                    }
                    if let Some((msg, when, is_error)) = &self.status {
                        if when.elapsed().as_secs() < 6 {
                            let color = if *is_error { theme::NEGATIVE } else { theme::POSITIVE };
                            ui.add_space(theme::SPACE_SM);
                            ui.colored_label(color, msg);
                        }
                    }
                });
                ui.add_space(theme::SPACE_XS);
                ui.label(
                    RichText::new("drag a chat onto another account to copy it there")
                        .small()
                        .color(theme::TEXT_SECONDARY),
                );
            });

        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme::BG).inner_margin(theme::SPACE_LG))
            .show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACE_LG;
                    for account in &self.accounts {
                        let rows = Self::rows_for_account(account);
                        if let Some(payload) = self.render_account_column(ui, account, &rows) {
                            if payload.account_id != account.account_id {
                                pending_copy = Some((payload, account.account_id.clone()));
                            }
                        }
                    }
                });
            });
        });

        if let Some((payload, dest_account_id)) = pending_copy {
            self.do_copy(payload, dest_account_id);
        }

        if do_refresh {
            self.refresh();
            self.set_status("refreshed", false);
        }
    }
}

impl AccountManagerApp {
    fn do_copy(&mut self, payload: DragPayload, dest_account_id: String) {
        let Some(src_account) = self
            .accounts
            .iter()
            .find(|a| a.account_id == payload.account_id)
        else {
            return;
        };
        let Some((_, src_session)) = src_account.find_session(&payload.file_name) else {
            return;
        };
        let src_path = src_session.path.clone();
        let src_turns = src_session.completed_turns;
        let src_la = src_session.last_activity_at;
        let title = src_session.title.clone();

        let Some(dest_account) = self
            .accounts
            .iter()
            .find(|a| a.account_id == dest_account_id)
        else {
            return;
        };
        let Some(live_org) = dest_account.live_org_id.clone() else {
            self.set_status(
                format!(
                    "can't copy into {}, no authenticated org yet, sign into it in Claude Desktop then refresh",
                    short_uuid(&dest_account.account_id)
                ),
                true,
            );
            return;
        };
        let dest_dir = scan::sessions_dir(&dest_account_id, &live_org);
        let dest_label = short_uuid(&dest_account.account_id);

        match ops::copy_session(&src_path, &dest_dir, src_turns, src_la) {
            Ok(CopyOutcome::Copied) => {
                self.set_status(format!("copied \"{title}\" to {dest_label}"), false);
                self.refresh();
            }
            Ok(CopyOutcome::Updated) => {
                self.set_status(
                    format!("updated \"{title}\" in {dest_label} with the richer copy"),
                    false,
                );
                self.refresh();
            }
            Ok(CopyOutcome::SkippedUpToDate) => {
                self.set_status(
                    format!("{dest_label} already has an equal or newer copy of \"{title}\""),
                    false,
                );
            }
            Err(e) => {
                self.set_status(format!("copy failed: {e}"), true);
            }
        }
    }
}
