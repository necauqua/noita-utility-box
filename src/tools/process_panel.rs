use derive_more::Debug;
use eframe::egui::{
    text::LayoutJob, ComboBox, Context, Grid, Hyperlink, RichText, TextFormat, TextStyle, Ui,
};
use noita_utility_box::{
    memory::{
        exe_image::{PeHeader, ReadImageError},
        ProcessRef,
    },
    noita::Noita,
};
use smart_default::SmartDefault;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use thiserror::Error;

use crate::{app::AppState, util::persist};

use super::{Result, Tool};

#[derive(Debug)]
pub struct NoitaData {
    pid: sysinfo::Pid,
    exe_name: Option<String>,
    timestamp: u32,

    noita: Noita,
}

#[derive(Error, Debug)]
enum NoitaError {
    #[error("Not Noita\nExport exe name: {name:?}, not wizard_physics.exe")]
    NotNoita { name: String },
    #[error("Unmapped Noita version (timestamp 0x{:x})", header.timestamp())]
    Unmapped {
        #[debug(skip)]
        proc: ProcessRef,
        header: PeHeader,
    },
    #[error(transparent)]
    BadProcess(#[from] ReadImageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

type NoitaResult<T> = std::result::Result<T, NoitaError>;

impl NoitaData {
    fn connect(pid: sysinfo::Pid, exe_name: Option<String>, state: &AppState) -> NoitaResult<Self> {
        let proc = ProcessRef::connect(pid.as_u32())?;
        let header = PeHeader::read(&proc)?;

        if state.settings.check_export_name {
            let export_name = header.export_name();
            if export_name != b"wizard_physics.exe\0" {
                let name = String::from_utf8_lossy(&export_name[..export_name.len() - 1]);
                return Err(NoitaError::NotNoita { name: name.into() });
            }
        }

        let timestamp = header.timestamp();

        let Some(address_map) = state.address_maps.get(timestamp) else {
            return Err(NoitaError::Unmapped { proc, header });
        };

        let noita = Noita::new(proc, address_map.as_noita_globals());

        Ok(Self {
            pid,
            exe_name,
            timestamp,
            noita,
        })
    }
}

#[derive(Debug, SmartDefault)]
pub struct ProcessPanel {
    #[default(true)]
    look_for_noita: bool,

    #[default(System::new())]
    system_info: System,

    #[default(Ok(None))]
    noita: NoitaResult<Option<NoitaData>>,
    selected_process: Option<(sysinfo::Pid, Option<String>)>,
}

persist!(ProcessPanel {
    look_for_noita: bool,
});

#[typetag::serde]
impl Tool for ProcessPanel {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        self.ui(ui, state);
        Ok(())
    }

    fn tick(&mut self, ctx: &Context, state: &mut AppState) {
        self.update(ctx, state);
    }
}

impl ProcessPanel {
    fn set_noita(
        &mut self,
        ctx: &Context,
        state: &mut AppState,
        noita: NoitaResult<Option<NoitaData>>,
    ) {
        // update the global handle to be used by things
        if let Ok(Some(ref data)) = noita {
            state.noita = Some(data.noita.clone());
        } else {
            state.noita = None;
        }
        self.noita = noita;
        self.selected_process = None;
        ctx.request_repaint();
    }

    fn processes_box(&mut self, ui: &mut Ui, state: &mut AppState) {
        let mut combo = ComboBox::from_id_salt("processes").height(400.0);

        if let Some((pid, exe)) = &self.selected_process {
            combo = combo.selected_text(process_label(ui, *pid, exe.as_deref()));
        } else {
            combo = combo.selected_text(RichText::new("Select process").italics());
        }

        let response = combo.show_ui(ui, |ui| {
            let mut processes = self
                .system_info
                .processes()
                .iter()
                .filter(|(_, p)| p.thread_kind().is_none())
                .collect::<Vec<_>>();

            processes.sort_unstable_by_key(|(pid, _)| *pid);

            for (pid, p) in processes {
                let exe = p
                    .exe()
                    .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()));
                let label = process_label(ui, *pid, exe.as_deref());
                ui.selectable_value(&mut self.selected_process, Some((*pid, exe)), label);
            }
        });

        if response.response.clicked() {
            self.system_info.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::new().with_exe(UpdateKind::OnlyIfNotSet),
            );
        }

        if let Some((pid, exe)) = self.selected_process.clone() {
            self.set_noita(
                ui.ctx(),
                state,
                NoitaData::connect(pid, exe, state).map(Some),
            );
        }
    }

    pub fn update(&mut self, ctx: &Context, state: &mut AppState) {
        let Ok(noita) = &self.noita else {
            return;
        };
        if noita.is_none() && !self.look_for_noita {
            return;
        }

        // Has to be all because either we don't have noita and we're looking
        // for it or we have it, but we want to check if it's still there, for
        // which refresh all is required
        self.system_info.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new().with_exe(UpdateKind::OnlyIfNotSet),
        );

        if let Some(noita) = noita {
            // check that we still have it
            if self.system_info.process(noita.pid).is_none() {
                self.set_noita(ctx, state, Ok(None));
                return;
            }

            state.seed = noita.noita.read_seed().ok().flatten();

            return;
        }

        // no noita and we're looking for it

        let Some(p) = self
            .system_info
            .processes_by_exact_name("noita.exe".as_ref())
            .find(|p| p.thread_kind().is_none())
        else {
            return;
        };
        let exe = p
            .exe()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()));

        self.set_noita(
            ctx,
            state,
            NoitaData::connect(p.pid(), exe, state).map(Some),
        );
    }

    pub fn ui(&mut self, ui: &mut Ui, state: &mut AppState) {
        match &self.noita {
            Err(e) => {
                ui.label(RichText::new(e.to_string()).color(ui.style().visuals.error_fg_color));

                if let NoitaError::Unmapped { proc, header } = e {
                    if ui.button("Run auto-discovery").clicked() {
                        if let Err(e) = state.address_maps.discover(proc, header) {
                            self.set_noita(ui.ctx(), state, Err(e.into()))
                        } else {
                            self.set_noita(ui.ctx(), state, Ok(None))
                        }
                    }
                    if !self.look_for_noita {
                        self.processes_box(ui, state);
                    }
                } else if self.look_for_noita {
                    self.set_noita(ui.ctx(), state, Ok(None));
                } else {
                    self.processes_box(ui, state);
                }
            }
            Ok(None) => {
                if self.look_for_noita {
                    ui.label("Noita process not found");
                } else {
                    self.processes_box(ui, state);
                }
            }
            Ok(Some(noita)) => {
                Grid::new("noita").show(ui, |ui| {
                    ui.label("Process:");
                    ui.label(process_label(ui, noita.pid, noita.exe_name.as_deref()));
                    ui.end_row();

                    ui.label("Version:");
                    ui.label(format!("0x{:x}", noita.timestamp));
                    ui.end_row();

                    if let Some(s) = &state.seed {
                        ui.label("Seed:");
                        let seed = s.world_seed.to_string();
                        let link = format!("https://noitool.com/info?seed={seed}");

                        ui.add(Hyperlink::from_label_and_url(seed, link).open_in_new_tab(true))
                            .on_hover_text("Open the seed in noitool");

                        ui.end_row();

                        ui.label("NG+ count:");
                        ui.label(s.ng_count.to_string());
                        ui.end_row();
                    }
                });

                if !self.look_for_noita && ui.button("Disconnect").clicked() {
                    self.set_noita(ui.ctx(), state, Ok(None));
                }
            }
        }

        ui.checkbox(&mut self.look_for_noita, "Auto-detect Noita process");
    }
}

fn process_label(ui: &Ui, pid: sysinfo::Pid, fname: Option<&str>) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        &pid.to_string(),
        0.0,
        TextFormat {
            font_id: TextStyle::Monospace.resolve(ui.style()),
            ..Default::default()
        },
    );
    if let Some(name) = fname {
        job.append(": ", 0.0, TextFormat::default());
        job.append(
            name,
            0.0,
            TextFormat {
                italics: true,
                ..Default::default()
            },
        )
    }
    job
}
