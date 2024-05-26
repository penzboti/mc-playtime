// found egui here: https://blog.logrocket.com/state-rust-gui-libraries/
// basic window code from https://github.com/emilk/egui/blob/master/examples/hello_world_simple/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;

use eframe::egui;

use rfd::FileDialog;

#[derive(PartialEq, Debug)]
enum State {
    Test,
    End,
}


// 3 versions of folder checking:

// then after that we search in the statictics
// do we need both of the uuids in both singleplayer and servers?

fn test() {
    let (_, folders) = read_folder(&std::path::PathBuf::from(r"C:\Users\penzboti\AppData\Roaming\.minecraft\saves"));
    folders.iter().for_each(|x| {
        if is_minecraft_save(x) {
            println!("Minecraft save found in folder: {:?}", x.file_name().unwrap());
        }
    });
}

fn is_minecraft_save(path: &std::path::PathBuf) -> bool {
    // we have to check if it is a valid minecraft generated folder
    
    // read the contents beforehand
    let (files, _folders) = read_folder(path);

    // 3 versions: 

    // 1. .minecraft folder: search for /saves
    // 2. saves folder: search for in one of the folders a level.dat
    
    // 3. a save folder: search for level.dat
    let level_dat = files.iter().find(|x| x.file_name().unwrap() == "level.dat");
    match level_dat {
        Some(_) => true,
        None => false,
    }
}

fn read_folder(path: &std::path::PathBuf) -> (Vec<std::path::PathBuf>, Vec<std::path::PathBuf>) {
    // https://stackoverflow.com/questions/26076005/how-can-i-list-files-of-a-directory-in-rust
    let items = fs::read_dir(path).unwrap().map(|x| x.unwrap().path()).collect::<Vec<std::path::PathBuf>>();

    // into_iter instead of iter
    // it took me so long
    // fount out about it in https://doc.rust-lang.org/std/iter/struct.Map.html
    // explained in https://stackoverflow.com/questions/34733811/what-is-the-difference-between-iter-and-into-iter
    let files: Vec<std::path::PathBuf> = items.clone().into_iter().filter( |x| x.is_file() ).collect();
    let folders = items.clone().into_iter().filter( |x| x.is_dir() ).collect::<Vec<std::path::PathBuf>>();
    (files, folders)
}

fn state_test(ui: &mut egui::Ui, state: &mut State) {
    ui.heading("Test environment");
    if ui.button("rfd test").clicked() {
        let folder = FileDialog::new()
            .set_title("Choose your saves folder")
            .set_directory("/")
            .pick_folder();
        match folder {
            Some(_folder) => test(),
            None => println!("No files selected"),
        }
    }
    if ui.button("End test").clicked() {
        *state = State::End;
    }
}

fn state_end(ui: &mut egui::Ui, _state: &mut State) {
    ui.heading("End of test");
}

fn main() -> Result<(), eframe::Error> {
    test();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };

    let mut state = State::Test;

    // keep in mind!!: https://stackoverflow.com/a/75716961/12706133
    eframe::run_simple_native("Minecraft Playtime Calculator", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state, State::Test, "Test");
                ui.selectable_value(&mut state, State::End, "End");
            });
            ui.separator();
            match state {
                State::Test => state_test(ui, &mut state),
                State::End => state_end(ui, &mut state),
            }
        });
    })
}