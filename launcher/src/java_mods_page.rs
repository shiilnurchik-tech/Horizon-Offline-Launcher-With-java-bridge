//! Offline Java Bridge controls on the selected Horizon instance's Mods tab.
use super::*;
use horizon_launcher::java_mods::{self, JavaMod};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct State {
    home: Option<PathBuf>,
    mods: Vec<JavaMod>,
    fresh: bool,
    reading: bool,
    busy: bool,
    confirm: Option<String>,
    pub(super) note: Option<String>,
}

impl Launcher {
    /// Read JAR metadata off the UI thread. Do not show results for an instance switched away from.
    fn refresh_fabric_mods(&mut self, home: PathBuf, cx: &mut Context<Self>) {
        if self.java_mods_page.home.as_ref() != Some(&home) {
            self.java_mods_page = State { home: Some(home.clone()), ..State::default() };
        }
        if self.java_mods_page.fresh || self.java_mods_page.reading {
            return;
        }
        self.java_mods_page.reading = true;
        let task = cx.background_executor().spawn(async move {
            let mods = java_mods::list(&home);
            (home, mods)
        });
        cx.spawn(async move |this, cx| {
            let (home, mods) = task.await;
            let _ = this.update(cx, |this, cx| {
                let state = &mut this.java_mods_page;
                if state.home.as_ref() == Some(&home) {
                    state.mods = mods;
                    state.reading = false;
                    state.fresh = true;
                    cx.notify();
                }
            });
        }).detach();
    }

    /// Pick the Java Bridge program from the beta archive, then import it on a worker thread.
    fn choose_fabric_client(&mut self, cx: &mut Context<Self>) {
        if !self.can_import_build() { return; }
        let Some(instance) = self.selected_instance().filter(|i| i.java.is_none()).cloned() else { return };
        let base = self.version_of(&instance).filter(|tag| {
            backend::is_game_tag(tag) || horizon_launcher::nightly::date_of(tag).is_some()
        }).cloned();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true, directories: false, multiple: false, prompt: Some(tr("Import").into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = paths.await else { return };
            let Some(path) = paths.pop() else { return };
            let _ = this.update(cx, |this, cx| {
                if !this.can_import_build() { return; }
                this.importing_build = true;
                this.import_error = None;
                this.importing_java_for = Some(instance.id.clone());
                this.java_mods_page.note = None;
                let tx = this.tx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::Imported(build_import::import(&path, base.as_deref())));
                });
                cx.notify();
            });
        }).detach();
    }

    fn choose_fabric_mods(&mut self, cx: &mut Context<Self>) {
        let Some(home) = self.selected_instance().filter(|i| i.java.is_none()).map(|i| i.home.clone()) else { return };
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true, directories: false, multiple: true, prompt: Some(tr("Add").into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let _ = this.update(cx, |this, cx| {
                if this.client.is_some() || this.starting_client.is_some() || this.java_mods_page.busy { return; }
                this.java_mods_page.busy = true;
                let task = cx.background_executor().spawn(async move {
                    let mut added = 0;
                    let mut errors = Vec::new();
                    for path in paths {
                        match java_mods::add(&home, &path) {
                            Ok(_) => added += 1,
                            Err(e) => errors.push(e),
                        }
                    }
                    (home, added, errors)
                });
                cx.spawn(async move |this, cx| {
                    let (home, added, errors) = task.await;
                    let _ = this.update(cx, |this, cx| {
                        if this.java_mods_page.home.as_ref() == Some(&home) {
                            let state = &mut this.java_mods_page;
                            state.busy = false;
                            state.fresh = false;
                            state.note = Some(if errors.is_empty() {
                                trf("Added {count} Fabric mods.", &[("count", &added.to_string())])
                            } else { format!("{}\n{}", trf("Added {count} Fabric mods.", &[("count", &added.to_string())]), errors.join("\n")) });
                            cx.notify();
                        }
                    });
                }).detach();
                cx.notify();
            });
        }).detach();
    }

    fn remove_fabric_mod(&mut self, home: PathBuf, file: String, cx: &mut Context<Self>) {
        if self.client.is_some() || self.starting_client.is_some() || self.java_mods_page.busy { return; }
        self.java_mods_page.busy = true;
        let task = cx.background_executor().spawn(async move {
            let result = java_mods::remove(&home, &file);
            (home, result)
        });
        cx.spawn(async move |this, cx| {
            let (home, result) = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.java_mods_page.home.as_ref() == Some(&home) {
                    let state = &mut this.java_mods_page;
                    state.busy = false;
                    state.fresh = false;
                    state.confirm = None;
                    state.note = Some(match result {
                        Ok(m) => trf("{name} moved to the trash.", &[("name", m.title())]),
                        Err(e) => e,
                    });
                    cx.notify();
                }
            });
        }).detach();
    }

    /// The same panel works in the glass and classic layouts. Fabric is off until explicitly
    /// enabled, and the warning stays visible before importing or executing any code.
    pub(super) fn fabric_panel(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let instance = self.selected_instance()?.clone();
        if instance.java.is_some() { return None; }
        self.refresh_fabric_mods(instance.home.clone(), cx);
        let t = self.theme;
        let running = self.client.is_some() || self.starting_client.is_some();
        let busy = running || self.java_mods_page.busy || self.java_mods_page.reading;
        let imported = self.version_of(&instance).is_some_and(|tag| build_import::is_imported(tag));
        let id = instance.id.clone();
        let toggle = switch("fabric-toggle", instance.offline_fabric, &t);
        let toggle = if busy { toggle.opacity(0.4).cursor_default() } else {
            toggle.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                match instances::edit(&id, |m| m.offline_fabric = !instance.offline_fabric) {
                    Ok(_) => { this.reload_instances(); this.java_mods_page.note = None; }
                    Err(e) => this.java_mods_page.note = Some(e),
                }
                cx.notify();
            }))
        };
        let import = button("fabric-client-import", tr("Import Java Bridge client…"), Kind::Secondary, &t);
        let import = if busy || !self.can_import_build() { import.opacity(0.4).cursor_default() } else {
            import.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.choose_fabric_client(cx)))
        };
        let add = button("fabric-add", tr("Add Fabric mods…"), Kind::Secondary, &t);
        let add = if busy { add.opacity(0.4).cursor_default() } else {
            add.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.choose_fabric_mods(cx)))
        };
        let home = self.java_mods_page.home.clone().unwrap_or_default();
        let open = button("fabric-folder", tr("Open folder"), Kind::Secondary, &t).on_click(move |_, _, cx| {
            let dir = java_mods::dir(&home);
            if std::fs::create_dir_all(&dir).is_ok() { crate::glass_toast::open(cx, &dir); }
        });
        let refresh = button("fabric-refresh", tr("Refresh"), Kind::Secondary, &t).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.java_mods_page.fresh = false; cx.notify();
        }));
        let rows = self.java_mods_page.mods.clone().into_iter().map(|m| {
            let file = m.file.clone();
            let home = self.java_mods_page.home.clone().unwrap_or_default();
            let confirming = self.java_mods_page.confirm.as_ref() == Some(&file);
            let label = if confirming { tr("Click again to remove") } else { tr("Remove") };
            let remove = button(format!("fabric-remove-{file}"), label, Kind::Secondary, &t);
            let remove = if busy { remove.opacity(0.4).cursor_default() } else {
                remove.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if confirming { this.remove_fabric_mod(home.clone(), file.clone(), cx); }
                    else { this.java_mods_page.confirm = Some(file.clone()); }
                    cx.notify();
                }))
            };
            let title = format!("{} · {}", m.title(), m.file);
            let wrong = m.wrong_minecraft.map(|v| trf("Minecraft version required: {version}", &[("version", &v)]));
            div().flex().items_center().gap(px(12.)).child(div().flex_1().min_w_0().child(title).children(wrong.map(|s| div().child(s)))).child(remove)
        }).collect::<Vec<_>>();
        let status = if imported { tr("Imported client selected; make sure it is this beta's Java Bridge client.") } else { tr("Import the Java Bridge client from the beta archive before enabling Fabric.") };
        Some(plate(&t).p(px(16.)).flex().flex_col().gap(px(10.))
            .child(div().flex().items_center().gap(px(12.)).child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child(tr("Fabric mods (experimental)"))).child(toggle))
            .child(div().text_color(t.muted).child(tr("Offline Fabric does not verify ownership. Java mods can access your files and accounts. Use only trusted mods; back up worlds.")))
            .child(div().text_color(t.muted).child(tr("Install a Horizon version and prepare it with your Minecraft 26.3 files first. JDK 25 and a first-time internet connection may be needed.")))
            .child(div().text_color(t.muted).child(tr("Add exactly one Fabric API JAR for Minecraft 26.3 yourself. This beta client does not substitute another version.")))
            .child(div().text_color(t.muted).child(status))
            .child(div().flex().flex_wrap().gap(px(8.)).child(import).child(add).child(open).child(refresh))
            .children(self.java_mods_page.note.clone().map(|note| notice(note, &t)))
            .children((!self.java_mods_page.fresh).then(|| div().child(tr("Reading the mods…"))))
            .children(rows))
    }
}
