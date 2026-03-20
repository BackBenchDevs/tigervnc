// RuVNC Viewer - Modern Rust/egui VNC viewer
// Copyright (C) 2026 BackBenchDevs
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use crate::address_book::ServerEntry;
use egui::Context;

pub struct ConnectionDialog {
    entry: ServerEntry,
    is_edit: bool,
    tags_text: String,
    result: Option<ServerEntry>,
    cancelled: bool,
    password_input: String,
    save_password: bool,
    connect_after_save: bool,
}

impl ConnectionDialog {
    pub fn wants_connect(&self) -> bool {
        self.connect_after_save
    }
}

impl ConnectionDialog {
    pub fn new(existing: Option<ServerEntry>) -> Self {
        let is_edit = existing.is_some();
        let mut entry = existing.unwrap_or_default();
        if !is_edit && entry.username.is_empty() {
            entry.username = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_default();
        }
        let tags_text = entry.tags.join(", ");
        Self {
            entry,
            is_edit,
            tags_text,
            result: None,
            cancelled: false,
            password_input: String::new(),
            save_password: true,
            connect_after_save: false,
        }
    }

    pub fn take_result(&mut self) -> Option<ServerEntry> {
        self.result.take()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn show(&mut self, ctx: &Context, groups: &[String]) {
        let title = if self.is_edit {
            "Edit Server"
        } else {
            "New Server"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("connection_form")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.entry.name);
                        ui.end_row();

                        ui.label("Host:");
                        ui.text_edit_singleline(&mut self.entry.host);
                        ui.end_row();

                        ui.label("Display/Port:");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut self.entry.port).range(0..=65535));
                            ui.weak("(0-99 = display, 100+ = TCP port)");
                        });
                        ui.end_row();

                        ui.label("Username:");
                        ui.text_edit_singleline(&mut self.entry.username);
                        ui.end_row();

                        ui.label("Password:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.password_input)
                                    .password(true),
                            );
                            ui.checkbox(&mut self.save_password, "Save");
                        });
                        ui.end_row();

                        ui.label("Group:");
                        egui::ComboBox::from_id_salt("group_select")
                            .selected_text(&self.entry.group)
                            .show_ui(ui, |ui| {
                                for group in groups {
                                    ui.selectable_value(
                                        &mut self.entry.group,
                                        group.clone(),
                                        group,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Tags:");
                        ui.text_edit_singleline(&mut self.tags_text);
                        ui.end_row();

                        ui.label("Notes:");
                        ui.text_edit_multiline(&mut self.entry.notes);
                        ui.end_row();
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let can_save = !self.entry.name.is_empty() && !self.entry.host.is_empty();

                    let do_save = |s: &mut Self| {
                        s.entry.port = crate::address_book::resolve_port(s.entry.port);

                        s.entry.tags = s
                            .tags_text
                            .split(',')
                            .map(|t| t.trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();

                        if s.save_password && !s.password_input.is_empty() {
                            crate::credentials::store_password(
                                &s.entry.host,
                                s.entry.port,
                                &s.entry.username,
                                &s.password_input,
                            )
                            .ok();
                        }

                        s.result = Some(s.entry.clone());
                    };

                    if ui
                        .add_enabled(can_save, egui::Button::new("Save & Connect"))
                        .clicked()
                    {
                        self.connect_after_save = true;
                        do_save(self);
                    }

                    if ui
                        .add_enabled(can_save, egui::Button::new("Save"))
                        .clicked()
                    {
                        do_save(self);
                    }

                    if ui.button("Cancel").clicked() {
                        self.cancelled = true;
                    }
                });
            });
    }
}
