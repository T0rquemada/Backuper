use slint::{ComponentHandle, Model, VecModel, SharedString, ModelRc};
use std::rc::Rc;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir; // Import WalkDir

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    std::env::set_var("SLINT_BACKEND", "gl");

    let ui = MainWindow::new()?;
    let files_model = Rc::new(VecModel::<SharedString>::from(vec![]));
    ui.set_file_list(ModelRc::from(files_model.clone()));

    let ui_handle = ui.as_weak();

    // --- PICK FILE LOGIC ---
    let files_model_add = files_model.clone();
    ui.on_pick_file(move || {
        let _ui = ui_handle.unwrap();
        // Allow picking folders too, or just use pick_file if you want mixed.
        // rfd::FileDialog doesn't easily support "both" on all platforms
        // usually you pick one or the other. For now, we assume pick_file
        // but users might type a folder path or your UI might need a separate "Add Folder" button.
        // If you specifically need to pick folders, use .pick_folder()
        let file = rfd::FileDialog::new()
            .set_title("Select a file or folder")
            .pick_folder(); // Changed to pick_folder for this example, or keep pick_file

        if let Some(path) = file {
            let path_str = path.display().to_string();
            files_model_add.push(path_str.into());
            println!("Added: {}", path.display());
        }
    });

    // --- ZIP LOGIC ---
    let files_model_zip = files_model.clone();
    ui.on_create_zip(move || {
        println!("Starting Archive Process...");

        let count = files_model_zip.row_count();
        if count == 0 {
            println!("No files selected to zip!");
            return;
        }

        let path = Path::new("archive.zip");
        let file = File::create(&path).expect("Could not create zip file");
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Define a buffer once to reuse memory
        let mut buffer = Vec::new();

        for i in 0..count {
            if let Some(selected_path_str) = files_model_zip.row_data(i) {
                let selected_path = Path::new(selected_path_str.as_str());

                if !selected_path.exists() { continue; }

                // LOGIC: Check if it is a directory or a file
                if selected_path.is_dir() {
                    // Use WalkDir to find all nested files
                    let walker = WalkDir::new(selected_path);

                    for entry in walker.into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();

                        // We only care about files (directories are created implicitly by the zip structure)
                        if path.is_file() {
                            // Calculate the path relative to the selected parent
                            // e.g. if we selected "/home/user/data" and found "/home/user/data/img/1.png"
                            // we want the zip entry to be "data/img/1.png"
                            let name = path.strip_prefix(selected_path.parent().unwrap_or(selected_path))
                                .unwrap_or(path);

                            let name_str = name.to_string_lossy();
                            println!("Adding file from dir: {}", name_str);

                            zip.start_file(name_str, options).unwrap();
                            if let Ok(mut f) = File::open(path) {
                                buffer.clear();
                                if let Ok(_) = f.read_to_end(&mut buffer) {
                                    let _ = zip.write_all(&buffer);
                                }
                            }
                        }
                    }
                } else {
                    // It is a single file
                    let file_name = selected_path.file_name().unwrap().to_string_lossy();
                    println!("Adding single file: {}", file_name);

                    zip.start_file(file_name, options).unwrap();
                    if let Ok(mut f) = File::open(selected_path) {
                        buffer.clear();
                        if let Ok(_) = f.read_to_end(&mut buffer) {
                            let _ = zip.write_all(&buffer);
                        }
                    }
                }
            }
        }

        match zip.finish() {
            Ok(_) => println!("Success! Archive created at: {}", path.display()),
            Err(e) => println!("Error finishing zip: {:?}", e),
        }
    });

    ui.run()
}
