use std::io::SeekFrom;

use glam::{Mat4, Quat, Vec3};

use crate::archive::FArchiveUE;
use crate::error::UnrealError;
use crate::types::PackageIndex;
use crate::UPackage;

/// A placed actor instance in a level.
#[derive(Debug, Clone)]
pub struct ActorInstance {
    pub name: String,
    pub class_name: String,
    pub transform: Mat4,
    pub components: Vec<ActorComponent>,
}

/// A component attached to an actor.
#[derive(Debug, Clone)]
pub enum ActorComponent {
    StaticMesh(StaticMeshComponent),
    SkeletalMesh(SkeletalMeshComponent),
    Light(LightComponent),
    Camera(CameraComponent),
    Unknown { name: String, class: String },
}

/// A static mesh component (placed mesh in the level).
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

/// Parsed level (ULevel / UWorld) data.
#[derive(Debug, Clone)]
pub struct LevelAsset {
    pub name: String,
    pub actors: Vec<ActorInstance>,
    pub referenced_exports: Vec<usize>,
}

/// Read a UWorld or ULevel from a package export.
pub fn read_world(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<LevelAsset, UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "read_world",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let export_name = pkg.resolve_name(export.object_name);
    let class_name = pkg.resolve_export_class_name(export.class_index);
    let is_world = class_name == "World" || class_name == "WorldPartition"
        || class_name == "BlueprintGeneratedClass";

    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;

    if is_world {
        reader.seek(SeekFrom::Start(start_offset))?;

        reader.seek(SeekFrom::Start(start_offset))?;

        let archive = FArchiveUE::new(reader, pkg.version.clone());
        let mut pr = pkg.property_reader(archive);

        if !pr.find_property("PersistentLevel")? {
            return Err(UnrealError::Conversion {
                asset_type: "World",
                detail: format!("world '{}' has no PersistentLevel", export_name),
            });
        }

        let level_export = pr.archive().read_package_index()?;

        if level_export.is_export() {
            let level_idx = (level_export.0 - 1) as usize;
            return read_world(pkg, level_idx, reader);
        }

        return Err(UnrealError::Conversion {
            asset_type: "World",
            detail: "PersistentLevel is not an export in this package".into(),
        });
    }

    // ULevel: read properties to find Actors array
    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    if !pr.find_property("Actors")? {
        return Err(UnrealError::Conversion {
            asset_type: "Level",
            detail: format!("level '{}' has no Actors array", export_name),
        });
    }

    let actor_count = pr.archive().read_serial_size()? as usize;
    drop(pr); // release reader borrow

    let mut actors = Vec::new();
    for _ in 0..actor_count {
        let archive = FArchiveUE::new(reader, pkg.version.clone());
        let mut pr = pkg.property_reader(archive);

        let actor_ref = pr.archive().read_package_index()?;
        drop(pr); // release reader borrow

        if let Some(actor) = try_read_actor(pkg, actor_ref, reader)? {
            actors.push(actor);
        }
    }

    Ok(LevelAsset {
        name: export_name,
        actors,
        referenced_exports: Vec::new(),
    })
}

/// Try to read an actor from a package index reference.
fn try_read_actor(
    pkg: &UPackage,
    actor_ref: PackageIndex,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<Option<ActorInstance>, UnrealError> {
    if !actor_ref.is_export() {
        return Ok(None);
    }

    let export_idx = (actor_ref.0 - 1) as usize;
    let export = match pkg.exports.get(export_idx) {
        Some(e) => e,
        None => return Ok(None),
    };

    let class_name = pkg.resolve_export_class_name(export.class_index);
    let actor_name = pkg.resolve_name(export.object_name);

    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;
    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    let mut transform = Mat4::IDENTITY;
    let mut components = Vec::new();

    loop {
        match pr.read_tag()? {
            None => break,
            Some(tag) => {
                let ar = pr.archive();
                match tag.name.as_str() {
                    "RootComponent" => {
                        let comp_ref = ar.read_package_index()?;
                        drop(pr); // release reader borrow before recursion

                        if comp_ref.is_export() {
                            let comp_idx = (comp_ref.0 - 1) as usize;
                            if let Some(result) = try_read_scene_component(pkg, comp_idx, reader)? {
                                transform = result.transform;
                                components.push(result.component);
                            }
                        }

                        // Re-acquire borrow
                        reader.seek(SeekFrom::Start(start_offset))?;
                        let archive = FArchiveUE::new(reader, pkg.version.clone());
                        pr = pkg.property_reader(archive);

                        // Re-find RootComponent to update position
                        // Actually, we just need to continue the loop from the cached position
                        // For now, skip remaining properties
                        pr.skip_remaining_properties()?;
                        break;
                    }
                    _ => {
                        let data_start = ar.pos;
                        if tag.raw.next_offset > data_start {
                            ar.seek_to(tag.raw.next_offset)?;
                        }
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

/// Try to read a scene component (USceneComponent or subclass).
fn try_read_scene_component(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<Option<ComponentReadResult>, UnrealError> {
    let export = match pkg.exports.get(export_idx) {
        Some(e) => e,
        None => return Ok(None),
    };

    let class_name = pkg.resolve_export_class_name(export.class_index);
    let comp_name = pkg.resolve_name(export.object_name);

    let start_offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(SeekFrom::Start(start_offset))?;
    let archive = FArchiveUE::new(reader, pkg.version.clone());
    let mut pr = pkg.property_reader(archive);

    let mut relative_location = Vec3::ZERO;
    let mut relative_rotation = Vec3::ZERO;
    let mut relative_scale = Vec3::ONE;
    let mut component: Option<ActorComponent> = None;

    loop {
        match pr.read_tag()? {
            None => break,
            Some(tag) => {
                let ar = pr.archive();
                match tag.name.as_str() {
                    "RelativeLocation" => {
                        if tag.type_name == "Vector" {
                            let x = ar.read_f64()?;
                            let y = ar.read_f64()?;
                            let z = ar.read_f64()?;
                            relative_location = Vec3::new(x as f32, y as f32, z as f32);
                        } else if tag.type_name == "Vector3f" {
                            let x = ar.read_f32()?;
                            let y = ar.read_f32()?;
                            let z = ar.read_f32()?;
                            relative_location = Vec3::new(x, y, z);
                        }
                    }
                    "RelativeRotation" => {
                        if tag.struct_name == "Rotator" {
                            let pitch = ar.read_f64()?;
                            let yaw = ar.read_f64()?;
                            let roll = ar.read_f64()?;
                            relative_rotation = Vec3::new(pitch as f32, yaw as f32, roll as f32);
                        }
                    }
                    "RelativeScale3D" => {
                        if tag.struct_name == "Vector" {
                            let x = ar.read_f64()?;
                            let y = ar.read_f64()?;
                            let z = ar.read_f64()?;
                            relative_scale = Vec3::new(x as f32, y as f32, z as f32);
                        } else if tag.struct_name == "Vector3f" {
                            let x = ar.read_f32()?;
                            let y = ar.read_f32()?;
                            let z = ar.read_f32()?;
                            relative_scale = Vec3::new(x, y, z);
                        }
                    }
                    "StaticMesh" => {
                        let mesh_ref = ar.read_package_index()?;
                        if mesh_ref.is_export() {
                            let mesh_idx = (mesh_ref.0 - 1) as usize;
                            component = Some(ActorComponent::StaticMesh(StaticMeshComponent {
                                name: comp_name.clone(),
                                static_mesh_export_idx: Some(mesh_idx),
                                transform: Mat4::IDENTITY,
                                materials: Vec::new(),
                            }));
                        }
                    }
                    "SkeletalMesh" => {
                        let mesh_ref = ar.read_package_index()?;
                        if mesh_ref.is_export() {
                            let mesh_idx = (mesh_ref.0 - 1) as usize;
                            component = Some(ActorComponent::SkeletalMesh(SkeletalMeshComponent {
                                name: comp_name.clone(),
                                skeletal_mesh_export_idx: Some(mesh_idx),
                                transform: Mat4::IDENTITY,
                            }));
                        }
                    }
                    _ => {
                        let data_start = ar.pos;
                        if tag.raw.next_offset > data_start {
                            ar.seek_to(tag.raw.next_offset)?;
                        }
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


