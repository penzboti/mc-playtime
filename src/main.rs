// found egui here: https://blog.logrocket.com/state-rust-gui-libraries/
// basic window code from https://github.com/emilk/egui/blob/master/examples/hello_world_simple/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

use rfd::FileDialog;

#[derive(PartialEq, Debug)]
enum State {
    Test,
    End,
}

fn state_test(ui: &mut egui::Ui, state: &mut State) {
    ui.heading("Test environment");
    if ui.button("rfd test").clicked() {
        let files = FileDialog::new()
            .set_title("Choose your saves folder")
            .set_directory("/")
            .pick_folder();
        match files {
            Some(files) => println!("{:?}", files),
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