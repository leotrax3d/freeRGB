//! Session-opt-in global shortcuts. No key recording; only registered shortcut IDs.
use super::{Command, Snapshot};
use eframe::egui;
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use serde_json::json;
use std::collections::BTreeSet;
use std::io::Read;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    function_key: u8,
    alt: bool,
    shift: bool,
    macro_name: String,
}

fn validate(bindings: &[Binding]) -> Result<(), String> {
    if bindings.len() > 16 {
        return Err("at most 16 hotkeys".into());
    }
    let mut seen = BTreeSet::new();
    for binding in bindings {
        if !(6..=10).contains(&binding.function_key) || !(binding.alt || binding.shift) {
            return Err(
                "use Ctrl plus Alt and/or Shift with F6–F10; F11 is reserved for cancel".into(),
            );
        }
        if binding.macro_name.is_empty()
            || binding.macro_name.chars().count() > 64
            || binding.macro_name.trim() != binding.macro_name
            || !binding
                .macro_name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        {
            return Err("select a saved macro".into());
        }
        if !seen.insert((binding.function_key, binding.alt, binding.shift)) {
            return Err("duplicate hotkey".into());
        }
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn apply_plan<K: Copy>(
    keys: &[K],
    mut apply: impl FnMut(K, bool) -> Result<(), String>,
) -> Result<(), String> {
    for (index, key) in keys.iter().enumerate() {
        if let Err(error) = apply(*key, true) {
            let mut errors = vec![error];
            for previous in keys[..index].iter().rev() {
                if let Err(error) = apply(*previous, false) {
                    errors.push(error);
                }
            }
            return Err(errors.join("; "));
        }
    }
    Ok(())
}

pub struct Hotkeys {
    bindings: Vec<Binding>,
    enabled: bool,
    error: Option<String>,
    #[cfg(windows)]
    native: Option<native::Registration>,
    #[cfg(windows)]
    events: Option<std::sync::mpsc::Receiver<global_hotkey::GlobalHotKeyEvent>>,
    #[cfg(windows)]
    last_trigger: std::collections::BTreeMap<u32, std::time::Instant>,
}

impl Default for Hotkeys {
    fn default() -> Self {
        let mut instance = Self {
            bindings: Vec::new(),
            enabled: false,
            error: None,
            #[cfg(windows)]
            native: None,
            #[cfg(windows)]
            events: None,
            #[cfg(windows)]
            last_trigger: Default::default(),
        };
        if let Some(path) = super::pages::config_root().map(|p| p.join("hotkeys.json")) {
            match std::fs::File::open(path) {
                Ok(file) => {
                    let mut text = String::new();
                    let result = file
                        .take(64 * 1024 + 1)
                        .read_to_string(&mut text)
                        .map_err(|e| e.to_string())
                        .and_then(|_| {
                            if text.len() > 64 * 1024 {
                                return Err("hotkey configuration too large".into());
                            }
                            let bindings: Vec<Binding> =
                                serde_json::from_str(&text).map_err(|e| e.to_string())?;
                            validate(&bindings)?;
                            Ok(bindings)
                        });
                    match result {
                        Ok(bindings) => instance.bindings = bindings,
                        Err(error) => instance.error = Some(error),
                    }
                }
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    instance.error = Some(error.to_string())
                }
                _ => {}
            }
        }
        instance
    }
}

impl Hotkeys {
    fn disable(&mut self) {
        #[cfg(windows)]
        {
            self.native = None;
            self.last_trigger.clear();
            if let Some(events) = &self.events {
                while events.try_recv().is_ok() {}
            }
        }
        self.enabled = false;
    }
    fn enable(&mut self, ctx: &egui::Context, snapshot: &Snapshot) -> Result<(), String> {
        validate(&self.bindings)?;
        if !snapshot.connected {
            return Err("daemon is not connected".into());
        }
        for binding in &self.bindings {
            if snapshot.control["macros"]
                .get(&binding.macro_name)
                .is_none()
            {
                return Err(format!(
                    "saved macro {:?} no longer exists",
                    binding.macro_name
                ));
            }
        }
        #[cfg(windows)]
        {
            if self.events.is_none() {
                let (sender, receiver) = std::sync::mpsc::sync_channel(64);
                let ctx = ctx.clone();
                global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |event| {
                    let _ = sender.try_send(event);
                    ctx.request_repaint();
                }));
                self.events = Some(receiver);
            }
            self.disable();
            self.native = Some(native::Registration::new(&self.bindings)?);
            self.enabled = true;
            self.error = None;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = ctx;
            Err("global shortcuts currently support Windows only".into())
        }
    }
    pub fn poll(&mut self, tx: &std::sync::mpsc::Sender<Command>, snapshot: &Snapshot) {
        if self.enabled && !snapshot.connected {
            self.disable();
            self.error = Some("Daemon disconnected; re-enable shortcuts after reconnecting".into());
        }
        #[cfg(windows)]
        {
            let events: Vec<_> = self
                .events
                .as_ref()
                .map(|events| events.try_iter().take(64).collect())
                .unwrap_or_default();
            for event in events {
                if !self.enabled || event.state != global_hotkey::HotKeyState::Pressed {
                    continue;
                }
                let now = std::time::Instant::now();
                if self.last_trigger.get(&event.id).is_some_and(|last| {
                    now.duration_since(*last) < std::time::Duration::from_millis(500)
                }) {
                    continue;
                }
                let action = self
                    .native
                    .as_ref()
                    .and_then(|native| native.actions.get(&event.id))
                    .cloned();
                let Some(action) = action else {
                    continue;
                };
                self.last_trigger.insert(event.id, now);
                match action {
                    native::Action::Cancel => {
                        let _ = tx.send(Command {
                            method: "macros.cancel",
                            params: json!({}),
                        });
                    }
                    native::Action::Run(name) => {
                        if snapshot.control["macros"].get(&name).is_none() {
                            self.disable();
                            self.error = Some("A bound macro was deleted; review bindings".into());
                            break;
                        }
                        if !snapshot.control["macro_status"]["running"].is_null() {
                            continue;
                        }
                        let _ = tx.send(Command {
                            method: "macros.run",
                            params: json!({"name":name}),
                        });
                    }
                }
            }
        }
        #[cfg(not(windows))]
        let _ = tx;
    }
    pub fn active(&self) -> bool {
        self.enabled
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, snapshot: &Snapshot) {
        egui::CollapsingHeader::new("Global macro shortcuts").show(ui,|ui| {
            ui.label("Off at every launch. Active only while this GUI is running. Ctrl+Shift+F11 cancels the running macro.");
            let mut enabled=self.enabled;
            if ui.add_enabled(cfg!(windows)&&snapshot.connected,egui::Checkbox::new(&mut enabled,"Enable global macro shortcuts for this session")).changed() {
                if enabled { if let Err(error)=self.enable(ui.ctx(),snapshot) { self.error=Some(error); } } else { self.disable(); }
            }
            if !cfg!(windows) { ui.label("Global shortcuts are currently Windows-only."); }
            if self.enabled { ui.colored_label(egui::Color32::LIGHT_GREEN,"GLOBAL SHORTCUTS ACTIVE"); }
            ui.add_enabled_ui(!self.enabled,|ui| {
                let mut remove=None;
                for (index,binding) in self.bindings.iter_mut().enumerate() {
                    ui.push_id(index,|ui| {ui.horizontal(|ui| {
                        ui.label("Ctrl +");ui.checkbox(&mut binding.alt,"Alt");ui.checkbox(&mut binding.shift,"Shift");
                        egui::ComboBox::from_id_salt("function").selected_text(format!("F{}",binding.function_key)).show_ui(ui,|ui| {for key in 6..=10 {ui.selectable_value(&mut binding.function_key,key,format!("F{key}"));}});
                        egui::ComboBox::from_id_salt("macro").selected_text(&binding.macro_name).show_ui(ui,|ui| {if let Some(macros)=snapshot.control["macros"].as_object() {for name in macros.keys() {ui.selectable_value(&mut binding.macro_name,name.clone(),name);}}});
                        if ui.small_button("Remove").clicked() {remove=Some(index);}
                    });});
                }
                if let Some(index)=remove {self.bindings.remove(index);}
                if ui.add_enabled(self.bindings.len()<16,egui::Button::new("Add shortcut")).clicked() {self.bindings.push(Binding{function_key:8,alt:false,shift:true,macro_name:String::new()});}
                if ui.button("Save shortcut bindings").clicked() {
                    let result=validate(&self.bindings).and_then(|()| {
                        let path=super::pages::config_root().ok_or("configuration directory unavailable")?.join("hotkeys.json");
                        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e|e.to_string())?;
                        let temporary=path.with_extension("json.tmp");
                        std::fs::write(&temporary,serde_json::to_vec_pretty(&self.bindings).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                        std::fs::rename(temporary,path).map_err(|e|e.to_string())
                    });self.error=result.err();
                }
            });
            if let Some(error)=&self.error {ui.colored_label(egui::Color32::LIGHT_RED,error);}
        });
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use global_hotkey::{
        hotkey::{Code, HotKey, Modifiers},
        GlobalHotKeyManager,
    };
    #[derive(Clone)]
    pub enum Action {
        Run(String),
        Cancel,
    }
    pub struct Registration {
        manager: GlobalHotKeyManager,
        keys: Vec<HotKey>,
        pub actions: std::collections::BTreeMap<u32, Action>,
    }
    impl Registration {
        pub fn new(bindings: &[Binding]) -> Result<Self, String> {
            let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
            let mut keys = Vec::new();
            let mut actions = std::collections::BTreeMap::new();
            for binding in bindings {
                let mut modifiers = Modifiers::CONTROL;
                if binding.alt {
                    modifiers |= Modifiers::ALT;
                }
                if binding.shift {
                    modifiers |= Modifiers::SHIFT;
                }
                let code = match binding.function_key {
                    6 => Code::F6,
                    7 => Code::F7,
                    8 => Code::F8,
                    9 => Code::F9,
                    10 => Code::F10,
                    _ => return Err("invalid function key".into()),
                };
                let key = HotKey::new(Some(modifiers), code);
                actions.insert(key.id(), Action::Run(binding.macro_name.clone()));
                keys.push(key);
            }
            let cancel = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F11);
            actions.insert(cancel.id(), Action::Cancel);
            keys.push(cancel);
            apply_plan(&keys, |key, enable| {
                if enable {
                    manager.register(key)
                } else {
                    manager.unregister(key)
                }
                .map_err(|e| e.to_string())
            })?;
            Ok(Self {
                manager,
                keys,
                actions,
            })
        }
    }
    impl Drop for Registration {
        fn drop(&mut self) {
            for key in &self.keys {
                let _ = self.manager.unregister(*key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_and_reserved_shortcuts_are_rejected() {
        let binding = Binding {
            function_key: 8,
            alt: false,
            shift: true,
            macro_name: "Test".into(),
        };
        assert!(validate(std::slice::from_ref(&binding)).is_ok());
        assert!(validate(&[binding.clone(), binding]).is_err());
        assert!(validate(&[Binding {
            function_key: 11,
            alt: false,
            shift: true,
            macro_name: "Test".into()
        }])
        .is_err());
    }
    #[test]
    fn partial_registration_is_rolled_back() {
        let mut calls = Vec::new();
        let result = apply_plan(&[1, 2, 3], |key, enable| {
            calls.push((key, enable));
            if key == 2 && enable {
                Err("busy".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert_eq!(calls, vec![(1, true), (2, true), (1, false)]);
    }
}
