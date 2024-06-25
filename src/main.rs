#![warn(dead_code)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use std::path::PathBuf;

use eframe::egui;

// rfd: https://github.com/emilk/egui/discussions/1597
use rfd::FileDialog;

use md5;

#[derive(PartialEq, Debug, Clone, Copy)]
enum State {
    Input,

    // you see, we could have made a loading bar, but only with tokio-rs async.
    // and i dont want to do that
    // but we still need initiation because rendering is last.
    // but we cant close the program while loading so tokio would be beneficial but idc
    LoadingInitiated,
    Loading,
    Result,
    Export,
}

#[derive(Debug, Clone)]
struct PlayerUuid {
    online: String,
    offline: String,
}

#[derive(PartialEq, Debug, Clone)]
enum PlayTime {
    // https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html (tuple struct)
    Online(u64),
    Offline(u64),
    Mixed(u64),
    None
}

#[derive(Debug, Clone)]
enum GameType {
    Singleplayer,
    Multiplayer
}

#[derive(Debug, Clone)]
struct World {
    origin: PathBuf,
    path: PathBuf,
    playtime: PlayTime,
    type_: GameType,
    active: bool // for exporting
}

#[derive(Debug, Clone)]
struct Folder {
    folders: Vec<PathBuf>,
    files: Vec<PathBuf>
}

fn handle_playtime(input_folders: Vec<PathBuf>, name: String) -> Vec<World> {
    let mut folders = vec![];
    let uuids = get_uuids(name);
    input_folders.iter().for_each(|x| {
        folders.extend(get_minecraft_worlds(&x, 0));
    });
    let worlds = folders.iter().map(|world| {
        World {
            origin: world.origin.clone(),
            playtime: get_playtime(world.path.clone(), uuids.clone()),
            // thought there was a ..world syntax, but there isn't
            // only when you set defaults
            path: world.path.clone(),
            type_: world.type_.clone(),
            active: true
        }
    }).collect::<Vec<World>>();
    worlds
}

fn get_uuids(name: String) -> PlayerUuid {
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
    // used chatgpt here
    let input = format!("OfflinePlayer:{}", name);
    let hash = md5::compute(input.as_bytes());
    let mut byte_array = hash.0;

    byte_array[6] = (byte_array[6] & 0x0f) | 0x30;
    byte_array[8] = (byte_array[8] & 0x3f) | 0x80;
    let hexstring = byte_array
        .iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<String>>()
    .join("");
    let offline = split_uuid(hexstring);


    PlayerUuid{online, offline}
}

// https://minecraft.wiki/w/Statistics
fn get_playtime(path: PathBuf, uuids: PlayerUuid) -> PlayTime {
    let folders = read_folder(&path).folders;
    // this shouldn't happen tho
    if !folders.iter().any(|x| x.file_name().unwrap() == "stats") { return PlayTime::None; }
    
    let stats_path = path.join("stats");
    let files = read_folder(&stats_path).files;
    // we store it in ticks
    let mut playtime= PlayTime::None;

    // https://stackoverflow.com/questions/28991050/how-to-iterate-a-vect-with-the-indexed-position
    [uuids.online.clone() + ".json", uuids.offline.clone() + ".json"].iter().enumerate().for_each(|(i, name)| {
        // https://profpatsch.de/notes/rust-string-conversions
        let is_player = files.iter().any(|x| x.file_name().unwrap().to_os_string().into_string().unwrap() == *name);
        if is_player {
            let filepath = stats_path.clone().join(name);
            let file_string = fs::read_to_string(filepath).unwrap();

            // documentation: https://github.com/serde-rs/json
            let json: serde_json::Value = serde_json::from_str(&file_string).unwrap();
            // ticks are 1/20 of a second (normally)
            let playtime_ticks = json["stats"]["minecraft:custom"]["minecraft:play_time"].as_u64().unwrap_or(0);
            // legacy playtime
            // it seems like it still tracked the playtime in ticks, so the change was just a rename (a thruthful one)
            let playtime_minute = json["stats"]["minecraft:custom"]["minecraft:play_one_minute"].as_u64().unwrap_or(0);

            let current_playtime = playtime_ticks + playtime_minute;
            if current_playtime == 0 { return; }
            match i {
                0 => playtime = PlayTime::Online(current_playtime),
                1 => {
                    match playtime {
                        PlayTime::Online(t) => playtime = PlayTime::Mixed(t + current_playtime),
                        _ => playtime = PlayTime::Offline(current_playtime),
                    }
                },
                _ => {}
            }
        }
    });

    playtime
}

fn get_minecraft_worlds(path: &PathBuf, depth: u8) -> Vec<World> {
    // stop looking too far into the fs, since we're using recursivity
    //? we might get away with depth > 3, test it please
    if depth > 4 {return vec![];}

    // we get your inputted path and get all the worlds from it
    let mut worlds: Vec<World>= vec![];
    
    // read the contents beforehand
    let read_folder = read_folder(path);
    let folders = read_folder.folders;
    let files = read_folder.files;

    // go down a level, and search for the stats folder
    folders.clone().iter().for_each(|f| {
        worlds.extend(get_minecraft_worlds(f, depth.clone()+1));
    });

    // a stats folder is found, we return the path
    if folders.iter().any(|x| x.file_name().unwrap() == "stats") {
        let mut origin = path.clone();
        for _ in 0..depth {
            origin = origin.parent().unwrap().to_path_buf();
        }
        let mut current = World {
            origin,
            path: path.clone(),
            playtime: PlayTime::None,
            type_: GameType::Singleplayer,
            active: true
        };
        if !files.iter().any(|x| x.file_name().unwrap() == "icon.png") {
            current.type_ = GameType::Multiplayer;
        }
        worlds.push(current);
    }

    worlds
}

fn read_folder(path: &PathBuf) -> Folder {
    // https://stackoverflow.com/questions/26076005/how-can-i-list-files-of-a-directory-in-rust
    let items = fs::read_dir(path).unwrap().map(|x| x.unwrap().path()).collect::<Vec<PathBuf>>();

    // into_iter instead of iter
    // it took me so long
    // fount out about it in https://doc.rust-lang.org/std/iter/struct.Map.html
    // explained in https://stackoverflow.com/questions/34733811/what-is-the-difference-between-iter-and-into-iter
    let files: Vec<PathBuf> = items.clone().into_iter().filter( |x| x.is_file() ).collect();
    let folders = items.clone().into_iter().filter( |x| x.is_dir() ).collect::<Vec<PathBuf>>();
    Folder{files, folders}
}


fn state_input(ui: &mut egui::Ui, state: &mut State, input_folders: &mut Vec<PathBuf>) {
    ui.heading("Input");
    ui.label("you may choose your minecraft folder, save folder, or a world folder. this also applies to server folders.");
    if ui.button("Select folders").clicked() {
        let folder = FileDialog::new()
            .set_title("Folder selection")
            .pick_folders();
        match folder {
            Some(folder) => input_folders.extend(folder),
            None => {},
        }
    }
    if ui.button("Search folders").clicked() {
        *state = State::LoadingInitiated;
    }

    let mut removed = false;
    for i in 0..input_folders.len() {
        let mut j = i;
        if removed {j-=1;}
        let folder = input_folders[j].clone();
        ui.horizontal(|ui| {
            if ui.button("X").clicked() {
                removed = true;
                input_folders.remove(i);
            }
            ui.label(format!("Folder: {}", folder.display()));
        });
        
    }
}

fn state_loading(ui: &mut egui::Ui, state: &mut State, folders: Vec<PathBuf>, name: String, worlds: &mut Vec<World>) {
    ui.heading("Loading");
    ui.label("Loading the playtime data from the selected folders");
    ui.label("This may take some time");
    if state == &State::LoadingInitiated {*state = State::Loading;}
    else {
        worlds.extend(handle_playtime(folders, name));
        *state = State::Result;
    }
}

fn state_result(ui: &mut egui::Ui, _state: &mut State, worlds: &mut Vec<World>) {
    ui.heading("Results");
    // found this here: https://github.com/emilk/egui/issues/296
    egui::Grid::new("table").show(ui, |ui| {
        ui.label("Origin");
        ui.label("Name");
        ui.label("Type");
        ui.label("Playtime");
        ui.label("Active");
        for world in worlds.iter_mut() {
            ui.end_row();
            ui.label(world.origin.file_name().unwrap().to_str().unwrap());
            ui.label(world.path.file_name().unwrap().to_str().unwrap());
            ui.label(match world.type_ {
                GameType::Singleplayer => "Singleplayer",
                GameType::Multiplayer => "Multiplayer",
            });

            //* playtime type is only for export
            let _playtime_type = match world.playtime {
                PlayTime::Online(_) => "online",
                PlayTime::Offline(_) => "offline",
                PlayTime::Mixed(_) => "both offline and online",
                PlayTime::None => "no",
            };

            let playtime = match world.playtime {
                PlayTime::Online(n) => n,
                PlayTime::Offline(n) => n,
                PlayTime::Mixed(n) => n,
                PlayTime::None => 0
            };

            // found no other way to display this. there is no built in formatting for time
            let playtime_raw_seconds = playtime /20;
            let days = playtime_raw_seconds/60/60/24;
            let hours = playtime_raw_seconds/60/60 - days*24;
            let minutes = playtime_raw_seconds/60 - hours*60 - days*24*60;
            let seconds = playtime_raw_seconds - minutes*60 - hours*60*60 - days*24*60*60;

            ui.label(format!("{}d : {}h : {}m : {}s", days, hours, minutes, seconds));
            ui.checkbox(&mut world.active, "");
        }
    });
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
        ..Default::default()
    };

    let mut state = State::Input;

    let mut input_folders: Vec<PathBuf> = vec![];
    let mut worlds: Vec<World> = vec![];

    // found egui here: https://blog.logrocket.com/state-rust-gui-libraries/
    // basic window code from https://github.com/emilk/egui/blob/master/examples/hello_world_simple/src/main.rs
    // keep in mind!!: https://stackoverflow.com/a/75716961/12706133
    eframe::run_simple_native("Minecraft Playtime Calculator", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state, State::Input, "Input");
                if worlds.len() > 0 {
                    ui.selectable_value(&mut state, State::Result, "Result");
                    // ui.selectable_value(&mut state, State::Export, "Export");
                }
            });
            ui.separator();
            egui::ScrollArea::both().show(ui, |ui| {
                match state {
                    State::Input => state_input(ui, &mut state, &mut input_folders),
                    State::LoadingInitiated | State::Loading => state_loading(ui, &mut state, input_folders.clone(), "penzboti".to_owned(), &mut worlds),
                    State::Result => state_result(ui, &mut state, &mut worlds),
                    // State::Export => state_export(ui, &mut state, worlds.clone()),
                    _ => {ui.label("This state is not implemented yet");},
                }
            });
        });
    })
}