use std::{
    cmp::Ordering,
    collections::{HashMap, hash_map::Entry},
    fmt::Write,
    fs::File,
    io::Read,
    iter,
    net::TcpStream,
    path::Path,
    sync::Arc,
    thread::JoinHandle,
    time::Instant,
};

use anyhow::Context as _;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use derive_more::Debug;
use eframe::egui::{Button, CollapsingHeader, Context, Grid, TextEdit, Ui};
use noita_engine_reader::{
    CachedTranslations, Noita, PlayerState,
    memory::MemoryStorage,
    types::{
        Entity, Vec2,
        cell_factory::CellData,
        components::{
            AbilityComponent, DamageModelComponent, GameEffectComponent, ItemActionComponent,
            ItemComponent, MaterialInventoryComponent, PotionComponent, UIIconComponent,
            WalletComponent,
        },
    },
};
use rfd::FileHandle;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};
use zip::ZipArchive;

use crate::{
    app::AppState,
    tools::{ComponentStoreExt, Result, Tool},
    util::{Promise, persist},
    widgets::JsonWidget,
};

mod data;

#[derive(Debug, SmartDefault, Serialize, Deserialize)]
#[serde(default)]
struct State {
    token: String,
    #[default("wss://onlywands.com/")]
    host: String,
    features: Features,
    was_connected: bool,
}

#[derive(Debug, SmartDefault)]
pub struct StreamerWands {
    state: State,
    username: Option<String>,

    cached_translations: Arc<CachedTranslations>,
    cached_cell_data: Vec<CellData>,

    #[default(Promise::Taken)]
    picked_file: Promise<Option<FileHandle>>,

    websocket: WebsocketState,

    #[default(Instant::now())]
    last_ping: Instant,
    #[default(Instant::now())]
    last_send: Instant,
    last_sent: String,
}

persist!(StreamerWands { state: State });

type ConnectionHandle = JoinHandle<tungstenite::Result<Box<WebSocket<MaybeTlsStream<TcpStream>>>>>;

#[derive(Debug, Default)]
enum WebsocketState {
    #[default]
    NotConnected,
    Connecting(#[debug(skip)] ConnectionHandle),
    Connected(Box<WebSocket<MaybeTlsStream<TcpStream>>>),
    Error(String),
}

impl StreamerWands {
    fn connect(&self) -> WebsocketState {
        if self.state.token.is_empty() {
            WebsocketState::Error("Token is empty".into())
        } else if self.state.host.is_empty() {
            WebsocketState::Error("Host is empty".into())
        } else if self.username.is_none() {
            WebsocketState::Error("Invalid token".into())
        } else {
            let url = format!("{}/{}", self.state.host, self.state.token);

            let handle = std::thread::spawn(|| {
                let (ws, _) = tungstenite::connect(url)?;
                Ok(Box::new(ws))
            });

            WebsocketState::Connecting(handle)
        }
    }

    fn poll_connecting(&mut self, handle: ConnectionHandle) -> WebsocketState {
        if handle.is_finished() {
            match handle.join() {
                Ok(Ok(stream)) => {
                    self.state.was_connected = true;
                    WebsocketState::Connected(stream)
                }
                Ok(Err(err)) => WebsocketState::Error(err.to_string()),
                Err(err) => WebsocketState::Error(match err.downcast() {
                    Ok(s) => *s,
                    Err(_) => "panic".into(),
                }),
            }
        } else {
            WebsocketState::Connecting(handle)
        }
    }
}

#[typetag::serde]
impl Tool for StreamerWands {
    fn tick(&mut self, _ctx: &Context, state: &mut AppState) {
        self.websocket = match std::mem::replace(&mut self.websocket, WebsocketState::NotConnected)
        {
            WebsocketState::NotConnected if self.state.was_connected && state.noita.is_some() => {
                self.connect()
            }
            WebsocketState::Connecting(handle) => self.poll_connecting(handle),
            WebsocketState::Error(e)
                if self.state.was_connected
                    && state.noita.is_some()
                    && self.last_send.elapsed().as_secs() >= 3 =>
            {
                tracing::error!(%e, "websocket error, trying to reconnect..");
                self.last_sent.clear();
                self.connect()
            }
            WebsocketState::Connected(mut stream) => {
                if self.last_ping.elapsed().as_secs() >= 5 {
                    if let Err(e) = stream.send(Message::Text("im alive".into())) {
                        tracing::error!(%e, "failed to send keepalive");
                        self.websocket = WebsocketState::Error(e.to_string());
                        return;
                    }
                    self.last_ping = Instant::now();
                    tracing::debug!("sent ping!");
                }

                if self.last_send.elapsed().as_secs() < 3 {
                    // ugh just reassign it back before every early return..
                    self.websocket = WebsocketState::Connected(stream);
                    return;
                }
                let Some(noita) = &mut state.noita else {
                    self.websocket = WebsocketState::Connected(stream);
                    return;
                };

                let payload = match Payload::read(self, noita).and_then(|p| {
                    Ok(p.map(|p| serde_json::to_string(&p))
                        .transpose()
                        .context("payload serialization")?)
                }) {
                    Ok(Some(payload)) => payload,
                    Ok(None) => {
                        self.websocket = WebsocketState::Connected(stream);
                        return;
                    }
                    Err(e) => {
                        tracing::error!(%e, "failed to read payload");
                        self.websocket = WebsocketState::Connected(stream);
                        return;
                    }
                };

                if payload == self.last_sent {
                    self.websocket = WebsocketState::Connected(stream);
                    return;
                }
                self.last_sent = payload.clone();

                if let Err(e) = stream.send(Message::Text(payload.into())) {
                    tracing::error!(%e, "failed to send the payload");
                    self.websocket = WebsocketState::Error(e.to_string());
                    return;
                }
                tracing::info!("sent payload!");
                self.last_send = Instant::now();

                WebsocketState::Connected(stream)
            }
            ws => ws,
        };
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        if let Some(noita) = &mut state.noita {
            let refresh = ui.button("Refresh").clicked();
            if refresh || self.cached_translations.is_empty() {
                self.cached_translations = Arc::new(
                    noita
                        .translations()
                        .context("Failed to read language data")?,
                );
            }
            if refresh || self.cached_cell_data.is_empty() {
                self.cached_cell_data =
                    noita.read_cell_data().context("Failed to read cell data")?;
            }
        } else {
            ui.label("Not connected to Noita");
        }

        ui.separator();

        let f = &mut self.state.features;
        ui.checkbox(&mut f.seed, "Send world seed");
        ui.checkbox(&mut f.ngp, "Send NG+ level");
        ui.checkbox(&mut f.pos, "Send player position");
        ui.checkbox(&mut f.shifts, "Send fungal shifts");
        ui.checkbox(&mut f.timer, "Send fungal shift timer");

        ui.separator();

        Grid::new("auth").num_columns(2).show(ui, |ui| {
            ui.label("Token");
            if ui
                .add(TextEdit::singleline(&mut self.state.token).password(true))
                .changed()
                || self.username.is_none()
            {
                self.username = get_username_from_token(&self.state.token);
            }
            ui.end_row();
            ui.label("Host");
            ui.text_edit_singleline(&mut self.state.host);
            ui.end_row();
        });

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            ui.label("Read the token and host from streamer-wands.zip: ");
            if ui
                .add_enabled(self.picked_file.is_taken(), Button::new("browse"))
                .clicked()
            {
                self.picked_file = Promise::spawn(rfd::AsyncFileDialog::new().pick_file());
            }
        });

        if !self.picked_file.is_taken()
            && let Some(Some(file)) = self.picked_file.poll_take()
        {
            (self.state.host, self.state.token) = read_token_and_host_from_mod(file.path())?;
            self.username = get_username_from_token(&self.state.token);
            // reconnect if needed
            self.websocket = WebsocketState::NotConnected;
            self.last_sent.clear();
        }

        if let Some(username) = &self.username {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;
                ui.label("Valid token for ");
                ui.hyperlink_to(username, format!("https://twitch.tv/{username}"));
            });
        }

        if state.noita.is_none() {
            self.websocket = WebsocketState::NotConnected;
            self.last_sent.clear();
            ui.label("Not connected to website");
            return Ok(());
        }

        self.websocket = match std::mem::replace(&mut self.websocket, WebsocketState::NotConnected)
        {
            WebsocketState::NotConnected => {
                if ui.button("Connect").clicked() || self.state.was_connected {
                    self.connect()
                } else {
                    WebsocketState::NotConnected
                }
            }
            WebsocketState::Connecting(handle) => {
                ui.label("Connecting...");
                self.poll_connecting(handle)
            }
            ws @ WebsocketState::Connected(_) => {
                if ui.button("Disconnect").clicked() {
                    self.state.was_connected = false;
                    self.last_sent.clear();
                    WebsocketState::NotConnected
                } else {
                    ws
                }
            }
            WebsocketState::Error(err) => {
                ui.label(format!("Error: {err}"));
                if ui.button("Retry").clicked() {
                    self.last_sent.clear();
                    WebsocketState::NotConnected
                } else {
                    WebsocketState::Error(err)
                }
            }
        };

        ui.separator();

        CollapsingHeader::new("Debug")
            .show(ui, |ui| {
                ui.label("Those are the values that were read from the game and are being sent to the onlywands server");
                ui.separator();
                if let Some(payload) = Payload::read(self, state.get_noita()?)? {
                    let json = serde_json::to_value(&payload).context("Payload serialization")?;
                    if ui.small_button("Copy JSON").clicked() {
                        ui.ctx().copy_text(json.to_string());
                    }
                    ui.add(JsonWidget::new(&json));
                } else {
                    ui.label("Player not found");
                }
                Result::Ok(())
            })
            .body_returned
            .transpose()?;

        Ok(())
    }
}

fn read_token_and_host_from_mod(path: &Path) -> Result<(String, String)> {
    let mut archive =
        ZipArchive::new(File::open(path)?).context("Opening the streamer wands archive")?;

    let mut buf = String::with_capacity(256);
    archive
        .by_name("streamer_wands/token.lua")
        .context("Reading token.lua")?
        .read_to_string(&mut buf)?;

    let token = buf
        .trim()
        .trim_start_matches("return")
        .trim_start()
        .trim_matches('"')
        .into();

    archive
        .by_name("streamer_wands/files/ws/host.lua")
        .context("Reading host.lua")?
        .read_to_string(&mut buf)?;

    let host = buf
        .lines()
        .find(|l| l.contains("HOST_URL"))
        .context("No HOST_URL found")?
        .trim()
        .trim_start_matches("HOST_URL =")
        .trim_end_matches(".. token")
        .trim()
        .trim_matches('"')
        .into();

    Ok((host, token))
}

fn get_username_from_token(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    parts.next(); // skip header

    let payload = BASE64_URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct JwtPayload {
        display_name: String,
    }

    let payload = serde_json::from_slice::<JwtPayload>(&payload).ok()?;
    Some(payload.display_name)
}

#[derive(Debug, Serialize, SmartDefault)]
#[serde(rename_all = "camelCase")]
struct Payload {
    wands: Vec<Wand>,
    inventory: Vec<String>,
    items: Vec<String>,
    progress: Progress,
    run_info: RunInfo,
    player_info: PlayerInfo,
    mod_features: Features,
    #[default("1.2.10")]
    mod_version: String,
}

fn clamp_potion_brightness(packed: u32) -> u32 {
    // just hardcode the magic number meh
    const RENDER_POTION_PARTICLE_MAX_COLOR_COMPONENT: f32 = 0.7;

    let [r, g, b, a] = packed.to_le_bytes();
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let a = a as f32 / 255.0;

    let brightest = r.max(g.max(b));

    if brightest <= RENDER_POTION_PARTICLE_MAX_COLOR_COMPONENT {
        return packed;
    }

    let scale = RENDER_POTION_PARTICLE_MAX_COLOR_COMPONENT / brightest * 255.0;

    u32::from_le_bytes([
        (r * scale) as u8,
        (g * scale) as u8,
        (b * scale) as u8,
        (a * scale) as u8,
    ])
}

fn read_inv_items(
    tool: &mut StreamerWands,
    noita: &mut Noita,
    player: &Entity,
) -> Result<Vec<String>> {
    let mut inventory = Vec::new();
    let Some(inv) = player.first_child_by_name("inventory_quick", noita.proc())? else {
        return Ok(inventory);
    };
    if inv.children.is_null() {
        return Ok(inventory);
    }

    let wand = noita.get_entity_tag_index("wand")?;
    let potion = noita.get_entity_tag_index("potion")?;
    let powder_stash = noita.get_entity_tag_index("powder_stash")?;

    let nonwands = inv
        .children
        .read(noita.proc())?
        .read_storage(noita.proc())?
        .into_iter()
        .filter(|c| !c.tags[wand])
        .collect::<Vec<_>>();

    let mut last_slot = 0;
    let beer = "data/items_gfx/beer_bottle.png";

    let potion_comp_store = noita.component_store::<PotionComponent>()?;
    let item_comp_store = noita.component_store::<ItemComponent>()?;
    let mat_inv_comp_store = noita.component_store::<MaterialInventoryComponent>()?;

    for child in nonwands {
        let comp = item_comp_store.get_checked(&child)?;
        let name = comp.item_name.read(noita.proc())?;
        let desc = comp.ui_description.read(noita.proc())?;
        let mut amt = "$-1".to_string();
        let spr = comp.ui_sprite.read(noita.proc())?;

        if child.tags[potion] || child.tags[powder_stash] || spr == beer {
            let mats = mat_inv_comp_store
                .get_checked(&child)?
                .count_per_material_type
                .read(noita.proc())?;

            // computing potion color
            let color_material = potion_comp_store
                .get(&child)?
                .map(|p| p.custom_color_material)
                .filter(|p| *p != 0)
                .unwrap_or_else(|| {
                    // find the majority
                    mats.iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Less))
                        .map(|(i, _)| i as _)
                        .unwrap_or_default()
                });

            let color = tool
                .cached_cell_data
                .get(color_material as usize)
                .map(|cd| clamp_potion_brightness(cd.graphics.color.0 | 0xff000000))
                .unwrap_or_default();

            amt.clear();
            write!(&mut amt, "${color}").unwrap();
            for (i, &mat) in mats.iter().enumerate() {
                if mat > 0.0 {
                    let mat_id = noita.get_material_name(i as _)?.unwrap_or_default();
                    let mat_key = noita
                        .get_material_ui_name(i as _)
                        .unwrap_or(None)
                        .unwrap_or_default();
                    let mat_name = tool
                        .cached_translations
                        .translate(mat_key.trim_start_matches('$'), true)
                        .unwrap_or_else(|| mat_key.to_owned());
                    write!(&mut amt, "@{mat_name} ({mat_id})#{mat}").unwrap();
                }
            }
        }
        let slot = comp.inventory_slot.x;
        let empty_slots = slot - last_slot;
        if empty_slots > 0 {
            for _ in 0..empty_slots {
                inventory.push("0".into());
                last_slot += 1;
            }
        }
        inventory.push(format!("{spr}{name}{desc}{amt}"));
        last_slot += 1;
    }

    Ok(inventory)
}

fn read_inv_spells(noita: &mut Noita, player: &Entity) -> Result<Vec<String>> {
    let mut inventory = Vec::new();

    let Some(inv) = player.first_child_by_name("inventory_full", noita.proc())? else {
        return Ok(inventory);
    };
    if inv.children.is_null() {
        return Ok(inventory);
    }
    let ics = noita.component_store::<ItemComponent>()?;
    let iacs = noita.component_store::<ItemActionComponent>()?;

    let mut last_slot = 0;

    for child in inv
        .children
        .read(noita.proc())?
        .read_storage(noita.proc())?
    {
        let Some(item_action_comp) = iacs.get(&child)? else {
            continue;
        };
        let Some(item_comp) = ics.get(&child)? else {
            continue;
        };
        let action_id = item_action_comp.action_id.read(noita.proc())?;
        let charges = item_comp.uses_remaining;
        let slot = item_comp.inventory_slot.x;
        let empty_slots = slot - last_slot;
        if empty_slots > 0 {
            for _ in 0..empty_slots {
                inventory.push("0".into());
                last_slot += 1;
            }
        }
        if action_id.is_empty() {
            inventory.push("sampo_#-1".into());
        } else {
            inventory.push(format!("{action_id}_#{charges}"));
        }
        last_slot += 1;
    }

    Ok(inventory)
}

impl Payload {
    fn read(tool: &mut StreamerWands, noita: &mut Noita) -> Result<Option<Self>> {
        let Some((player, PlayerState::Normal)) = noita.get_player()? else {
            return Ok(None);
        };
        Ok(Some(Self {
            wands: Wand::read_from_player(tool, noita, &player)?,
            inventory: read_inv_spells(noita, &player)?,
            items: read_inv_items(tool, noita, &player)?,
            progress: Progress::read(noita)?,
            run_info: RunInfo::read(tool, noita)?,
            player_info: PlayerInfo::read(tool, noita, &player)?,
            mod_features: tool.state.features.clone(),
            ..Default::default()
        }))
    }
}

#[derive(Debug, Serialize, SmartDefault)]
struct RunInfo {
    mods: Vec<String>,
    beta: bool,
    ngp: Option<u32>,
    seed: Option<u32>,
    #[default("1999,9,7,6,0,0")]
    start: String,
    playtime: f64,
}

impl RunInfo {
    fn read(tool: &mut StreamerWands, noita: &mut Noita) -> Result<Self> {
        let mut mods = vec![];
        for md in noita.read_mod_context()?.mods.read_storage(noita.proc())? {
            if !md.id.is_empty() || md.enabled1 != 0 || md.enabled2 != 0 {
                mods.push(md.id.read(noita.proc())?);
            }
        }
        let beta = noita
            .get_file("_branch.txt")
            .is_ok_and(|b| b.trim_ascii_end() != b"master");
        let seed = noita.read_seed()?;

        let playtime = noita.read_config_player_stats()?.stats.playtime;

        Ok(Self {
            mods,
            beta,
            ngp: seed.map(|s| s.ng_count).filter(|_| tool.state.features.ngp),
            seed: seed
                .map(|s| s.world_seed)
                .filter(|_| tool.state.features.seed),
            playtime,
            ..Default::default()
        })
    }
}

#[derive(Debug, Serialize, Default)]
struct Progress(Vec<String>, Vec<String>, Vec<String>, Vec<String>);

impl Progress {
    fn read(noita: &mut Noita) -> Result<Self> {
        let pfm = noita.read_persistent_flag_manager()?;
        let flags = pfm.read_flags(noita.proc())?;

        let perks = flags
            .iter()
            .filter_map(|f| f.strip_prefix("perk_picked_"))
            .map(|s| s.to_uppercase())
            .collect::<Vec<_>>();

        let spells = flags
            .iter()
            .filter_map(|f| f.strip_prefix("action_"))
            .map(|s| s.to_uppercase())
            .collect::<Vec<_>>();

        let kv_stats = noita.read_stats()?.key_value_stats.read(noita.proc())?;

        let enemies = String::from_utf8(
            noita
                .get_file("data/ui_gfx/animal_icons/_list.txt")?
                .to_vec(),
        )
        .context("_list.txt is not utf-8")?
        .lines()
        .chain(["boss_sky", "meatmaggot", "mimic_potion"]) // they forgot to add those 3 :(
        .filter(|enemy| kv_stats.get(*enemy).copied().unwrap_or_default() > 0)
        .map(|s| s.to_owned())
        .collect();

        let pillars = data::PILLARS
            .iter()
            .filter(|p| flags.contains(&p.to_string()))
            .map(|s| (*s).to_owned())
            .collect();

        Ok(Self(perks, spells, enemies, pillars))
    }
}

#[derive(Debug, Serialize, Default)]
struct Perks(Vec<String>, Vec<u32>);

impl Perks {
    fn read(noita: &mut Noita, player: &Entity) -> Result<Self> {
        if player.children.is_null() {
            return Ok(Self(vec![], vec![]));
        }
        let p = noita.proc().clone();
        let p = &p;

        let uis = noita.component_store::<UIIconComponent>()?;
        let ges = noita.component_store::<GameEffectComponent>()?;

        let tag_perk = noita.get_entity_tag_index("perk")?;
        let tag_essence = noita.get_entity_tag_index("perk")?;
        let tag_essence_effect = noita.get_entity_tag_index("perk")?;
        let tag_pseudo_perk = noita.get_entity_tag_index("pseudo_perk")?;
        let tag_greed_curse = noita.get_entity_tag_index("greed_curse")?;

        let mut perks = HashMap::<_, u32>::new();
        let mut order = vec![];

        for child in player.children.read(p)?.read_storage(p)? {
            let Some(ui_comp) = uis.get(&child)? else {
                continue;
            };

            let name = ui_comp.name.read(p)?;

            if child.tags[tag_perk]
                || child.tags[tag_essence]
                || child.tags[tag_essence_effect]
                || child.tags[tag_pseudo_perk]
                || child.tags[tag_greed_curse]
                || name.starts_with("$perk")
            {
                if ges.get(&child)?.map_or(0, |ge| ge.frames) != 0 {
                    continue;
                };
                // uggh ok fine lets support this part of apoth
                let name = if name == "$status_apotheosis_creature_shifted_name" {
                    let sprite = ui_comp.icon_sprite_file.read(p)?;
                    let last_slash = sprite.rfind('/').unwrap_or(0);
                    format!("{}_{}", name, &sprite[last_slash + 1..sprite.len() - 5])
                } else {
                    name
                };
                match perks.entry(name) {
                    Entry::Occupied(mut occupied) => *occupied.get_mut() += 1,
                    Entry::Vacant(vacant) => {
                        order.push(vacant.key().to_owned());
                        vacant.insert(1);
                    }
                }
            }
        }

        let amounts = order.iter().map(|perk| perks[perk]).collect::<Vec<_>>();

        Ok(Self(order, amounts))
    }
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PlayerInfo {
    perks: Perks,
    health: (String, String),
    gold: u64,
    orbs: u32,
    pos: Option<(f32, f32)>,
    shifts_total: u32,
    shifts_timer: Option<i32>,
    shifts_list: Option<Vec<String>>,
}

fn translate_material_by_original_id(translations: &CachedTranslations, mat: &str) -> String {
    translations
        .translate(&format!("material_{mat}"), true) // apoth
        .or_else(|| translations.translate(&format!("mat_{mat}"), true))
        .or_else(|| {
            data::MATERIAL_NAMES
                .get(mat)
                .and_then(|key| translations.translate(key, true))
        })
        .unwrap_or_else(|| mat.to_owned())
}

impl PlayerInfo {
    fn read(tool: &mut StreamerWands, noita: &mut Noita, player: &Entity) -> Result<Self> {
        let dmc = noita.component_store::<DamageModelComponent>()?;
        let wc = noita.component_store::<WalletComponent>()?;

        let dmc = dmc.get_checked(player)?;
        let Vec2 { x, y } = player.transform.pos;

        let ws = noita.get_world_state()?;

        let shifts_total = ws
            .as_ref()
            .map(|ws| ws.lua_globals.get(noita.proc(), "fungal_shift_iteration"))
            .transpose()?
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let shifts_list = ws
            .as_ref()
            .filter(|_| tool.state.features.shifts)
            .map(|ws| {
                let changed_materials = ws.changed_materials.read_storage(noita.proc())?;
                let shifts = FungalShift::from_changed_materials(changed_materials)
                    .into_iter()
                    .map(|shift| {
                        let mats = shift
                            .from
                            .iter()
                            .zip(iter::repeat(&shift.to))
                            .flat_map(|(from, to)| [from, to].into_iter());

                        let mut shift = String::new();
                        for mat in mats {
                            let mat_name =
                                translate_material_by_original_id(&tool.cached_translations, mat);
                            writeln!(&mut shift, "{mat}%@%{mat_name}<,>").unwrap();
                        }
                        // strip trailing <,> thingy
                        if !shift.is_empty() {
                            shift.truncate(shift.len() - 4);
                        }
                        shift
                    })
                    .collect();
                Result::Ok(shifts)
            })
            .transpose()?;

        let shifts_timer = ws
            .filter(|_| tool.state.features.timer)
            .map(|ws| {
                let last_trip = ws
                    .lua_globals
                    .get(noita.proc(), "fungal_shift_last_frame")?
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<i32>()
                    .unwrap_or(0);

                let current_frame = noita.read_game_global()?.frame_counter as i32;
                let mut shift_timer = (current_frame - last_trip) / 60;
                if shift_timer >= 300 || (current_frame < 300 * 60 && last_trip == 0) {
                    shift_timer = -1;
                }
                Result::Ok(shift_timer)
            })
            .transpose()?;

        fn lua_tostring(f: f64) -> String {
            if f.is_nan() {
                "nan".into()
            } else if f.is_infinite() {
                if f.is_sign_negative() {
                    "-inf".into()
                } else {
                    "inf".into()
                }
            } else {
                f.to_string()
            }
        }

        Ok(Self {
            perks: Perks::read(noita, player)?,
            health: (lua_tostring(dmc.hp.get()), lua_tostring(dmc.max_hp.get())),
            gold: wc.get_checked(player)?.money.get(),
            orbs: noita
                .get_world_state()?
                .map_or(0, |ws| ws.orbs_found_thisrun.len()),
            pos: Some((x, y)).filter(|_| tool.state.features.pos),
            shifts_total,
            shifts_list,
            shifts_timer,
        })
    }
}

#[derive(Debug, SmartDefault, Serialize, Deserialize, Clone)]
#[serde(default)]
struct Features {
    #[default(true)]
    seed: bool,
    #[default(true)]
    pos: bool,
    #[default(true)]
    ngp: bool,
    #[default(true)]
    shifts: bool,
    #[default(true)]
    timer: bool,
}

#[derive(Debug, Serialize)]
struct WandStats {
    sprite: String,
    ui_name: String,
    mana_max: f32,
    mana_charge_speed: f32,
    reload_time: i32,
    actions_per_round: i32,
    deck_capacity: i32,
    shuffle_deck_when_empty: bool,
    spread_degrees: f32,
    speed_multiplier: f32,
    fire_rate_wait: i32,
}

impl WandStats {
    fn read(tool: &mut StreamerWands, noita: &mut Noita, wand: &Entity) -> Result<WandStats> {
        let acs = noita.component_store::<AbilityComponent>()?;
        let ics = noita.component_store::<ItemComponent>()?;

        let ability_comp = acs.get_checked(wand)?;
        let item_comp = ics.get_checked(wand)?;

        Ok(WandStats {
            sprite: ability_comp.sprite_file.read(noita.proc())?,
            ui_name: if item_comp.always_use_item_name_in_ui.as_bool() {
                let ui_name = ability_comp.ui_name.read(noita.proc())?;
                let ui_name = ui_name.trim_start_matches('$');
                tool.cached_translations
                    .translate(ui_name, true)
                    .unwrap_or_else(|| ui_name.to_owned())
            } else {
                "wand".into()
            },
            mana_max: ability_comp.mana_max,
            mana_charge_speed: ability_comp.mana_charge_speed,
            reload_time: ability_comp.gun_config.reload_time,
            actions_per_round: ability_comp.gun_config.actions_per_round,
            deck_capacity: ability_comp.gun_config.deck_capacity,
            shuffle_deck_when_empty: ability_comp.gun_config.shuffle_deck_when_empty.as_bool(),
            spread_degrees: ability_comp.gunaction_config.spread_degrees,
            speed_multiplier: ability_comp.gunaction_config.speed_multiplier,
            fire_rate_wait: ability_comp.gunaction_config.fire_rate_wait,
        })
    }
}

#[derive(Debug, Serialize)]
struct Wand(WandStats, Vec<String>, Vec<String>);

impl Wand {
    fn read_from_player(
        tool: &mut StreamerWands,
        noita: &mut Noita,
        player: &Entity,
    ) -> Result<Vec<Self>> {
        let Some(inv_quick) = player.first_child_by_name("inventory_quick", noita.proc())? else {
            return Ok(vec![]);
        };
        if inv_quick.children.is_null() {
            return Ok(vec![]);
        }
        let wand = noita.get_entity_tag_index("wand")?;
        let mut wands = Vec::new();
        for child in inv_quick
            .children
            .read(noita.proc())?
            .read_storage(noita.proc())?
        {
            if child.tags[wand] {
                wands.push(Self::read(tool, noita, &child)?);
            }
        }
        Ok(wands)
    }

    fn read(tool: &mut StreamerWands, noita: &mut Noita, wand: &Entity) -> Result<Self> {
        let stats = WandStats::read(tool, noita, wand)?;
        let mut always_cast = Vec::new();
        let mut deck = Vec::new();

        if wand.children.is_null() {
            return Ok(Self(stats, vec![], vec![]));
        }

        let ics = noita.component_store::<ItemComponent>()?;
        let iacs = noita.component_store::<ItemActionComponent>()?;

        let childs = wand
            .children
            .read(noita.proc())?
            .read_storage(noita.proc())?;

        let mut last_slot = 0;
        for child in childs {
            let Some(item_comp) = ics.get(&child)? else {
                continue;
            };
            let Some(item_action_comp) = iacs.get(&child)? else {
                continue;
            };
            let spell = format!(
                "{}_#{}",
                item_action_comp.action_id.read(noita.proc())?,
                item_comp.uses_remaining
            );

            let slot = item_comp.inventory_slot.x;
            let empty_slots = slot - last_slot;
            if empty_slots > 0 {
                for _ in 0..empty_slots {
                    deck.push("0".into());
                    last_slot += 1;
                }
            }

            if item_comp.permanently_attached.as_bool() {
                always_cast.push(spell);
            } else {
                deck.push(spell);
                last_slot += 1;
            }
        }

        Ok(Self(stats, always_cast, deck))
    }
}

#[rustfmt::skip]
const SHIFT_GROUPS: &[&[&str]] = &[
    &["water", "water_static", "water_salt", "water_ice"],
    &["radioactive_liquid", "poison", "material_darkness"],
    &["oil", "swamp", "peat"],
    &["blood_fungi", "fungi", "fungisoil"],
    &["blood_cold", "blood_worm"],
    &["acid_gas", "acid_gas_static", "poison_gas", "fungal_gas", "radioactive_gas", "radioactive_gas_static"],
    &["magic_liquid_polymorph", "magic_liquid_unstable_polymorph"],
    &["magic_liquid_berserk", "magic_liquid_charm", "magic_liquid_invisibility"],
    &["silver", "brass", "copper"],
    &["steam", "smoke"],
    &["gold", "gold_box2d"],
];

#[derive(Debug, PartialEq, Eq)]
struct FungalShift {
    from: Vec<String>,
    to: String,
}

impl FungalShift {
    #[cfg(test)]
    fn of<const N: usize>(from: [&str; N], to: &str) -> Self {
        Self {
            from: from.iter().map(|s| (*s).into()).collect(),
            to: to.into(),
        }
    }

    fn from_changed_materials(materials: Vec<String>) -> Vec<Self> {
        let mut iter = materials.as_chunks::<2>().0.iter();

        let mut result = vec![];

        'outer: loop {
            // peek heh
            let Some([_, to]) = iter.clone().next() else {
                break;
            };
            for group in SHIFT_GROUPS {
                let group = group.iter().filter(|next_to| next_to != &to);
                if iter
                    .clone() // peek the following shifts without consuming
                    .chain(iter::repeat(&[String::new(), String::new()]))
                    .zip(group.clone())
                    .all(|([from, next_to], group_from)| next_to == to && from == group_from)
                {
                    let from = group.map(|&s| s.to_owned()).collect::<Vec<_>>();

                    // if everything matched consume it
                    iter.by_ref().take(from.len()).count();

                    result.push(Self {
                        from,
                        to: to.to_owned(),
                    });
                    continue 'outer;
                }
            }
            let Some([from, to]) = iter.next() else {
                break;
            };
            result.push(Self {
                from: vec![from.to_owned()],
                to: to.to_owned(),
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fungal_parsing() {
        #[rustfmt::skip]
        let changed_materials = [
            "lava", "acid",

            "oil",  "water", "swamp", "water", "peat", "water",

            "oil",  "water",
            "swamp", "water",
            "gold", "water",
        ];

        let shifts = FungalShift::from_changed_materials(
            changed_materials
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
        );

        assert_eq!(shifts[0], FungalShift::of(["lava"], "acid"));
        assert_eq!(
            shifts[1],
            FungalShift::of(["oil", "swamp", "peat"], "water")
        );
        assert_eq!(shifts[2], FungalShift::of(["oil"], "water"));
        assert_eq!(shifts[3], FungalShift::of(["swamp"], "water"));
        assert_eq!(shifts[4], FungalShift::of(["gold"], "water"));
        assert_eq!(shifts.len(), 5);
    }

    #[test]
    fn fungal_target_was_in_input_group() {
        #[rustfmt::skip]
        let changed_materials = [
            "blood_fungi", "fungi", "fungisoil", "fungi",

            "acid_gas", "lava",
            "acid_gas_static", "lava",
            "poison_gas", "lava",
            "fungal_gas", "lava",
            "radioactive_gas", "lava",
            "radioactive_gas_static", "lava",
        ];

        let shifts = FungalShift::from_changed_materials(
            changed_materials
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
        );

        assert_eq!(
            shifts[0],
            FungalShift::of(["blood_fungi", "fungisoil"], "fungi")
        );
        assert_eq!(
            shifts[1],
            FungalShift::of(
                [
                    "acid_gas",
                    "acid_gas_static",
                    "poison_gas",
                    "fungal_gas",
                    "radioactive_gas",
                    "radioactive_gas_static"
                ],
                "lava"
            )
        );
        assert_eq!(shifts.len(), 2);
    }
}
