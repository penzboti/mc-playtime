# idea
calculate your playtime in minecraft with the files you own
this means:
- your singleplayer worlds
- offline or online server files
specify the filepaths and a username.

GUI! with egui
graph?
table for sure -> grid in egui (or another one?)
export in text, or json
{
    (playtime ticks), playtime minutes, type (singleplayer|mp|mp(offlinemode)|mixed), world name, folder name, anything else?
}

# todo
create a struct that holds all the data.
we can store data in enums, like playtime on offline|online: https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html .
return that from handle_playtime, and display it in a separate windows state.

later: styling