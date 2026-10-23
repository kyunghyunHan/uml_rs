//! Reproducible diagram artifacts, independent of the viewport zoom.
use std::path::Path;
use uml_studio::{demo, export, scene, storage};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(args.get(1).expect("output directory"));
    std::fs::create_dir_all(root).unwrap();
    let p = demo::relationship_review_project();
    p.validate().unwrap();
    let sc = scene::build(&p.diagrams[0]);
    let options = scene::ExportOptions::default();
    export::png(&sc, &root.join("relationships.png"), &options).unwrap();
    export::svg(&sc, &root.join("relationships.svg"), &options).unwrap();
    let path = root.join("relationships.umlproj");
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
    println!(
        "Wrote relation PNG, SVG and v3 project to {}",
        root.display()
    );
}
