// basic window code from https://github.com/emilk/egui/blob/master/examples/hello_world_simple/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };

    let mut istrue = true;
    let mut string = "string".to_owned();

    // keep in mind!!: https://stackoverflow.com/a/75716961/12706133
    eframe::run_simple_native("Minecraft Playtime Calculator", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Button").clicked() {
                string = "Button string".to_string();
            }
            ui.label(&string);
            ui.checkbox(&mut istrue, "Checkbox");
            if istrue {
                ui.text_edit_singleline(&mut string);
            }
            if ui.link("link").clicked() {
                string = "Link string".to_string();
            }
        });
    })
}