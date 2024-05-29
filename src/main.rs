// #![warn(dead_code)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use std::path::PathBuf;

use eframe::egui;

// rfd: https://github.com/emilk/egui/discussions/1597
use rfd::FileDialog;

use md5;

#[derive(PartialEq, Debug)]
enum State {
    Test,
    End,
}

fn test() {
    let playtimes = handle_playtime(vec![
        PathBuf::from("C:/Users/penzboti/AppData/Roaming/ATLauncher/servers".to_owned()),
        PathBuf::from("C:/Users/penzboti/AppData/Roaming/ATLauncher/instances/test/saves".to_owned()),
    ]);
}

fn handle_playtime(files: Vec<PathBuf>) -> Vec<u64> {
    let mut folders = vec![];
    let mut playtime = vec![];
    files.iter().for_each(|x| {
        folders.extend(get_minecraft_worlds(&x, 0));
    });
    folders.iter().for_each(|path| {
        let n = get_playtime(path, "penzboti".to_owned());
        playtime.push(n);
        println!("Playtime on path {} is {}", path.clone().display(), n);
        // println!("{}", path.display());
    });
    playtime
}

fn get_uuid(name: String) -> (String, String) {
    fn split_uuid(raw: String) -> String {
        vec![
            &raw[0..8],
            &raw[8..12],
            &raw[12..16],
            &raw[16..20],
            &raw[20..32]
        ].join("-")
    }
    // online
    // api: https://wiki.vg/Mojang_API#Username_to_UUID
    // working with api in rust: https://rustfordata.com/chapter_3.html
    let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
    let ans = reqwest::blocking::get(url).expect("request failed")
    .text().expect("body failed");
    let json: serde_json::Value = serde_json::from_str(ans.as_str()).unwrap();
    let online_raw = json["id"].as_str().unwrap().to_owned();
    let online = split_uuid(online_raw);

    // offline
    // found first here: https://gist.github.com/Nikdoge/474f74688b52865bf8d682a97fd4f2fe
    // then here: https://github.com/nuckle/minecraft-offline-uuid-generator/blob/main/src/js/uuid.js
    let input = format!("OfflinePlayer:{}", name);
    let hash = md5::compute(input.as_bytes());
    let mut byte_array = hash.0;

    byte_array[6] = (byte_array[6] & 0x0f) | 0x30;
    byte_array[8] = (byte_array[8] & 0x3f) | 0x80;
    let hexstring = byte_array
        .iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>()
    .join("");
    let offline = split_uuid(hexstring);


    (online, offline)
}

// https://minecraft.wiki/w/Statistics
fn get_playtime(path: &PathBuf, name: String) -> u64 {
    let (_, folders) = read_folder(path);
    if !folders.iter().any(|x| x.file_name().unwrap() == "stats") { return 0 }
    
    let stats_path = path.join("stats");
    let (files, _) = read_folder(&stats_path);
    let uuids = get_uuid(name.clone());
    // we store it in ticks
    let mut playtime = 0;

    let mut is_player_global = false;

    [uuids.0.clone() + ".json", uuids.1.clone() + ".json"].iter().for_each(|name| {
        // https://profpatsch.de/notes/rust-string-conversions
        let is_player = files.iter().any(|x| x.file_name().unwrap().to_os_string().into_string().unwrap() == *name);
        match is_player {
            false => {},
            true => {
                is_player_global = true;
                let filepath = stats_path.clone().join(name);
                let file_string = fs::read_to_string(filepath).unwrap();
                // documentation: https://github.com/serde-rs/json
                let json: serde_json::Value = serde_json::from_str(&file_string).unwrap();
                // ticks are 1/20 of a second (normally)
                let playtime_ticks = json["stats"]["minecraft:custom"]["minecraft:play_time"].as_u64().unwrap_or(0);
                // legacy playtime
                let playtime_minute = json["stats"]["minecraft:custom"]["minecraft:play_one_minute"].as_u64().unwrap_or(0);
                playtime += playtime_ticks + playtime_minute*60*20;
            },
        }
    });

    if !is_player_global {
        println!("No player found with name {}", name.clone());
    }

    playtime
}

fn get_minecraft_worlds(path: &PathBuf, depth: u8) -> Vec<PathBuf> {
    // stop looking too far into the fs, since we're using recursivity
    //? we might get away with depth > 3, test it please
    if depth > 4 {return vec![];}

    // we get your inputted path and get all the worlds from it
    let mut worlds: Vec<PathBuf>= vec![];
    
    // read the contents beforehand
    let (_, folders) = read_folder(path);

    //* variations
    // 1. .minecraft folder: search for /saves
    if folders.clone().iter().any(|x| x.file_name().unwrap() == "saves") {
        let saves_path = path.clone().join("saves");
        worlds.extend(get_minecraft_worlds(&saves_path, depth.clone()+1));
    }

    // 2. saves folder: search for in one of the folders a level.dat
    // or variation 4: a folder of server folders
    // or variation 5: a server folder
    folders.clone().iter().for_each(|f| {
        worlds.extend(get_minecraft_worlds(f, depth.clone()+1));
    });

    // 3. a save folder: search for level.dat
    if folders.iter().any(|x| x.file_name().unwrap() == "stats") {
        worlds.push(path.clone());
    }

    worlds
}

fn read_folder(path: &PathBuf) -> (Vec<PathBuf>, Vec<PathBuf>) {
    // https://stackoverflow.com/questions/26076005/how-can-i-list-files-of-a-directory-in-rust
    let items = fs::read_dir(path).unwrap().map(|x| x.unwrap().path()).collect::<Vec<PathBuf>>();

    // into_iter instead of iter
    // it took me so long
    // fount out about it in https://doc.rust-lang.org/std/iter/struct.Map.html
    // explained in https://stackoverflow.com/questions/34733811/what-is-the-difference-between-iter-and-into-iter
    let files: Vec<PathBuf> = items.clone().into_iter().filter( |x| x.is_file() ).collect();
    let folders = items.clone().into_iter().filter( |x| x.is_dir() ).collect::<Vec<PathBuf>>();
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

    // found egui here: https://blog.logrocket.com/state-rust-gui-libraries/
    // basic window code from https://github.com/emilk/egui/blob/master/examples/hello_world_simple/src/main.rs
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