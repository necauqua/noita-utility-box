use std::{collections::HashMap, io, marker::PhantomData};

use derive_more::{derive::Display, Debug};
use types::{
    components::{Component, ComponentName},
    ComponentBuffer, ComponentTypeManager, Entity, EntityManager, GameGlobal, GlobalStats,
    TagManager,
};

use crate::memory::{MemoryStorage, Pod, ProcessRef, Ptr};

pub mod discovery;
pub mod rng;
pub mod types;

#[derive(Debug, Clone)]
pub struct Noita {
    proc: ProcessRef,
    g: NoitaGlobals,
    entity_tag_cache: HashMap<String, u8>,
    no_player_not_polied: bool,

    materials: Vec<String>,
    material_ui_names: Vec<String>,
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
}

impl NoitaGlobals {
    pub fn debug() -> Self {
        Self {
            world_seed: Some(Ptr::of(0x1202fe4)),
            ng_count: Some(Ptr::of(0x1203004)),
            global_stats: Some(Ptr::of(0x1206920)),
            game_global: Some(Ptr::of(0x0122172c)),
            entity_manager: Some(Ptr::of(0x1202b78)),
            entity_tag_manager: Some(Ptr::of(0x1204fbc)),
            component_type_manager: Some(Ptr::of(0x01221c08)),
        }
    }
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
            .read(&$self.proc)?
    };
}

pub trait TagRef {
    fn get_tag_index(&self, noita: &mut Noita) -> io::Result<Option<u8>>;
}

impl TagRef for str {
    fn get_tag_index(&self, noita: &mut Noita) -> io::Result<Option<u8>> {
        noita.get_entity_tag_index(self)
    }
}

impl TagRef for u8 {
    fn get_tag_index(&self, _: &mut Noita) -> io::Result<Option<u8>> {
        Ok(Some(*self))
    }
}

impl TagRef for Option<u8> {
    fn get_tag_index(&self, _: &mut Noita) -> io::Result<Option<u8>> {
        Ok(*self)
    }
}

impl Noita {
    pub fn new(proc: ProcessRef, g: NoitaGlobals) -> Self {
        Self {
            proc,
            g,
            entity_tag_cache: HashMap::new(),
            no_player_not_polied: false,
            materials: Vec::new(),
            material_ui_names: Vec::new(),
        }
    }

    pub const fn proc(&self) -> &ProcessRef {
        &self.proc
    }

    pub fn read_seed(&self) -> io::Result<Option<Seed>> {
        let world_seed = read_ptr!(self.world_seed).read(&self.proc)?;
        if world_seed == 0 {
            return Ok(None);
        }
        Ok(Some(Seed {
            world_seed,
            ng_count: read_ptr!(self.ng_count).read(&self.proc)?,
        }))
    }

    pub fn read_stats(&self) -> io::Result<GlobalStats> {
        Ok(read_ptr!(self.global_stats))
    }

    pub fn get_player(&mut self) -> io::Result<Option<(Entity, bool)>> {
        let Some(player_unit_idx) = self.get_entity_tag_index("player_unit")? else {
            // no player_unit means definitely no player
            return Ok(None);
        };

        if let Some(player) = self.get_first_tagged_entity(player_unit_idx)? {
            self.no_player_not_polied = false;
            return Ok(Some((player, false)));
        }

        // avoid repeatedly trying to look up the polymorphed_player tag if it wasn't created yet
        if self.no_player_not_polied {
            return Ok(None);
        }

        let Some(polymorphed_player_idx) = self.get_entity_tag_index("polymorphed_player")? else {
            // no polymorphed_player means player was never polymorphed,
            // and without a player it means there's no player lol
            self.no_player_not_polied = true;
            return Ok(None);
        };
        Ok(self
            .get_first_tagged_entity(polymorphed_player_idx)?
            .map(|p| (p, true)))
    }

    pub fn get_first_tagged_entity(&mut self, tag: impl TagRef) -> io::Result<Option<Entity>> {
        let entity_manager = read_ptr!(self.entity_manager).read(&self.proc)?;

        let Some(tag_idx) = tag.get_tag_index(self)? else {
            return Ok(None);
        };
        let Some(bucket) = entity_manager.entity_buckets.get(tag_idx as u32) else {
            return Ok(None);
        };
        let Some(entity) = bucket.read(&self.proc)?.get(0) else {
            return Ok(None);
        };
        let entity = entity.read(&self.proc)?;
        if entity.is_null() {
            return Ok(None);
        }
        Ok(Some(entity.read(&self.proc)?))
    }

    /// Can store the index and check entity bitset directly to avoid hashmap
    /// lookups
    pub fn get_entity_tag_index(&mut self, tag: &str) -> io::Result<Option<u8>> {
        if let Some(idx) = self.entity_tag_cache.get(tag) {
            return Ok(Some(*idx));
        }

        let idx = read_ptr!(self.entity_tag_manager)
            .read(&self.proc)?
            .tag_indices
            .get(&self.proc, tag)?;

        if let Some(index) = idx {
            self.entity_tag_cache.insert(tag.to_string(), index);

            tracing::debug!("Found {tag} index: {index}");
        } else {
            // this can spam when the tag was never touched yet and thus doesn't exist
            tracing::trace!("Did not find {tag} index");
        }

        Ok(idx)
    }

    pub fn has_tag(&mut self, entity: &Entity, tag: impl TagRef) -> io::Result<bool> {
        Ok(entity.tags[tag.get_tag_index(self)?])
    }

    pub fn materials(&mut self) -> io::Result<&[String]> {
        if !self.materials.is_empty() {
            return Ok(&self.materials);
        }

        let material_ptrs = read_ptr!(self.game_global)
            .read(&self.proc)?
            .cell_factory
            .read(&self.proc)?
            .materials
            .read(&self.proc)?;

        let mut materials = Vec::with_capacity(material_ptrs.len());
        for ptr in material_ptrs {
            materials.push(ptr.read(&self.proc)?);
        }
        self.materials = materials;
        Ok(&self.materials)
    }

    pub fn get_material_name(&mut self, index: u32) -> io::Result<Option<String>> {
        Ok(self.materials()?.get(index as usize).cloned())
    }

    pub fn get_material_ui_name(&mut self, index: u32) -> io::Result<Option<String>> {
        if !self.material_ui_names.is_empty() {
            return Ok(self.material_ui_names.get(index as usize).cloned());
        }

        let cell_factory = read_ptr!(self.game_global)
            .read(&self.proc)?
            .cell_factory
            .read(&self.proc)?;
        let material_descs = cell_factory
            .material_descs_maybe
            .truncated(cell_factory.number_of_materials)
            .read(&self.proc)?;

        let mut material_ui_names = Vec::with_capacity(material_descs.len());
        for desc in material_descs {
            material_ui_names.push(desc.ui_name.read(&self.proc)?);
        }
        self.material_ui_names = material_ui_names;
        Ok(self.material_ui_names.get(index as usize).cloned())
    }

    pub fn component_store<T: ComponentName>(&self) -> io::Result<ComponentStore<T>> {
        let index = read_ptr!(self.component_type_manager)
            .component_indices
            .get(&self.proc, T::NAME)?
            .ok_or_else(not_found!("Component type index not found"))?;

        let buffer = read_ptr!(self.entity_manager)
            .read(&self.proc)?
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
}

#[derive(Display, Debug, Clone, Copy)]
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

#[cfg(test)]
#[ignore]
#[test]
fn test() -> anyhow::Result<()> {
    use anyhow::Context;
    use sysinfo::ProcessesToUpdate;
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env()?,
        )
        .init();

    let mut system = sysinfo::System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let noita_pid = system
        .processes_by_exact_name("noita.exe".as_ref())
        .find(|p| p.thread_kind().is_none())
        .context("Noita process not found")?;

    let proc = ProcessRef::connect(noita_pid.pid().as_u32())?;
    let noita = Noita::new(proc, NoitaGlobals::debug());

    println!("{:#?}", noita.read_stats()?);

    Ok(())
}
