//! Bookle Desktop - Tauri commands and app setup

use bookle_core::Book;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

/// Application state
pub struct AppState {
    library: Mutex<Library>,
}

/// Library index and storage management
pub struct Library {
    /// Map of book ID to metadata
    index: HashMap<String, BookSummary>,
    /// Storage directory for books
    storage_path: PathBuf,
}

impl Library {
    fn new(storage_path: PathBuf) -> Self {
        let mut lib = Self {
            index: HashMap::new(),
            storage_path,
        };
        lib.load_index();
        lib
    }

    fn index_path(&self) -> PathBuf {
        self.storage_path.join("library.json")
    }

    fn books_dir(&self) -> PathBuf {
        self.storage_path.join("books")
    }

    fn cache_dir(&self) -> PathBuf {
        self.storage_path.join("cache")
    }

    fn load_index(&mut self) {
        let index_path = self.index_path();
        if index_path.exists() {
            if let Ok(file) = File::open(&index_path) {
                if let Ok(index) = serde_json::from_reader(file) {
                    self.index = index;
                }
            }
        }
    }

    fn save_index(&self) -> Result<(), String> {
        let index_path = self.index_path();
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = File::create(&index_path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(file, &self.index).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn book_path(&self, id: &str) -> PathBuf {
        self.books_dir().join(format!("{}.json", id))
    }

    fn add_book(&mut self, book: Book) -> Result<BookSummary, String> {
        let summary = BookSummary::from(&book);
        let book_path = self.book_path(&summary.id);

        // Ensure directories exist
        fs::create_dir_all(self.books_dir()).map_err(|e| e.to_string())?;

        // Save book IR
        let file = File::create(&book_path).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &book).map_err(|e| e.to_string())?;

        // Update index
        self.index.insert(summary.id.clone(), summary.clone());
        self.save_index()?;

        Ok(summary)
    }

    fn get_book(&self, id: &str) -> Result<Book, String> {
        let book_path = self.book_path(id);
        if !book_path.exists() {
            return Err("Book not found".to_string());
        }

        let file = File::open(&book_path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| e.to_string())
    }

    fn delete_book(&mut self, id: &str) -> Result<(), String> {
        // Remove from index
        self.index.remove(id);
        self.save_index()?;

        // Delete book file
        let book_path = self.book_path(id);
        if book_path.exists() {
            fs::remove_file(&book_path).map_err(|e| e.to_string())?;
        }

        // Delete any cached conversions
        let cache_pattern = self.cache_dir().join(format!("{}.*", id));
        if let Ok(entries) = fs::read_dir(self.cache_dir()) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(id) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        let _ = cache_pattern; // Suppress unused warning

        Ok(())
    }

    fn list_books(&self) -> Vec<BookSummary> {
        self.index.values().cloned().collect()
    }
}

/// Book summary for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSummary {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
}

impl From<&Book> for BookSummary {
    fn from(book: &Book) -> Self {
        Self {
            id: book.id.to_string(),
            title: book.metadata.title.clone(),
            authors: book.metadata.creator.clone(),
            language: book.metadata.language.clone(),
        }
    }
}

/// Book detail response with chapters
#[derive(Debug, Serialize)]
pub struct BookDetail {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub language: String,
    pub chapters: Vec<ChapterSummary>,
}

#[derive(Debug, Serialize)]
pub struct ChapterSummary {
    pub title: String,
    pub index: usize,
}

impl From<&Book> for BookDetail {
    fn from(book: &Book) -> Self {
        Self {
            id: book.id.to_string(),
            title: book.metadata.title.clone(),
            authors: book.metadata.creator.clone(),
            description: book.metadata.description.clone(),
            language: book.metadata.language.clone(),
            chapters: book
                .chapters
                .iter()
                .enumerate()
                .map(|(i, ch)| ChapterSummary {
                    title: ch.title.clone(),
                    index: i,
                })
                .collect(),
        }
    }
}

/// Get application version
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// List books in the library
#[tauri::command]
async fn list_books(state: State<'_, AppState>) -> Result<Vec<BookSummary>, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    Ok(library.list_books())
}

/// Import a book from a file path
#[tauri::command]
async fn import_book(path: String, state: State<'_, AppState>) -> Result<BookSummary, String> {
    use bookle_core::decoder::decoder_for_extension;
    use std::path::Path;

    let path = Path::new(&path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Could not determine file extension".to_string())?;

    let decoder = decoder_for_extension(ext)
        .ok_or_else(|| format!("Unsupported format: {}", ext))?;

    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);

    let book = decoder.decode(&mut reader).map_err(|e| e.to_string())?;

    let mut library = state.library.lock().map_err(|e| e.to_string())?;
    library.add_book(book)
}

/// Export a book to a specific format
#[tauri::command]
async fn export_book(
    id: String,
    format: String,
    output_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use bookle_core::encoder::encoder_for_format;

    let encoder = encoder_for_format(&format)
        .ok_or_else(|| format!("Unsupported output format: {}", format))?;

    let library = state.library.lock().map_err(|e| e.to_string())?;
    let book = library.get_book(&id)?;
    drop(library); // Release lock before encoding

    // Encode the book
    let mut output = Vec::new();
    encoder
        .encode(&book, &mut output)
        .map_err(|e| e.to_string())?;

    // Write to file
    fs::write(&output_path, output).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get book details by ID
#[tauri::command]
async fn get_book(id: String, state: State<'_, AppState>) -> Result<BookDetail, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    let book = library.get_book(&id)?;
    Ok(BookDetail::from(&book))
}

/// Delete a book from the library
#[tauri::command]
async fn delete_book(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut library = state.library.lock().map_err(|e| e.to_string())?;
    library.delete_book(&id)
}

/// Open file dialog to select a book to import
#[tauri::command]
async fn open_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file = app
        .dialog()
        .file()
        .add_filter("Ebooks", &["epub", "mobi", "azw", "azw3", "pdf", "md"])
        .blocking_pick_file();

    Ok(file.and_then(|f| f.as_path().map(|p| p.to_string_lossy().to_string())))
}

/// Open save dialog for exporting
#[tauri::command]
async fn save_file_dialog(
    app: tauri::AppHandle,
    default_name: String,
    format: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let extension = match format.as_str() {
        "epub" => "epub",
        "pdf" | "typst" | "typ" => "typ",
        _ => return Err(format!("Unknown format: {}", format)),
    };

    let file = app
        .dialog()
        .file()
        .add_filter(&format.to_uppercase(), &[extension])
        .set_file_name(&default_name)
        .blocking_save_file();

    Ok(file.and_then(|f| f.as_path().map(|p| p.to_string_lossy().to_string())))
}

fn get_storage_path() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "bookle", "Bookle") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        // Fallback to current directory
        PathBuf::from("bookle_data")
    }
}

fn create_menu(app: &tauri::AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

    // File menu
    let import_item = MenuItemBuilder::with_id("import", "Import Book...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;

    let export_item = MenuItemBuilder::with_id("export", "Export Book...")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(app)?;

    let quit_item = MenuItemBuilder::with_id("quit", "Quit")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&import_item)
        .item(&export_item)
        .separator()
        .item(&quit_item)
        .build()?;

    // Edit menu
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // View menu
    let refresh_item = MenuItemBuilder::with_id("refresh", "Refresh Library")
        .accelerator("CmdOrCtrl+R")
        .build(app)?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&refresh_item)
        .build()?;

    // Help menu
    let about_item = MenuItemBuilder::with_id("about", "About Bookle")
        .build(app)?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&about_item)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&help_menu)
        .build()?;

    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage_path = get_storage_path();
    let library = Library::new(storage_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            library: Mutex::new(library),
        })
        .setup(|app| {
            // Create and set the menu
            let menu = create_menu(app.handle())?;
            app.set_menu(menu)?;

            Ok(())
        })
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "import" => {
                    // Trigger file dialog from frontend
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("menu:import", ());
                    }
                }
                "export" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("menu:export", ());
                    }
                }
                "refresh" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("menu:refresh", ());
                    }
                }
                "about" => {
                    use tauri_plugin_dialog::DialogExt;
                    app.dialog()
                        .message(format!(
                            "Bookle Desktop v{}\n\nAn ebook management application.",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .title("About Bookle")
                        .blocking_show();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            list_books,
            import_book,
            export_book,
            get_book,
            delete_book,
            open_file_dialog,
            save_file_dialog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
