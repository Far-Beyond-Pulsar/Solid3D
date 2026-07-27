use std::collections::{HashMap, HashSet};

use glam::{Mat4, Vec4};

use crate::assets::material::MaterialAsset;
use crate::assets::static_mesh::{StaticMeshAsset, UnrealVertex};
use crate::assets::texture::Texture2DAsset;
use crate::assets::world::{ActorComponent, LevelAsset};
use crate::error::UnrealError;
use crate::UPackage;

/// Configuration for the UE → Solid3D conversion.
#[derive(Debug, Clone)]
pub struct UnrealConvertConfig {
    /// Merge all meshes into a single mesh (default: true).
    pub merge_meshes: bool,
    /// Embed textures as PNG data in the scene (default: true).
    pub embed_textures: bool,
    /// Maximum texture dimension (0 = no limit).
    pub max_texture_size: u32,
    /// Flatten the node hierarchy (default: true).
    pub flatten_hierarchy: bool,
    /// Generate normals for meshes that don't have them.
    pub generate_normals: bool,
    /// Triangulate any non-triangle polygons.
    pub triangulate: bool,
}

impl Default for UnrealConvertConfig {
    fn default() -> Self {
        Self {
            merge_meshes: true,
            embed_textures: true,
            max_texture_size: 2048,
            flatten_hierarchy: true,
            generate_normals: true,
            triangulate: true,
        }
    }
}

/// Convert a fully parsed `UPackage` to a Solid3D `Scene`.
///
/// This is the main conversion entry point.
pub fn package_to_scene(
    pkg: &UPackage,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
    config: &UnrealConvertConfig,
) -> Result<solid_rs::scene::Scene, UnrealError> {
    use solid_rs::builder::SceneBuilder;
    use solid_rs::geometry::Primitive;
        use solid_rs::scene::{Image, Material, Mesh, Texture};

    let mut builder = SceneBuilder::named("UnrealScene");
    let root = builder.add_root_node("Root");

    // Standard export scanning
    let mut texture_exports: Vec<(usize, Texture2DAsset)> = Vec::new();
    let mut material_exports: Vec<(usize, MaterialAsset)> = Vec::new();
    let mut mesh_exports: Vec<(usize, StaticMeshAsset)> = Vec::new();
    let mut level_exports: Vec<(usize, LevelAsset)> = Vec::new();

    let mut export_names: HashMap<String, usize> = HashMap::new();
    for (i, export) in pkg.exports.iter().enumerate() {
        let name = pkg.resolve_name(export.object_name);
        export_names.insert(name.clone(), i);
    }

        // Classify each export
    for (export_idx, export) in pkg.exports.iter().enumerate() {
        let class_name = pkg.resolve_export_class_name(export.class_index);
        if class_name.is_empty() { continue; }

        match class_name.as_str() {
            "Texture2D" => {
                if let Ok(tex) = crate::assets::texture::read_texture2d(pkg, export_idx, reader) {
                    texture_exports.push((export_idx, tex));
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            "Material" | "MaterialInstanceConstant" | "MaterialInstance" => {
                if let Ok(mat) = crate::assets::material::read_material(pkg, export_idx, reader) {
                    material_exports.push((export_idx, mat));
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            "StaticMesh" => {
                if let Ok(mesh) = crate::assets::static_mesh::read_static_mesh(pkg, export_idx, reader) {
                    mesh_exports.push((export_idx, mesh));
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            "CubeBuilder" => {
                match crate::assets::static_mesh::read_cube_builder(pkg, export_idx, reader) {
                    Ok(mesh) => mesh_exports.push((export_idx, mesh)),
                    Err(e) => println!("[ERR] CubeBuilder: {e}"),
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            "BrushComponent" => {
                match crate::assets::static_mesh::read_brush_component(pkg, export_idx, reader) {
                    Ok(mesh) => mesh_exports.push((export_idx, mesh)),
                    Err(e) => println!("[ERR] BrushComponent: {e}"),
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            "World" | "Level" | "WorldPartition" | "PersistentLevel" | "BlueprintGeneratedClass" => {
                if let Ok(level) = crate::assets::world::read_world(pkg, export_idx, reader) {
                    level_exports.push((export_idx, level));
                }
                let _ = rewind_to_export(pkg, export_idx, reader);
            }
            _ => {}
        }
    }



    // 3. Convert textures to Solid3D images
    let mut solid_images: Vec<Image> = Vec::new();
    let mut solid_textures: Vec<Texture> = Vec::new();
    let mut tex_export_to_image_index: HashMap<usize, usize> = HashMap::new();

    for (export_idx, tex) in &texture_exports {
        if config.embed_textures {
            match crate::assets::texture::texture_to_solid_image(tex) {
                Ok(img) => {
                    let img_idx = solid_images.len();
                    tex_export_to_image_index.insert(*export_idx, img_idx);
                    solid_images.push(img);

                    let tex_name = format!("{}_tex", tex.name);
                    solid_textures.push(Texture::new(tex_name, img_idx));
                }
                Err(_) => {}
            }
        }
    }

    // 4. Convert materials to Solid3D materials
    let mut solid_materials: Vec<solid_rs::scene::Material> = Vec::new();
    let mut mat_export_to_material_index: HashMap<usize, usize> = HashMap::new();

    for (export_idx, mat) in &material_exports {
        let mut solid_mat = Material::new(&mat.name);
        solid_mat.base_color_factor = mat.base_color;
        solid_mat.metallic_factor = mat.metallic;
        solid_mat.roughness_factor = mat.roughness;
        solid_mat.emissive_factor = mat.emissive_color;
        solid_mat.double_sided = mat.two_sided;

        // Map textures
        if let Some(ref tex_info) = mat.textures.base_color {
            if let Some(export_idx) = tex_info.export_index {
                if let Some(&img_idx) = tex_export_to_image_index.get(&export_idx) {
                    solid_mat.base_color_texture =
                        Some(solid_rs::scene::TextureRef::new(img_idx));
                }
            }
        }

        let mat_idx = builder.push_material(solid_mat.clone());
        mat_export_to_material_index.insert(*export_idx, mat_idx);
        solid_materials.push(solid_mat); // keep local copy for material_index fallback
    }

    // 5. Add a default material if none exist
    if solid_materials.is_empty() {
        let default_mat = Material::solid_color("DefaultMaterial", Vec4::new(0.8, 0.8, 0.8, 1.0));
        builder.push_material(default_mat);
        solid_materials.push(Material::solid_color("DefaultMaterial", Vec4::new(0.8, 0.8, 0.8, 1.0)));
    }

    // 6. Convert meshes — push directly to the builder so indices are valid
    let mut mesh_export_to_mesh_index: HashMap<usize, usize> = HashMap::new();

    for (export_idx, mesh_asset) in &mesh_exports {
        if let Some(lod) = mesh_asset.lods.first() {
            let mut solid_mesh = Mesh::new(&mesh_asset.name);

            for v in &lod.vertices {
                solid_mesh.vertices.push(ue_vertex_to_solid(v));
            }

            for section in &lod.sections {
                let mat_idx = if section.material_index < solid_materials.len() {
                    Some(section.material_index)
                } else {
                    Some(0)
                };

                let mut indices = Vec::with_capacity(section.num_indices as usize);
                for i in 0..section.num_indices as usize {
                    let idx = (section.first_index as usize) + i;
                    if idx < lod.indices.len() {
                        indices.push(lod.indices[idx]);
                    }
                }

                solid_mesh.primitives.push(Primitive::triangles(indices, mat_idx));
            }

            solid_mesh.compute_bounds();
            let mesh_idx = builder.push_mesh(solid_mesh);
            mesh_export_to_mesh_index.insert(*export_idx, mesh_idx);
        }
    }

    // 7. Process level actors into the scene graph
    // (If no level exports were found, all meshes are attached directly to root below)
    for (_export_idx, level) in &level_exports {
        for actor in &level.actors {
            let actor_node = if config.flatten_hierarchy {
                root
            } else {
                let node = builder.add_child_node(root, &actor.name);
                builder.set_transform(node, mat4_to_transform(actor.transform));
                node
            };

            for component in &actor.components {
                match component {
                    ActorComponent::StaticMesh(smc) => {
                        if let Some(mesh_idx) = smc.static_mesh_export_idx {
                            if let Some(&solid_mesh_idx) =
                                mesh_export_to_mesh_index.get(&mesh_idx)
                            {
                                let mesh_node = if config.flatten_hierarchy {
                                    actor_node
                                } else {
                                    let node = builder.add_child_node(
                                        actor_node,
                                        &smc.name,
                                    );
                                    builder.set_transform(
                                        node,
                                        mat4_to_transform(smc.transform),
                                    );
                                    node
                                };
                                builder.attach_mesh(mesh_node, solid_mesh_idx);
                            }
                        }
                    }
                    ActorComponent::SkeletalMesh(sk) => {
                        if let Some(mesh_idx) = sk.skeletal_mesh_export_idx {
                            if let Some(&solid_mesh_idx) =
                                mesh_export_to_mesh_index.get(&mesh_idx)
                            {
                                let mesh_node = if config.flatten_hierarchy {
                                    actor_node
                                } else {
                                    let node = builder.add_child_node(
                                        actor_node,
                                        &sk.name,
                                    );
                                    builder.set_transform(
                                        node,
                                        mat4_to_transform(sk.transform),
                                    );
                                    node
                                };
                                builder.attach_mesh(mesh_node, solid_mesh_idx);
                            }
                        }
                    }
                    _ => {} // Skip other component types for now
                }
            }
        }
    }

    // 7b. If no level actors were processed, fall back: attach all meshes directly to root
    let used_mesh_indices: HashSet<usize> = {
        let mut set = std::collections::HashSet::new();
        for (_export_idx, level) in &level_exports {
            for actor in &level.actors {
                for component in &actor.components {
                    match component {
                        ActorComponent::StaticMesh(smc) => {
                            if let Some(mi) = smc.static_mesh_export_idx {
                                if let Some(&si) = mesh_export_to_mesh_index.get(&mi) {
                                    set.insert(si);
                                }
                            }
                        }
                        ActorComponent::SkeletalMesh(sk) => {
                            if let Some(mi) = sk.skeletal_mesh_export_idx {
                                if let Some(&si) = mesh_export_to_mesh_index.get(&mi) {
                                    set.insert(si);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        set
    };

    // Attach any meshes not connected to the scene graph
    for (export_idx, solid_mesh_idx) in &mesh_export_to_mesh_index {
        if !used_mesh_indices.contains(solid_mesh_idx) {
            let mesh_name = pkg.resolve_name(pkg.exports[*export_idx].object_name);
            let mesh_node = if config.flatten_hierarchy {
                root
            } else {
                builder.add_child_node(root, &mesh_name)
            };
            builder.attach_mesh(mesh_node, *solid_mesh_idx);
            println!("[SCENE] Attached mesh '{mesh_name}' (export[{export_idx}]) to root");
        }
    }

    // 8. If merge_meshes is enabled, merge all meshes into one
    let scene = builder.build();
    if config.merge_meshes {
        Ok(merge_all_meshes(scene))
    } else {
        Ok(scene)
    }
}

/// Convert an UnrealVertex to a Solid3D Vertex.
fn ue_vertex_to_solid(uv: &UnrealVertex) -> solid_rs::geometry::Vertex {
    let mut v = solid_rs::geometry::Vertex::new(uv.position);
    v.normal = Some(uv.normal);
    v.tangent = Some(Vec4::new(uv.tangent.x, uv.tangent.y, uv.tangent.z, uv.tangent.w));
    v.uvs[0] = Some(uv.uv[0]);
    v.colors[0] = Some(Vec4::new(
        uv.color[0] as f32 / 255.0,
        uv.color[1] as f32 / 255.0,
        uv.color[2] as f32 / 255.0,
        uv.color[3] as f32 / 255.0,
    ));
    v
}

/// Convert a glam Mat4 to a Solid3D Transform.
fn mat4_to_transform(m: Mat4) -> solid_rs::geometry::Transform {
    let (scale, rotation, translation) = m.to_scale_rotation_translation();
    solid_rs::geometry::Transform {
        translation,
        rotation,
        scale,
    }
}

/// Merge all meshes in a scene into a single mesh, preserving material assignments.
fn merge_all_meshes(mut scene: solid_rs::scene::Scene) -> solid_rs::scene::Scene {
    if scene.meshes.len() <= 1 {
        return scene;
    }

    use solid_rs::geometry::Primitive;

    let mut merged = solid_rs::scene::Mesh::new("MergedLevel");
    let mut vertex_offset: u32 = 0;

    for mesh in &scene.meshes {
        // Copy vertices
        for v in &mesh.vertices {
            merged.vertices.push(v.clone());
        }

        // Copy primitives with adjusted indices
        for prim in &mesh.primitives {
            let adjusted_indices: Vec<u32> = prim
                .indices
                .iter()
                .map(|i| i + vertex_offset)
                .collect();
            merged
                .primitives
                .push(Primitive::triangles(adjusted_indices, prim.material_index));
        }

        vertex_offset += mesh.vertices.len() as u32;
    }

    merged.compute_bounds();
    scene.meshes = vec![merged];

    // Rebuild the node tree: attach all nodes to root referencing the merged mesh
    // For simplicity, replace the first mesh and detach all others
    for node in &mut scene.nodes {
        if node.mesh.is_some() {
            node.mesh = Some(0);
        }
    }

    scene
}

/// Seek back to the start of the export's serial data.
fn rewind_to_export(
    pkg: &UPackage,
    export_idx: usize,
    reader: &mut (dyn solid_rs::traits::ReadSeek),
) -> Result<(), UnrealError> {
    let export = pkg.exports.get(export_idx).ok_or_else(|| UnrealError::Parse {
        context: "rewind_to_export",
        detail: format!("export index {export_idx} out of range"),
    })?;

    let offset = if pkg.version.is_ue5() {
        export.serial_offset as u64
    } else {
        export.serial_offset as u64
    };

    reader.seek(std::io::SeekFrom::Start(offset))?;
    Ok(())
}


