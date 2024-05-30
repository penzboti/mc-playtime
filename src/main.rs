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

#[derive(Debug, Clone)]
struct Uuids {
    online: String,
    offline: String,
}

#[derive(PartialEq, Debug, Clone)]
enum PlayTime {
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
    path: PathBuf,
    playtime: PlayTime,
    type_: GameType
}

fn test() {
    let worlds = handle_playtime(vec![
        PathBuf::from("C:/Users/penzboti/AppData/Roaming/ATLauncher/servers".to_owned()),
        PathBuf::from("C:/Users/penzboti/AppData/Roaming/ATLauncher/instances/test/saves".to_owned()),
    ], "penzboti".to_owned());
    for world in worlds.iter() {
        let name = world.path.file_name().unwrap().to_str().unwrap();
        let parentname = world.path.parent().unwrap().file_name().unwrap().to_str().unwrap();
        println!("In path {:?}", world.path.display());
        println!("There is a folder named {}", parentname);
        println!("The worlds name is {}", name);
        print!("It is a {:?} world,\nwith ", world.type_);
        match world.playtime {
            PlayTime::Online(n) => {
                println!("{} online playtime", n);
            },
            PlayTime::Offline(n) => {
                println!("{} offline playtime", n);
            },
            PlayTime::Mixed(n) => {
                println!("{} playtime both on offline and online", n);
            },
            PlayTime::None => {
                println!("no playtime");
            }
        }

        let playtime = match world.playtime {
            PlayTime::Online(n) => n,
            PlayTime::Offline(n) => n,
            PlayTime::Mixed(n) => n,
            PlayTime::None => 0
        };
        println!("That is {:.2} minutes", playtime as f64/20_f64/60_f64);
        println!();
    }
}

fn handle_playtime(files: Vec<PathBuf>, name: String) -> Vec<World> {
    let mut folders = vec![];
    let uuids = get_uuids(name);
    files.iter().for_each(|x| {
        folders.extend(get_minecraft_worlds(&x, 0));
    });
    let worlds = folders.iter().map(|world| {
        World {
            playtime: get_playtime(world.path.clone(), uuids.clone()),
            // thought there was a ..world syntax, but there isn't
            // only when you set defaults
            path: world.path.clone(),
            type_: world.type_.clone()
        }
    }).collect::<Vec<World>>();
    worlds
}

fn get_uuids(name: String) -> Uuids {
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


    Uuids{online, offline}
}

// https://minecraft.wiki/w/Statistics
fn get_playtime(path: PathBuf, uuids: Uuids) -> PlayTime {
    let (_, folders) = read_folder(&path);
    // this shouldn't happen tho
    if !folders.iter().any(|x| x.file_name().unwrap() == "stats") { return PlayTime::None; }
    
    let stats_path = path.join("stats");
    let (files, _) = read_folder(&stats_path);
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
            let playtime_minute = json["stats"]["minecraft:custom"]["minecraft:play_one_minute"].as_u64().unwrap_or(0);

            let current_playtime = playtime_ticks + playtime_minute*60*20;
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
    let (files, folders) = read_folder(path);

    // go down a level, and search for the stats folder
    folders.clone().iter().for_each(|f| {
        worlds.extend(get_minecraft_worlds(f, depth.clone()+1));
    });

    // a stats folder is found, we return the path
    if folders.iter().any(|x| x.file_name().unwrap() == "stats") {
        let mut current = World {
            path: path.clone(),
            playtime: PlayTime::None,
            type_: GameType::Singleplayer
        };
        if !files.iter().any(|x| x.file_name().unwrap() == "icon.png") {
            current.type_ = GameType::Multiplayer;
        }
        worlds.push(current);
    }

    worlds
}

//? custom folder struct / enum?
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