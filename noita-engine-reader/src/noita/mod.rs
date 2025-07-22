use std::{borrow::Cow, collections::HashMap, io, marker::PhantomData, sync::Arc};

use convert_case::{Case, Casing};
use derive_more::{Debug, derive::Display};
use types::{
    ComponentBuffer, ComponentTypeManager, Entity, EntityManager, GameGlobal, GlobalStats,
    TagManager, TranslationManager, Vec2,
    cell_factory::{CellData, CellFactory},
    components::{Component, ComponentName, WorldStateComponent},
    platform::{FileDevice, PlatformWin},
};

use crate::memory::{MemoryStorage, Pod, ProcessRef, Ptr};

pub mod discovery;
pub mod rng;
pub mod types;

#[derive(Debug, Clone)]
pub struct Noita {
    proc: ProcessRef,
    g: NoitaGlobals,

    entity_tag_cache: HashMap<String, Option<usize>>,
    no_player_not_polied: bool,

    materials: Vec<String>,
    material_ui_names: Vec<String>,
    files: HashMap<String, Arc<[u8]>>,
}

#[derive(Debug, Default, Clone)]
pub struct NoitaGlobals {
    pub world_seed: Option<Ptr<u32>>,
    pub ng_count: Option<Ptr<u32>>,
    pub global_stats: Option<Ptr<GlobalStats>>,
    pub game_global: Option<Ptr<Ptr<GameGlobal>>>,
    pub entity_manager: Option<Ptr<Ptr<EntityManager>>>,
    pub entity_tag_manager: Option<Ptr<Ptr<TagManager>>>,
    pub component_type_manager: Option<Ptr<ComponentTypeManager>>,
    pub translation_manager: Option<Ptr<TranslationManager>>,
    pub platform: Option<Ptr<PlatformWin>>,
}

macro_rules! not_found {
    ($($args:tt)*) => {
        || ::std::io::Error::new(::std::io::ErrorKind::NotFound, format!($($args)*))
    };
}

macro_rules! read_ptr {
    ($self:ident.$ident:ident) => {
        $self
            .g
            .$ident
            .ok_or_else(not_found!(concat!("No ", stringify!($ident), " pointer")))?
            .read(&$self.proc)
    };
}

macro_rules! deep_read {
    ($self:ident.$ident:ident $(. $next:ident)*) => {{
        let thing = $self
            .g
            .$ident
            .ok_or_else(not_found!(concat!("No ", stringify!($ident), " pointer")))?
            .read(&$self.proc)?
            .read(&$self.proc);
        $(let thing = thing?.$next.read(&$self.proc);)*
        thing
    }};
    ($self:ident*.$ident:ident $(. $next:ident)*) => {{
        let thing = $self
            .g
            .$ident
            .ok_or_else(not_found!(concat!("No ", stringify!($ident), " pointer")))?
            .read(&$self.proc)?
            .read(&$self.proc);
        $(let thing = thing?.$next.read(&$self.proc);)*
        thing
    }};
}

pub trait TagRef {
    fn get_tag_index(&self, noita: &mut Noita) -> io::Result<Option<usize>>;
}

impl TagRef for &str {
    fn get_tag_index(&self, noita: &mut Noita) -> io::Result<Option<usize>> {
        noita.get_entity_tag_index(self)
    }
}

impl TagRef for usize {
    fn get_tag_index(&self, _: &mut Noita) -> io::Result<Option<usize>> {
        Ok(Some(*self))
    }
}

impl TagRef for Option<usize> {
    fn get_tag_index(&self, _: &mut Noita) -> io::Result<Option<usize>> {
        Ok(*self)
    }
}

#[derive(Debug)]
pub enum PlayerState {
    Normal,
    Polymorphed,
    Cessated,
}

impl Noita {
    pub fn new(proc: ProcessRef, g: NoitaGlobals) -> Self {
        Self {
            proc,
            g,
            entity_tag_cache: Default::default(),
            no_player_not_polied: Default::default(),
            materials: Default::default(),
            material_ui_names: Default::default(),
            files: Default::default(),
        }
    }

    pub const fn proc(&self) -> &ProcessRef {
        &self.proc
    }

    pub fn read_seed(&self) -> io::Result<Option<Seed>> {
        let world_seed = deep_read!(self.world_seed)?;
        if world_seed == 0 {
            return Ok(None);
        }
        Ok(Some(Seed {
            world_seed,
            ng_count: deep_read!(self.ng_count)?,
        }))
    }

    pub fn read_stats(&self) -> io::Result<GlobalStats> {
        read_ptr!(self.global_stats)
    }

    pub fn read_game_global(&self) -> io::Result<GameGlobal> {
        deep_read!(self.game_global)
    }

    #[track_caller]
    pub fn read_cell_factory(&self) -> io::Result<Option<CellFactory>> {
        let ptr = deep_read!(self.game_global)?.cell_factory;
        if ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(ptr.read(&self.proc)?))
    }

    pub fn read_translation_manager(&self) -> io::Result<TranslationManager> {
        read_ptr!(self.translation_manager)
    }

    pub fn read_platform(&self) -> io::Result<PlatformWin> {
        read_ptr!(self.platform)
    }

    pub fn get_file(&mut self, path: &str) -> io::Result<Arc<[u8]>> {
        if let Some(file) = self.files.get(path) {
            return Ok(file.clone());
        }

        let fs = self.read_platform()?.file_system.read(&self.proc)?;
        let devices = fs.devices.read(&self.proc)?;

        for device in devices {
            let Some(device) = FileDevice::get(&self.proc, device)? else {
                continue;
            };
            if let Some(file) = device.as_dyn().get_file(&self.proc, &fs, path)? {
                let file = Arc::<[u8]>::from(file);
                self.files.insert(path.to_owned(), file.clone());
                return Ok(file);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found in Noita fs: {path}"),
        ))
    }

    pub fn translations(&self) -> io::Result<CachedTranslations> {
        let manager = self.read_translation_manager()?;
        let lang_key_indices = manager.key_to_index.read(&self.proc)?;
        let current_lang_strings = manager
            .languages
            .read_at(manager.current_lang_idx, &self.proc)?
            .ok_or_else(not_found!("Current language not found"))?
            .strings
            .read_storage(&self.proc)?;
        Ok(CachedTranslations {
            lang_key_indices,
            current_lang_strings,
        })
    }

    // could also discover the static world state pointer
    pub fn get_world_state(&mut self) -> io::Result<Option<WorldStateComponent>> {
        let Some(world_state_idx) = self.get_entity_tag_index("world_state")? else {
            return Ok(None);
        };
        let Some(entity) = self.get_first_tagged_entity(world_state_idx)? else {
            return Ok(None);
        };
        self.component_store::<WorldStateComponent>()?.get(&entity)
    }

    pub fn get_player(&mut self) -> io::Result<Option<(Entity, PlayerState)>> {
        let Some(player_unit_idx) = self.get_entity_tag_index("player_unit")? else {
            // no player_unit means definitely no player
            return Ok(None);
        };

        if let Some(player) = self.get_first_tagged_entity(player_unit_idx)? {
            self.no_player_not_polied = false;
            return Ok(Some((player, PlayerState::Normal)));
        }

        // avoid repeatedly trying to look up the polymorphed_player tag if it wasn't created yet
        if self.no_player_not_polied {
            return Ok(None);
        }

        if let Some(e) = self.get_first_tagged_entity("polymorphed_player")? {
            return Ok(Some((e, PlayerState::Polymorphed)));
        }
        if let Some(e) = self.get_first_tagged_entity("polymorphed_cessation")? {
            return Ok(Some((e, PlayerState::Cessated)));
        }

        self.no_player_not_polied = true;
        Ok(None)
    }

    pub fn get_first_tagged_entity(&mut self, tag: impl TagRef) -> io::Result<Option<Entity>> {
        let entity_manager = deep_read!(self.entity_manager)?;

        let Some(tag_idx) = tag.get_tag_index(self)? else {
            return Ok(None);
        };
        let Some(bucket) = entity_manager.entity_buckets.get(tag_idx as u32) else {
            return Ok(None);
        };
        bucket
            .read(&self.proc)?
            .read(&self.proc)?
            .iter()
            .find(|e| !e.is_null())
            .map(|e| e.read(&self.proc))
            .transpose()
    }

    /// Can store the index and check entity bitset directly to avoid hashmap
    /// lookups
    pub fn get_entity_tag_index(&mut self, tag: &str) -> io::Result<Option<usize>> {
        let cache_entry = self.entity_tag_cache.get(tag).copied();
        if let Some(idx) = cache_entry.flatten() {
            return Ok(Some(idx));
        }

        let idx = deep_read!(self.entity_tag_manager)?
            .tag_indices
            .get(&self.proc, tag)?
            .map(|idx| idx as usize);

        self.entity_tag_cache.insert(tag.to_string(), idx);

        if let Some(index) = idx {
            tracing::debug!(tag, index, "Found entity tag");
        } else if cache_entry.is_none() {
            // ^ only log it once
            tracing::debug!(tag, "Did not find entity tag - doesn't exist yet?");
        }

        Ok(idx)
    }

    pub fn read_entity_manager(&self) -> io::Result<EntityManager> {
        deep_read!(self.entity_manager)
    }

    pub fn read_entity_tag_manager(&self) -> io::Result<TagManager> {
        deep_read!(self.entity_tag_manager)
    }

    pub fn has_tag(&mut self, entity: &Entity, tag: impl TagRef) -> io::Result<bool> {
        Ok(entity.tags[tag.get_tag_index(self)?])
    }

    pub fn read_materials(&mut self) -> io::Result<Vec<String>> {
        self.read_cell_factory()?.map_or(Ok(Vec::new()), |cf| {
            cf.material_ids.read_storage(&self.proc)
        })
    }

    pub fn read_cell_data(&mut self) -> io::Result<Vec<CellData>> {
        self.read_cell_factory()?.map_or(Ok(Vec::new()), |cf| {
            cf.cell_data
                .truncated(cf.number_of_materials)
                .read(&self.proc)
        })
    }

    pub fn materials(&mut self) -> io::Result<&[String]> {
        if self.materials.is_empty() {
            self.materials = self.read_materials()?;
        }
        Ok(&self.materials)
    }

    pub fn get_material_name(&mut self, index: u32) -> io::Result<Option<String>> {
        Ok(self.materials()?.get(index as usize).cloned())
    }

    pub fn get_material_ui_name(&mut self, index: u32) -> io::Result<Option<String>> {
        if !self.material_ui_names.is_empty() {
            return Ok(self.material_ui_names.get(index as usize).cloned());
        }

        let material_descs = self.read_cell_data()?;

        let mut material_ui_names = Vec::with_capacity(material_descs.len());
        for desc in material_descs {
            material_ui_names.push(desc.ui_name.read(&self.proc)?);
        }
        self.material_ui_names = material_ui_names;
        Ok(self.material_ui_names.get(index as usize).cloned())
    }

    pub fn read_component_type_manager(&self) -> io::Result<ComponentTypeManager> {
        read_ptr!(self.component_type_manager)
    }

    pub fn component_store<T: ComponentName>(&self) -> io::Result<ComponentStore<T>> {
        let index = read_ptr!(self.component_type_manager)?
            .component_indices
            .get(&self.proc, T::NAME)?
            .ok_or_else(not_found!(
                "Component type index not found for '{}'",
                T::NAME
            ))?;

        let buffer = deep_read!(self.entity_manager)?
            .component_buffers
            .get(index)
            .ok_or_else(not_found!(
                "Component buffer not found for index {index} ({})",
                T::NAME
            ))?
            .read(&self.proc)?;

        Ok(ComponentStore {
            proc: self.proc.clone(),
            buffer,
            _marker: PhantomData,
        })
    }

    pub fn get_camera_pos(&self) -> io::Result<Vec2> {
        Ok(deep_read!(self.game_global.camera)?.get_pos())
    }

    pub fn get_camera_bounds(&self) -> io::Result<[i32; 4]> {
        let bounds = deep_read!(self.game_global.camera.bounds)?;
        Ok([bounds.x, bounds.y, bounds.w, bounds.h])
    }
}

#[cfg(feature = "lookup")]
impl Noita {
    pub fn lookup(globals: NoitaGlobals) -> io::Result<Option<Self>> {
        use sysinfo::{ProcessesToUpdate, System};

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);

        let Some(process) = system
            .processes_by_exact_name("noita.exe".as_ref())
            .find(|p| p.thread_kind().is_none())
        else {
            return Ok(None);
        };

        let proc = ProcessRef::connect(process.pid().as_u32())?;
        Ok(Some(Self::new(proc, globals)))
    }
}

#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
#[display("{world_seed}+{ng_count}")]
pub struct Seed {
    pub world_seed: u32,
    pub ng_count: u32,
}

impl Seed {
    pub fn sum(&self) -> u32 {
        self.world_seed.wrapping_add(self.ng_count)
    }
}

#[derive(Debug)]
pub struct ComponentStore<T> {
    proc: ProcessRef,
    buffer: Ptr<ComponentBuffer>,
    _marker: PhantomData<T>,
}

impl<T> ComponentStore<T>
where
    T: ComponentName + Pod,
{
    pub fn get_full(&self, entity: &Entity) -> io::Result<Option<Component<T>>> {
        let buffer = self.buffer.read(&self.proc)?;

        let idx = buffer
            .indices
            .get(entity.comp_idx)
            .map(|i| i.read(&self.proc))
            .transpose()?
            .unwrap_or(buffer.default_index);

        let Some(ptr) = buffer.storage.get(idx.read(&self.proc)?) else {
            return Ok(None);
        };

        let ptr = ptr.read(&self.proc)?;
        // not sure it could be null, but just in case
        if ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(ptr.read::<Component<T>>(&self.proc)?))
    }

    pub fn get(&self, entity: &Entity) -> io::Result<Option<T>> {
        Ok(self.get_full(entity)?.map(|c| c.data))
    }
}

#[derive(Debug, Default)]
pub struct CachedTranslations {
    lang_key_indices: HashMap<String, u32>,
    current_lang_strings: Vec<String>,
}

impl CachedTranslations {
    pub fn is_empty(&self) -> bool {
        self.lang_key_indices.is_empty()
    }

    pub fn translate<'k>(&self, key: &'k str, title_case: bool) -> Cow<'k, str> {
        self.lang_key_indices
            .get(key)
            .and_then(|i| self.current_lang_strings.get(*i as usize))
            .map_or(Cow::Borrowed(key), |s| {
                Cow::Owned(if title_case {
                    s.to_case(Case::Title)
                } else {
                    (*s).clone()
                })
            })
    }
}
