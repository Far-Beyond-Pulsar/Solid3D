use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 { eprintln!("Usage: unreal-to-fbx <input.umap>"); std::process::exit(1); }

    let input_path = PathBuf::from(&args[1]);
    let output_path = if args.len() >= 3 {
        PathBuf::from(&args[2])
    } else {
        let mut out = input_path.clone();
        out.set_extension("fbx");
        out
    };

    println!("=== Unreal \u{2192} FBX Converter ===");
    println!("Input:  {}", input_path.display());
    println!("Output: {}", output_path.display());
    println!();

    // 1. Build the registry
    let mut registry = solid_rs::registry::Registry::new();
    registry.register_loader(solid_unreal::UnrealLoader);
    registry.register_saver(solid_fbx::FbxSaver);

    // 2. Load the .umap
    println!("Loading .umap file...");
    let load_start = Instant::now();

    let scene = match registry.load_file(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: Failed to load .umap: {e}");
            std::process::exit(1);
        }
    };

    let load_elapsed = load_start.elapsed();
    println!("Loaded in {:.2}s", load_elapsed.as_secs_f64());
    println!();

    // 3. Print scene statistics
    println!("=== Scene Statistics ===");
    println!("Name:       {}", scene.name);
    println!("Nodes:      {}", scene.nodes.len());
    println!("Roots:      {}", scene.roots.len());
    println!("Meshes:     {}", scene.meshes.len());
    println!("Vertices:   {}", scene.total_vertex_count());
    println!("Indices:    {}", scene.total_index_count());
    println!("Materials:  {}", scene.materials.len());
    println!("Textures:   {}", scene.textures.len());
    println!("Images:     {}", scene.images.len());
    println!("Cameras:    {}", scene.cameras.len());
    println!("Lights:     {}", scene.lights.len());
    println!("Animations: {}", scene.animations.len());

    for (i, mesh) in scene.meshes.iter().enumerate() {
        println!("  Mesh[{}] '{}': {} vertices, {} primitives, bounds={:?}",
            i, mesh.name, mesh.vertex_count(), mesh.primitives.len(), mesh.bounds);
    }
    for (i, mat) in scene.materials.iter().enumerate() {
        println!("  Material[{}] '{}': base={:?}, metal={}, rough={}, emissive={:?}, alpha={:?}",
            i, mat.name, mat.base_color_factor, mat.metallic_factor, mat.roughness_factor,
            mat.emissive_factor, mat.alpha_mode);
    }
    println!();

    // 4. Save as FBX
    println!("Saving to FBX...");
    let save_start = Instant::now();

    match registry.save_file(&scene, &output_path) {
        Ok(()) => {
            let save_elapsed = save_start.elapsed();
            println!("Saved to '{}' in {:.2}s", output_path.display(), save_elapsed.as_secs_f64());
        }
        Err(e) => {
            eprintln!("ERROR: Failed to save FBX: {e}");
            std::process::exit(1);
        }
    }

    println!();
    println!("Done! Total time: {:.2}s", load_start.elapsed().as_secs_f64());
}
