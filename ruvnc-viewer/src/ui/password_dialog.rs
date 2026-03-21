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

use egui::Context;

pub struct PasswordDialog {
    host: String,
    port: u16,
    username: String,
    password: String,
    save_password: bool,
    need_username: bool,
    result: Option<PasswordResult>,
    cancelled: bool,
}

pub struct PasswordResult {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub save: bool,
}

impl PasswordDialog {
    pub fn new(host: String, port: u16, username: String, need_username: bool) -> Self {
        Self {
            host,
            port,
            username,
            password: String::new(),
            save_password: true,
            need_username,
            result: None,
            cancelled: false,
        }
    }

    pub fn take_result(&mut self) -> Option<PasswordResult> {
        self.result.take()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn show(&mut self, ctx: &Context) {
        let title = format!("Password for {}:{}", self.host, self.port);
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(350.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(format!("Server: {}:{}", self.host, self.port));
                    ui.add_space(4.0);

                    egui::Grid::new("password_form")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            if self.need_username {
                                ui.label("Username:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.username)
                                        .desired_width(200.0),
                                );
                                ui.end_row();
                            }

                            ui.label("Password:");
                            let pw_response = ui.add(
                                egui::TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .desired_width(200.0),
                            );
                            ui.end_row();

                            pw_response.request_focus();

                            ui.label("");
                            ui.checkbox(&mut self.save_password, "Save password");
                            ui.end_row();
                        });

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let can_connect = !self.password.is_empty();
                        if ui
                            .add_enabled(can_connect, egui::Button::new("Connect"))
                            .clicked()
                            || (enter_pressed && can_connect)
                        {
                            self.result = Some(PasswordResult {
                                username: self.username.clone(),
                                password: self.password.clone(),
                                host: self.host.clone(),
                                port: self.port,
                                save: self.save_password,
                            });
                        }
                        if ui.button("Cancel").clicked() {
                            self.cancelled = true;
                        }
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_dialog_new() {
        let d = PasswordDialog::new("host".into(), 5900, "user".into(), false);
        assert_eq!(d.host, "host");
        assert_eq!(d.port, 5900);
        assert_eq!(d.username, "user");
        assert!(d.password.is_empty());
        assert!(d.save_password);
        assert!(!d.need_username);
        assert!(!d.is_cancelled());
        assert!(d.result.is_none());
    }

    #[test]
    fn test_password_dialog_need_username() {
        let d = PasswordDialog::new("host".into(), 5900, "user".into(), true);
        assert!(d.need_username);
    }

    #[test]
    fn test_password_dialog_cancel() {
        let mut d = PasswordDialog::new("h".into(), 1, "u".into(), false);
        d.cancelled = true;
        assert!(d.is_cancelled());
    }

    #[test]
    fn test_password_result_fields() {
        let r = PasswordResult {
            username: "admin".into(),
            password: "secret".into(),
            host: "srv".into(),
            port: 5901,
            save: true,
        };
        assert_eq!(r.username, "admin");
        assert_eq!(r.password, "secret");
        assert!(r.save);
    }
}
