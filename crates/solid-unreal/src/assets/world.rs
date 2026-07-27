use std::io::{Seek, SeekFrom};

use glam::{Mat4, Quat, Vec3};

use crate::error::UnrealError;
use crate::reader;
use crate::uobject::property::PropertyReader;

#[derive(Debug, Clone)]
pub struct ActorInstance {
    pub name: String,
    pub class_name: String,
    pub transform: Mat4,
    pub components: Vec<ActorComponent>,
}

#[derive(Debug, Clone)]
pub enum ActorComponent {
    StaticMesh(StaticMeshComponent),
    SkeletalMesh(SkeletalMeshComponent),
    Light(LightComponent),
    Camera(CameraComponent),
    Unknown { name: String, class: String },
}

#[derive(Debug, Clone)]
pub struct StaticMeshComponent {
    pub name: String,
    pub static_mesh_export_idx: Option<usize>,
    pub transform: Mat4,
    pub materials: Vec<Option<usize>>,
}

#[derive(Debug, Clone)]
pub struct SkeletalMeshComponent {
    pub name: String,
    pub skeletal_mesh_export_idx: Option<usize>,
    pub transform: Mat4,
}

#[derive(Debug, Clone)]
pub struct LightComponent {
    pub name: String,
    pub light_type: LightType,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
    Rect,
}

#[derive(Debug, Clone)]
pub struct CameraComponent {
    pub name: String,
    pub fov: f32,
    pub aspect_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct LevelAsset {
    pub name: String,
    pub actors: Vec<ActorInstance>,
    pub referenced_exports: Vec<usize>,
}

pub fn read_world(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<LevelAsset, UnrealError> {
    let export = header.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_world",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let class_name = match export.class() {
        uasset::ObjectReference::Import { import_index } => {
            header.imports.get(import_index)
                .and_then(|i| header.resolve_name(&i.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::Export { export_index } => {
            header.exports.get(export_index)
                .and_then(|e| header.resolve_name(&e.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::None => String::new(),
    };

    let is_world = class_name == "World" || class_name == "WorldPartition"
        || class_name == "BlueprintGeneratedClass";

    let start_offset = export.serial_offset as u64;

    if is_world {
        header.archive.seek(SeekFrom::Start(start_offset))?;

        let pr = PropertyReader::new(&header.names);

        if !pr.find_property(&mut header.archive, "PersistentLevel")? {
            return Err(UnrealError::Conversion {
                asset_type: "World",
                detail: format!("world '{}' has no PersistentLevel", export_name),
            });
        }

        let level_ref = reader::read_package_index(&mut header.archive)?;

        if level_ref > 0 {
            let level_idx = (level_ref - 1) as usize;
            return read_world(header, level_idx);
        }

        return Err(UnrealError::Conversion {
            asset_type: "World",
            detail: "PersistentLevel is not an export in this package".into(),
        });
    }

    header.archive.seek(SeekFrom::Start(start_offset))?;
    let pr = PropertyReader::new(&header.names);

    if !pr.find_property(&mut header.archive, "Actors")? {
        return Err(UnrealError::Conversion {
            asset_type: "Level",
            detail: format!("level '{}' has no Actors array", export_name),
        });
    }

    let actor_count = reader::read_i32(&mut header.archive)? as usize;

    let mut actors = Vec::new();
    for _ in 0..actor_count {
        let actor_ref = reader::read_package_index(&mut header.archive)?;

        if let Some(actor) = try_read_actor(header, actor_ref)? {
            actors.push(actor);
        }
    }

    Ok(LevelAsset {
        name: export_name,
        actors,
        referenced_exports: Vec::new(),
    })
}

fn try_read_actor(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    actor_ref: i32,
) -> Result<Option<ActorInstance>, UnrealError> {
    if actor_ref <= 0 { return Ok(None); }

    let export_idx = (actor_ref - 1) as usize;
    let export = match header.exports.get(export_idx) {
        Some(e) => e,
        None => return Ok(None),
    };

    let class_name = match export.class() {
        uasset::ObjectReference::Import { import_index } => {
            header.imports.get(import_index)
                .and_then(|i| header.resolve_name(&i.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::Export { export_index } => {
            header.exports.get(export_index)
                .and_then(|e| header.resolve_name(&e.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::None => String::new(),
    };

    let mut actor_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let mut transform = Mat4::IDENTITY;
    let mut components = Vec::new();

    loop {
        let tag = {
            let pr = PropertyReader::new(&header.names);
            pr.read_tag(&mut header.archive)?
        };
        match tag {
            None => break,
            Some(tag) => {
                match tag.name.as_str() {
                    "RootComponent" => {
                        let comp_ref = reader::read_package_index(&mut header.archive)?;
                        let after_root_pos = match header.archive.stream_position() {
                            Ok(p) => p,
                            Err(_) => 0,
                        };

                        if comp_ref > 0 {
                            let comp_idx = (comp_ref - 1) as usize;
                            if let Some(result) = try_read_scene_component(header, comp_idx)? {
                                transform = result.transform;
                                components.push(result.component);
                            }
                        }

                        if after_root_pos > 0 {
                            header.archive.seek(SeekFrom::Start(after_root_pos))?;
                        }
                    }
                    "ActorLabel" => {
                        if tag.type_name == "StrProperty" {
                            let label = reader::read_fstring(&mut header.archive)?;
                            if !label.is_empty() {
                                actor_name = label;
                            }
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    "Tags" => {
                        let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                        header.archive.seek(SeekFrom::Start(skip_to))?;
                    }
                    _ => {
                        let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                        header.archive.seek(SeekFrom::Start(skip_to))?;
                    }
                }
            }
        }
    }

    Ok(Some(ActorInstance {
        name: actor_name,
        class_name,
        transform,
        components,
    }))
}

fn try_read_scene_component(
    header: &mut uasset::AssetHeader<std::io::Cursor<&[u8]>>,
    export_idx: usize,
) -> Result<Option<ComponentReadResult>, UnrealError> {
    let export = match header.exports.get(export_idx) {
        Some(e) => e,
        None => return Ok(None),
    };

    let class_name = match export.class() {
        uasset::ObjectReference::Import { import_index } => {
            header.imports.get(import_index)
                .and_then(|i| header.resolve_name(&i.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::Export { export_index } => {
            header.exports.get(export_index)
                .and_then(|e| header.resolve_name(&e.object_name).ok())
                .map(|c| c.to_string()).unwrap_or_default()
        }
        uasset::ObjectReference::None => String::new(),
    };

    let comp_name = header.resolve_name(&export.object_name)
        .unwrap_or_default().to_string();

    let start_offset = export.serial_offset as u64;
    header.archive.seek(SeekFrom::Start(start_offset))?;

    let mut relative_location = Vec3::ZERO;
    let mut relative_rotation = Vec3::ZERO;
    let mut relative_scale = Vec3::ONE;
    let mut component: Option<ActorComponent> = None;

    loop {
        let tag = {
            let pr = PropertyReader::new(&header.names);
            pr.read_tag(&mut header.archive)?
        };
        match tag {
            None => break,
            Some(tag) => {
                match tag.name.as_str() {
                    "RelativeLocation" => {
                        if tag.struct_name == "Vector" || tag.type_name == "Vector" {
                            let x = reader::read_f64(&mut header.archive)?;
                            let y = reader::read_f64(&mut header.archive)?;
                            let z = reader::read_f64(&mut header.archive)?;
                            relative_location = Vec3::new(x as f32, y as f32, z as f32);
                        } else if tag.struct_name == "Vector3f" || tag.type_name == "Vector3f" {
                            let x = reader::read_f32(&mut header.archive)?;
                            let y = reader::read_f32(&mut header.archive)?;
                            let z = reader::read_f32(&mut header.archive)?;
                            relative_location = Vec3::new(x, y, z);
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    "RelativeRotation" => {
                        if tag.struct_name == "Rotator" || tag.type_name == "Rotator" {
                            let pitch = reader::read_f64(&mut header.archive)?;
                            let yaw = reader::read_f64(&mut header.archive)?;
                            let roll = reader::read_f64(&mut header.archive)?;
                            relative_rotation = Vec3::new(pitch as f32, yaw as f32, roll as f32);
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    "RelativeScale3D" => {
                        if tag.struct_name == "Vector" || tag.type_name == "Vector" {
                            let x = reader::read_f64(&mut header.archive)?;
                            let y = reader::read_f64(&mut header.archive)?;
                            let z = reader::read_f64(&mut header.archive)?;
                            relative_scale = Vec3::new(x as f32, y as f32, z as f32);
                        } else if tag.struct_name == "Vector3f" || tag.type_name == "Vector3f" {
                            let x = reader::read_f32(&mut header.archive)?;
                            let y = reader::read_f32(&mut header.archive)?;
                            let z = reader::read_f32(&mut header.archive)?;
                            relative_scale = Vec3::new(x, y, z);
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    "StaticMesh" => {
                        let mesh_ref = reader::read_package_index(&mut header.archive)?;
                        if mesh_ref > 0 {
                            let mesh_idx = (mesh_ref - 1) as usize;
                            component = Some(ActorComponent::StaticMesh(StaticMeshComponent {
                                name: comp_name.clone(),
                                static_mesh_export_idx: Some(mesh_idx),
                                transform: Mat4::IDENTITY,
                                materials: Vec::new(),
                            }));
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    "SkeletalMesh" => {
                        let mesh_ref = reader::read_package_index(&mut header.archive)?;
                        if mesh_ref > 0 {
                            let mesh_idx = (mesh_ref - 1) as usize;
                            component = Some(ActorComponent::SkeletalMesh(SkeletalMeshComponent {
                                name: comp_name.clone(),
                                skeletal_mesh_export_idx: Some(mesh_idx),
                                transform: Mat4::IDENTITY,
                            }));
                        } else {
                            let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                            header.archive.seek(SeekFrom::Start(skip_to))?;
                        }
                    }
                    _ => {
                        let skip_to = tag.raw.next_offset + tag.raw.size as u64;
                        header.archive.seek(SeekFrom::Start(skip_to))?;
                    }
                }
            }
        }
    }

    let rot_rad = Vec3::new(
        relative_rotation.x.to_radians(),
        relative_rotation.y.to_radians(),
        relative_rotation.z.to_radians(),
    );
    let quat = Quat::from_euler(glam::EulerRot::ZYX, rot_rad.z, rot_rad.y, rot_rad.x);
    let transform = Mat4::from_scale_rotation_translation(relative_scale, quat, relative_location);

    let component = component.unwrap_or(ActorComponent::Unknown {
        name: comp_name,
        class: class_name,
    });

    Ok(Some(ComponentReadResult { transform, component }))
}

struct ComponentReadResult {
    transform: Mat4,
    component: ActorComponent,
}
