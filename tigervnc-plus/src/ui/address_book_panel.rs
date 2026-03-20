use crate::address_book::{AddressBook, ServerEntry};
use egui::{Color32, RichText, Ui};

pub enum Action {
    Connect(String),
    QuickConnect(String, u16),
    Edit(String),
    Delete(String),
    Add,
}

pub struct AddressBookPanel {
    search_query: String,
    selected_group: Option<String>,
    selected_server: Option<String>,
    pending_action: Option<Action>,
    confirm_delete: Option<String>,
    quick_connect_input: String,
}

impl AddressBookPanel {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_group: None,
            selected_server: None,
            pending_action: None,
            confirm_delete: None,
            quick_connect_input: String::new(),
        }
    }

    pub fn take_action(&mut self) -> Option<Action> {
        self.pending_action.take()
    }

    fn parse_host_port(input: &str) -> (String, u16) {
        let trimmed = input.trim();
        if let Some(colon_pos) = trimmed.rfind(':') {
            let host_part = &trimmed[..colon_pos];
            let port_part = &trimmed[colon_pos + 1..];
            if let Ok(port) = port_part.parse::<u16>() {
                if port < 100 {
                    return (host_part.to_string(), 5900 + port);
                }
                return (host_part.to_string(), port);
            }
        }
        (trimmed.to_string(), 5900)
    }

    pub fn show(&mut self, ui: &mut Ui, book: &AddressBook) {
        ui.horizontal(|ui| {
            ui.heading("Address Book");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New Server").clicked() {
                    self.pending_action = Some(Action::Add);
                }
            });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("VNC Server:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.quick_connect_input)
                    .hint_text("host[:port]  e.g. 192.168.1.50:1")
                    .desired_width(250.0),
            );
            let connect_clicked = ui.button("Connect").clicked();
            let enter_pressed = response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if (connect_clicked || enter_pressed)
                && !self.quick_connect_input.trim().is_empty()
            {
                let (host, port) = Self::parse_host_port(&self.quick_connect_input);
                self.pending_action = Some(Action::QuickConnect(host, port));
                self.quick_connect_input.clear();
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            ui.separator();
            ui.label("Group:");
            egui::ComboBox::from_id_salt("group_filter")
                .selected_text(
                    self.selected_group
                        .as_deref()
                        .unwrap_or("All"),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.selected_group.is_none(), "All")
                        .clicked()
                    {
                        self.selected_group = None;
                    }
                    for group in &book.groups {
                        if ui
                            .selectable_label(
                                self.selected_group.as_deref() == Some(group.as_str()),
                                group,
                            )
                            .clicked()
                        {
                            self.selected_group = Some(group.clone());
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.separator();

        let filtered: Vec<&ServerEntry> = book
            .servers
            .iter()
            .filter(|s| s.matches_search(&self.search_query))
            .filter(|s| {
                self.selected_group
                    .as_ref()
                    .map_or(true, |g| &s.group == g)
            })
            .collect();

        if filtered.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("No servers found")
                        .size(18.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(12.0);
                if book.servers.is_empty() {
                    ui.label("Click '+ New Server' to add your first connection.");
                } else {
                    ui.label("Try adjusting your search or group filter.");
                }
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut current_group = String::new();
            let mut sorted = filtered;
            sorted.sort_by(|a, b| a.group.cmp(&b.group).then(a.name.cmp(&b.name)));

            for server in sorted {
                if server.group != current_group {
                    current_group = server.group.clone();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&current_group)
                            .strong()
                            .size(14.0),
                    );
                    ui.separator();
                }

                let is_selected = self.selected_server.as_deref() == Some(&server.id);
                let row_height = if server.tags.is_empty() { 44.0 } else { 56.0 };
                let desired_size = egui::vec2(ui.available_width(), row_height);
                let (rect, response) = ui.allocate_at_least(
                    desired_size,
                    egui::Sense::click(),
                );

                if response.clicked() {
                    self.selected_server = Some(server.id.clone());
                }
                if response.double_clicked() {
                    self.pending_action = Some(Action::Connect(server.id.clone()));
                }

                if is_selected || response.hovered() {
                    let bg = if is_selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.hovered.bg_fill
                    };
                    ui.painter().rect_filled(rect, 4.0, bg);
                }

                let text_pos = rect.left_top() + egui::vec2(8.0, 4.0);

                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &server.name,
                    egui::FontId::proportional(14.0),
                    if is_selected {
                        Color32::WHITE
                    } else {
                        ui.style().visuals.text_color()
                    },
                );

                ui.painter().text(
                    text_pos + egui::vec2(0.0, 18.0),
                    egui::Align2::LEFT_TOP,
                    &server.display_address(),
                    egui::FontId::proportional(11.0),
                    Color32::GRAY,
                );

                if server.is_team {
                    ui.painter().text(
                        egui::pos2(rect.right() - 60.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        "TEAM",
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(100, 149, 237),
                    );
                }

                if !server.tags.is_empty() {
                    let tags_str = server.tags.join(", ");
                    ui.painter().text(
                        text_pos + egui::vec2(0.0, 32.0),
                        egui::Align2::LEFT_TOP,
                        &tags_str,
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(150, 150, 150),
                    );
                }

                response.context_menu(|ui| {
                    if ui.button("Connect").clicked() {
                        self.pending_action = Some(Action::Connect(server.id.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Edit").clicked() {
                        self.pending_action = Some(Action::Edit(server.id.clone()));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        self.confirm_delete = Some(server.id.clone());
                        ui.close_menu();
                    }
                });
            }
        });

        if let Some(ref id) = self.confirm_delete.clone() {
            let name = book
                .find(id)
                .map(|s| s.name.clone())
                .unwrap_or_default();

            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Delete server '{}'?", name));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            self.pending_action = Some(Action::Delete(id.clone()));
                            self.confirm_delete = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                    });
                });
        }

        if self.selected_server.is_some() {
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    if let Some(ref id) = self.selected_server {
                        self.pending_action = Some(Action::Connect(id.clone()));
                    }
                }
                if ui.button("Edit").clicked() {
                    if let Some(ref id) = self.selected_server {
                        self.pending_action = Some(Action::Edit(id.clone()));
                    }
                }
                if ui.button("Delete").clicked() {
                    if let Some(ref id) = self.selected_server {
                        self.confirm_delete = Some(id.clone());
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port_bare_host() {
        let (host, port) = AddressBookPanel::parse_host_port("myserver");
        assert_eq!(host, "myserver");
        assert_eq!(port, 5900);
    }

    #[test]
    fn test_parse_host_port_display_number() {
        let (host, port) = AddressBookPanel::parse_host_port("myserver:1");
        assert_eq!(host, "myserver");
        assert_eq!(port, 5901);
    }

    #[test]
    fn test_parse_host_port_display_zero() {
        let (host, port) = AddressBookPanel::parse_host_port("myserver:0");
        assert_eq!(host, "myserver");
        assert_eq!(port, 5900);
    }

    #[test]
    fn test_parse_host_port_explicit_port() {
        let (host, port) = AddressBookPanel::parse_host_port("myserver:5901");
        assert_eq!(host, "myserver");
        assert_eq!(port, 5901);
    }

    #[test]
    fn test_parse_host_port_high_port() {
        let (host, port) = AddressBookPanel::parse_host_port("10.0.0.1:9000");
        assert_eq!(host, "10.0.0.1");
        assert_eq!(port, 9000);
    }

    #[test]
    fn test_parse_host_port_whitespace() {
        let (host, port) = AddressBookPanel::parse_host_port("  myserver:2  ");
        assert_eq!(host, "myserver");
        assert_eq!(port, 5902);
    }

    #[test]
    fn test_parse_host_port_display_99() {
        let (host, port) = AddressBookPanel::parse_host_port("host:99");
        assert_eq!(host, "host");
        assert_eq!(port, 5999);
    }

    #[test]
    fn test_parse_host_port_port_100_is_literal() {
        let (host, port) = AddressBookPanel::parse_host_port("host:100");
        assert_eq!(host, "host");
        assert_eq!(port, 100);
    }
}
