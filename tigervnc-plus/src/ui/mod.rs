mod address_book_panel;
mod connection_dialog;
mod password_dialog;
mod session_view;
mod sync_dialog;

use crate::address_book::AddressBook;
use crate::bridge;
use crate::connection::ConnectionManager;
use crate::renderer::{CursorData, FrameBuffer};
use crate::sync::TeamSyncManager;
use egui::{CentralPanel, Context, TopBottomPanel, Ui, ViewportBuilder, ViewportId};
use log::{debug, info, warn};

pub use address_book_panel::AddressBookPanel;
pub use connection_dialog::ConnectionDialog;
pub use session_view::SessionView;

struct VncSession {
    #[allow(dead_code)]
    id: String,
    name: String,
    viewport_id: ViewportId,
    connection: ConnectionManager,
    session_view: SessionView,
    closed: bool,
}

pub struct App {
    address_book: AddressBook,
    address_book_panel: AddressBookPanel,
    connection_dialog: Option<ConnectionDialog>,
    password_dialog: Option<password_dialog::PasswordDialog>,
    sessions: Vec<VncSession>,
    team_sync: TeamSyncManager,
    show_about: bool,
    show_sync_dialog: bool,
    sync_url_input: String,
    next_viewport_idx: u32,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let address_book = AddressBook::load();
        let team_sync = TeamSyncManager::new();
        let sync_url = team_sync.config().url.clone();

        info!(
            "Loaded address book: {} servers in {} groups",
            address_book.servers.len(),
            address_book.groups.len()
        );
        for server in &address_book.servers {
            debug!(
                "  Server '{}': {}:{} (group='{}', team={})",
                server.name, server.host, server.port, server.group, server.is_team
            );
        }
        debug!(
            "Team sync enabled={}, url='{}'",
            team_sync.config().enabled,
            team_sync.config().url
        );

        Self {
            address_book,
            address_book_panel: AddressBookPanel::new(),
            connection_dialog: None,
            password_dialog: None,
            sessions: Vec::new(),
            team_sync,
            show_about: false,
            show_sync_dialog: false,
            sync_url_input: sync_url,
            next_viewport_idx: 1,
        }
    }

    fn start_connection(&mut self, server_id: &str, host: &str, port: u16, username: &str, name: &str) {
        let port = crate::address_book::resolve_port(port);

        let viewport_id = ViewportId::from_hash_of(format!("vnc_session_{}", self.next_viewport_idx));
        self.next_viewport_idx += 1;

        let mut connection = ConnectionManager::new();
        connection.connect(host, port, username);

        let session = VncSession {
            id: server_id.to_string(),
            name: name.to_string(),
            viewport_id,
            connection,
            session_view: SessionView::new(),
            closed: false,
        };

        info!("Opening session '{}' in new viewport", name);
        self.sessions.push(session);
    }

    fn render_menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Connection...").clicked() {
                    self.connection_dialog = Some(ConnectionDialog::new(None));
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Import Servers...").clicked() {
                    ui.close_menu();
                }
                if ui.button("Export Servers...").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Team", |ui| {
                if ui.button("Sync Now").clicked() {
                    self.team_sync.trigger_sync();
                    ui.close_menu();
                }
                if ui.button("Configure Sync URL...").clicked() {
                    self.show_sync_dialog = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About TigerVNC Plus").clicked() {
                    self.show_about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn render_about_dialog(&mut self, ctx: &Context) {
        if !self.show_about {
            return;
        }
        egui::Window::new("About TigerVNC Plus")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("TigerVNC Plus");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.label("Modern UI for the TigerVNC protocol engine.");
                    ui.label("Built on TigerVNC v1.16.80 common/ libraries.");
                    ui.add_space(8.0);
                    ui.label("Licensed under GPL-2.0-or-later");
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
            });
    }

    fn render_sync_dialog(&mut self, ctx: &Context) {
        if !self.show_sync_dialog {
            return;
        }
        egui::Window::new("Team Sync Configuration")
            .collapsible(false)
            .resizable(false)
            .default_width(450.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Enter the URL of your team's server list (JSON):");
                ui.add_space(4.0);
                ui.text_edit_singleline(&mut self.sync_url_input);
                ui.add_space(4.0);
                ui.label("Supports: HTTPS URL, GitHub Gist raw URL, S3 presigned URL");
                ui.add_space(8.0);

                if self.team_sync.is_syncing() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Syncing...");
                    });
                }

                if let Some(err) = self.team_sync.last_error() {
                    ui.colored_label(egui::Color32::RED, format!("Last error: {}", err));
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.team_sync.set_url(self.sync_url_input.clone());
                        self.show_sync_dialog = false;
                    }
                    if ui.button("Save & Sync Now").clicked() {
                        self.team_sync.set_url(self.sync_url_input.clone());
                        self.team_sync.trigger_sync();
                        self.show_sync_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.sync_url_input = self.team_sync.config().url.clone();
                        self.show_sync_dialog = false;
                    }
                });
            });
    }

    fn system_username() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default()
    }

    fn handle_address_book_action(&mut self) {
        if let Some(action) = self.address_book_panel.take_action() {
            match action {
                address_book_panel::Action::Connect(id) => {
                    if let Some(entry) = self.address_book.find(&id) {
                        let entry = entry.clone();
                        info!("Connecting to {} ({})", entry.name, entry.display_address());
                        self.address_book.mark_connected(&id);
                        self.start_connection(
                            &id,
                            &entry.host,
                            entry.port,
                            &entry.username,
                            &entry.name,
                        );
                    }
                }
                address_book_panel::Action::QuickConnect(host, port) => {
                    let found = self
                        .address_book
                        .servers
                        .iter()
                        .find(|s| s.host == host && s.port == port)
                        .map(|s| (s.id.clone(), s.username.clone(), s.name.clone()));

                    if let Some((id, username, name)) = found {
                        info!(
                            "Quick-connect: found existing entry '{}' for {}:{}",
                            name, host, port
                        );
                        self.address_book.mark_connected(&id);
                        self.start_connection(&id, &host, port, &username, &name);
                    } else {
                        let username = Self::system_username();
                        let display = if port == 5900 {
                            host.clone()
                        } else {
                            format!("{}:{}", host, port)
                        };
                        info!(
                            "Quick-connect: new connection to {} (user='{}')",
                            display, username
                        );
                        let entry = crate::address_book::ServerEntry {
                            name: display.clone(),
                            host: host.clone(),
                            port,
                            username: username.clone(),
                            ..Default::default()
                        };
                        let id = entry.id.clone();
                        self.address_book.add(entry);
                        self.address_book.mark_connected(&id);
                        self.start_connection(&id, &host, port, &username, &display);
                    }
                }
                address_book_panel::Action::Edit(id) => {
                    if let Some(entry) = self.address_book.find(&id) {
                        self.connection_dialog =
                            Some(ConnectionDialog::new(Some(entry.clone())));
                    }
                }
                address_book_panel::Action::Delete(id) => {
                    self.address_book.remove(&id);
                }
                address_book_panel::Action::Add => {
                    self.connection_dialog = Some(ConnectionDialog::new(None));
                }
            }
        }
    }

    fn handle_connection_dialog(&mut self) {
        let mut close_dialog = false;
        let mut connect_entry_id: Option<String> = None;

        if let Some(ref mut dialog) = self.connection_dialog {
            if let Some(entry) = dialog.take_result() {
                let id = entry.id.clone();
                let wants_connect = dialog.wants_connect();

                if self.address_book.find(&entry.id).is_some() {
                    self.address_book.update(entry);
                } else {
                    self.address_book.add(entry);
                }

                if wants_connect {
                    connect_entry_id = Some(id);
                }
                close_dialog = true;
            }
            if dialog.is_cancelled() {
                close_dialog = true;
            }
        }
        if close_dialog {
            self.connection_dialog = None;
        }

        if let Some(id) = connect_entry_id {
            if let Some(entry) = self.address_book.find(&id) {
                let entry = entry.clone();
                info!("Connecting to {} ({})", entry.name, entry.display_address());
                self.address_book.mark_connected(&id);
                self.start_connection(
                    &id,
                    &entry.host,
                    entry.port,
                    &entry.username,
                    &entry.name,
                );
            }
        }
    }

    fn handle_password_dialog(&mut self) {
        // Check if the bridge needs credentials
        if self.password_dialog.is_none() {
            if let Some(req) = bridge::poll_credential_request() {
                info!(
                    "Showing password dialog for {}:{} user='{}'",
                    req.host, req.port, req.username
                );
                self.password_dialog = Some(password_dialog::PasswordDialog::new(
                    req.host,
                    req.port,
                    req.username,
                ));
            }
        }

        let mut close = false;
        if let Some(ref mut dialog) = self.password_dialog {
            if let Some(result) = dialog.take_result() {
                info!("Password provided for {}:{}", result.host, result.port);
                if result.save {
                    crate::credentials::store_password(
                        &result.host,
                        result.port,
                        &result.username,
                        &result.password,
                    )
                    .ok();
                }
                bridge::provide_credentials(result.username, result.password);
                close = true;
            }
            if dialog.is_cancelled() {
                info!("Password dialog cancelled");
                bridge::cancel_credentials();
                close = true;
            }
        }
        if close {
            self.password_dialog = None;
        }
    }

    fn handle_team_sync(&mut self) {
        if let Some(team_servers) = self.team_sync.take_result() {
            info!("Team sync: received {} servers", team_servers.len());
            self.address_book.merge_team_servers(team_servers);
        }
    }

    fn poll_bridge_events_for_sessions(&mut self) {
        if let Some((width, height)) = bridge::poll_init_done() {
            info!("Bridge: init done {}x{}", width, height);
            if let Some(session) = self.sessions.last_mut() {
                session
                    .session_view
                    .set_framebuffer(FrameBuffer::new(width as u32, height as u32));
            }
        }

        let damage_rects = bridge::poll_damage_rects();
        if !damage_rects.is_empty() {
            if let Some(session) = self.sessions.last_mut() {
                let (fb_w, fb_h, fb_stride, fb_data, _gen) =
                    session.connection.framebuffer_snapshot();
                if !fb_data.is_empty() && fb_w > 0 && fb_h > 0 {
                    session.session_view.set_framebuffer(FrameBuffer {
                        width: fb_w,
                        height: fb_h,
                        stride: fb_stride,
                        data: fb_data,
                    });
                    for rect in &damage_rects {
                        session
                            .session_view
                            .update_region(rect.x, rect.y, rect.w, rect.h);
                    }
                }
            }
        }

        if let Some((w, h, hx, hy, data)) = bridge::poll_cursor_data() {
            if let Some(session) = self.sessions.last_mut() {
                session.session_view.set_cursor(CursorData {
                    width: w as u32,
                    height: h as u32,
                    hotspot_x: hx,
                    hotspot_y: hy,
                    pixels: data,
                });
            }
        }

        if let Some(text) = bridge::poll_clipboard() {
            debug!("Received clipboard from server: {} bytes", text.len());
            let _ = text;
        }

        if let Some(err) = bridge::poll_error() {
            warn!("Bridge error: {}", err);
        }
    }

    fn forward_session_input(session: &mut VncSession) {
        for key_event in session.session_view.take_pending_keys() {
            if key_event.pressed {
                session
                    .connection
                    .send_key_press(key_event.key_code, key_event.key_sym);
            } else {
                session.connection.send_key_release(key_event.key_code);
            }
        }

        if let Some(ptr) = session.session_view.take_pending_pointer() {
            session
                .connection
                .send_pointer(ptr.x, ptr.y, ptr.button_mask);
        }

        if let Some(text) = session.session_view.take_pending_clipboard() {
            session.connection.send_clipboard(&text);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_team_sync();
        self.handle_connection_dialog();
        self.handle_address_book_action();
        self.handle_password_dialog();
        self.poll_bridge_events_for_sessions();

        // Show password dialog on main window
        if let Some(ref mut dialog) = self.password_dialog {
            dialog.show(ctx);
        }

        if let Some(ref mut dialog) = self.connection_dialog {
            dialog.show(ctx, &self.address_book.groups);
        }

        self.render_about_dialog(ctx);
        self.render_sync_dialog(ctx);

        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu_bar(ui);
        });

        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let count = self.address_book.servers.len();
                ui.label(format!(
                    "{} server{}",
                    count,
                    if count == 1 { "" } else { "s" }
                ));
                ui.separator();
                let active = self.sessions.len();
                if active > 0 {
                    ui.label(format!(
                        "{} active session{}",
                        active,
                        if active == 1 { "" } else { "s" }
                    ));
                } else {
                    ui.label("Ready");
                }
            });
        });

        // Main window always shows the address book
        CentralPanel::default().show(ctx, |ui| {
            self.address_book_panel.show(ui, &self.address_book);
        });

        // Render each VNC session in its own viewport (OS window)
        let mut session_indices_to_render: Vec<usize> = (0..self.sessions.len()).collect();
        for idx in session_indices_to_render.drain(..) {
            let session = &mut self.sessions[idx];
            if session.closed {
                continue;
            }

            // Check connection state
            if session.connection.has_error() {
                warn!("Session '{}' has error, closing viewport", session.name);
                session.closed = true;
                continue;
            }

            let viewport_id = session.viewport_id;
            let title = format!("{} - TigerVNC Plus", session.name);

            ctx.show_viewport_immediate(
                viewport_id,
                ViewportBuilder::default()
                    .with_title(title)
                    .with_inner_size([1024.0, 768.0]),
                |ctx, _class| {
                    let session = &mut self.sessions[idx];

                    Self::forward_session_input(session);

                    TopBottomPanel::top(format!("session_menu_{}", idx)).show(ctx, |ui| {
                        egui::menu::bar(ui, |ui| {
                            ui.menu_button("Connection", |ui| {
                                if ui.button("Disconnect").clicked() {
                                    session.connection.disconnect();
                                    session.closed = true;
                                    ui.close_menu();
                                }
                            });
                        });
                    });

                    CentralPanel::default().show(ctx, |ui| {
                        if session.connection.is_connected() {
                            session.session_view.show(ui);
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(100.0);
                                ui.spinner();
                                ui.label("Connecting...");
                                if ui.button("Cancel").clicked() {
                                    session.connection.disconnect();
                                    session.closed = true;
                                }
                            });
                        }
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        session.connection.disconnect();
                        session.closed = true;
                    }
                },
            );
        }

        // Remove closed sessions
        self.sessions.retain(|s| !s.closed);

        // Keep repainting while sessions are active or password dialog is open
        if !self.sessions.is_empty() || self.password_dialog.is_some() {
            ctx.request_repaint();
        }
    }
}
