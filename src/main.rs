use slint::{ComponentHandle, Model, VecModel, SharedString, ModelRc};
use std::rc::Rc;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;

slint::include_modules!();

fn make_zip(ui: &MainWindow, files_model: Rc<VecModel<SharedString>>, archive_name: &str) {
    let files_model_zip = files_model.clone();
    let archive_name_owned = archive_name.to_string();

    ui.on_create_zip(move || {
        println!("Starting Archive Process...");

        let count = files_model_zip.row_count();
        if count == 0 {
            println!("No files selected to zip!");
            return;
        }

        // Define the output folder
        let output_dir = Path::new("output");

        // Create the folder if it doesn't exist
        if let Err(e) = fs::create_dir_all(output_dir) {
            println!("Failed to create output directory: {:?}", e);
            return;
        }

        // Join the folder with the filename (e.g., "output" + "backup.zip" -> "output/backup.zip")
        let archive_path = output_dir.join(&archive_name_owned);

        println!("Creating file at: {}", archive_path.display());

        let file = File::create(&archive_path).expect("Could not create zip file");
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        let mut buffer = Vec::new();

        for i in 0..count {
            if let Some(selected_path_str) = files_model_zip.row_data(i) {
                let selected_path = Path::new(selected_path_str.as_str());
                if !selected_path.exists() { continue; }

                if selected_path.is_dir() {
                        let walker = WalkDir::new(selected_path);
                        for entry in walker.into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            let name = path.strip_prefix(selected_path.parent().unwrap_or(selected_path))
                                .unwrap_or(path);
                            let name_str = name.to_string_lossy();

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
                    let file_name = selected_path.file_name().unwrap().to_string_lossy();
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
            Ok(_) => println!("Success! Archive created at: {}", archive_path.display()),
            Err(e) => println!("Error finishing zip: {:?}", e),
        }
    });
}

fn file_picker(ui: &MainWindow, files_model: Rc<VecModel<SharedString>>, ui_handle: slint::Weak<MainWindow>) {
    let files_model_add = files_model.clone();

    ui.on_pick_file(move || {
        let _ui = ui_handle.unwrap();

        let file = rfd::FileDialog::new()
            .set_title("Select a file or folder")
            .pick_folder();

        if let Some(path) = file {
            let path_str = path.display().to_string();

            // Check for duplicats
            let mut already_exists = false;
            let count = files_model_add.row_count();

            for i in 0..count {
                // Get the string at row 'i'
                if let Some(existing_item) = files_model_add.row_data(i) {
                    if existing_item.as_str() == path_str {
                        already_exists = true;
                        break;
                    }
                }
            }

            if already_exists {
                println!("Duplicate ignored: {}", path_str);
                return; // Exit early, do not add the file
            }

            files_model_add.push(path_str.into());
            println!("Added: {}", path.display());
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    std::env::set_var("SLINT_BACKEND", "gl");

    let ui = MainWindow::new()?;
    let files_model = Rc::new(VecModel::<SharedString>::from(vec![]));
    ui.set_file_list(ModelRc::from(files_model.clone()));

    let ui_handle = ui.as_weak();

    // File picker logic
    file_picker(&ui, files_model.clone(), ui_handle);

    // Zip
    let archive_name = "backup.zip";
    make_zip(&ui, files_model, archive_name);

    ui.run()
}
